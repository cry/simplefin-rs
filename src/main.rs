use clap::Parser;
use simplefin::{AccountsRequest, SimpleFINClient};
use std::env;

/// SimpleFIN account and transaction viewer.
///
/// Set SIMPLEFIN_TOKEN (first-time) or SIMPLEFIN_ACCESS_URL (subsequent runs)
/// before invoking.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Start of transaction date range, inclusive.
    /// Accepts YYYY-MM-DD or a UNIX timestamp.
    #[arg(long, value_name = "DATE")]
    start: Option<String>,

    /// End of transaction date range, exclusive.
    /// Accepts YYYY-MM-DD or a UNIX timestamp.
    #[arg(long, value_name = "DATE")]
    end: Option<String>,

    /// Include pending (unposted) transactions.
    #[arg(long)]
    pending: bool,

    /// Return balances only — skip transaction data.
    #[arg(long)]
    balances_only: bool,

    /// Restrict to specific account IDs (repeatable).
    #[arg(long = "account", value_name = "ID")]
    accounts: Vec<String>,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> simplefin::Result<()> {
    let args = Args::parse();

    let client = if let Ok(token) = env::var("SIMPLEFIN_TOKEN") {
        println!("Claiming setup token…");
        let client = SimpleFINClient::claim(&token).await?;
        println!(
            "Success! Save this Access URL securely:\n  {}\n",
            client.access_url_str()
        );
        client
    } else if let Ok(access_url) = env::var("SIMPLEFIN_ACCESS_URL") {
        SimpleFINClient::from_access_url(&access_url)?
    } else {
        eprintln!(
            "Usage:\n  \
             SIMPLEFIN_TOKEN=<base64-setup-token> simplefin    # first-time claim\n  \
             SIMPLEFIN_ACCESS_URL=<access-url>   simplefin    # subsequent runs"
        );
        std::process::exit(1);
    };

    let start_date = args
        .start
        .as_deref()
        .map(parse_date)
        .transpose()
        .map_err(|e| {
            eprintln!("Invalid --start value: {e}");
            std::process::exit(1);
        })
        .unwrap();

    let end_date = args
        .end
        .as_deref()
        .map(parse_date)
        .transpose()
        .map_err(|e| {
            eprintln!("Invalid --end value: {e}");
            std::process::exit(1);
        })
        .unwrap();

    println!("Fetching accounts…\n");
    let account_set = client
        .get_accounts(AccountsRequest {
            start_date,
            end_date,
            pending: args.pending,
            balances_only: args.balances_only,
            accounts: args.accounts,
        })
        .await?;

    // Report any server-side errors (sanitized: we only print the code, not raw msg).
    for err in &account_set.errors {
        eprintln!(
            "[server error] code={} conn={:?} account={:?}",
            err.code, err.conn_id, err.account_id
        );
    }

    if account_set.accounts.is_empty() {
        println!("No accounts returned.");
        return Ok(());
    }

    for account in &account_set.accounts {
        println!(
            "Account: {} ({})\n  Balance: {} {}",
            account.name, account.id, account.balance, account.currency,
        );
        if let Some(avail) = &account.available_balance {
            println!("  Available: {avail}");
        }
        println!();

        if let Some(transactions) = &account.transactions {
            if transactions.is_empty() {
                println!("  No transactions in range.\n");
            } else {
                println!("  Transactions:");
                for txn in transactions {
                    let date = if txn.posted == 0 {
                        "pending".to_string()
                    } else {
                        format_date(txn.posted)
                    };
                    println!("    [{date}] {:>12}  {}", txn.amount, txn.description);
                }
                println!();
            }
        }
    }

    Ok(())
}

/// Parse a date string as either `YYYY-MM-DD` (UTC midnight) or a raw UNIX timestamp.
fn parse_date(s: &str) -> Result<i64, String> {
    // Try YYYY-MM-DD first.
    if let Some((date_part, "")) = s.split_once(' ').map(|_| ("", "")).or_else(|| {
        // exactly 10 chars: YYYY-MM-DD
        if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
            Some((s, ""))
        } else {
            None
        }
    }) {
        let parts: Vec<&str> = date_part.splitn(3, '-').collect();
        if parts.len() == 3 {
            let year: i32 = parts[0]
                .parse()
                .map_err(|_| format!("invalid year in '{s}'"))?;
            let month: u32 = parts[1]
                .parse()
                .map_err(|_| format!("invalid month in '{s}'"))?;
            let day: u32 = parts[2]
                .parse()
                .map_err(|_| format!("invalid day in '{s}'"))?;
            if month < 1 || month > 12 || day < 1 || day > 31 {
                return Err(format!("date out of range: '{s}'"));
            }
            return Ok(ymd_to_unix(year, month, day));
        }
    }

    // Fall back to raw UNIX timestamp.
    s.parse::<i64>()
        .map_err(|_| format!("expected YYYY-MM-DD or a UNIX timestamp, got '{s}'"))
}

/// Convert a Gregorian date to a UNIX timestamp at UTC midnight.
/// Uses Howard Hinnant's civil-from-days algorithm (public domain).
fn ymd_to_unix(year: i32, month: u32, day: u32) -> i64 {
    let y = year as i64 - (month <= 2) as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m = month as i64;
    let doy = (153 * (m + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) * 86400
}

/// Format a UNIX timestamp as `YYYY-MM-DD` for display.
fn format_date(ts: i64) -> String {
    // Reverse of ymd_to_unix — civil_from_days (Hinnant).
    let z = ts / 86400 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}
