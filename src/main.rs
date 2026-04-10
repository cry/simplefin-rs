use clap::Parser;
use jiff::{Timestamp, civil::Date, tz::TimeZone};
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
    // Detect YYYY-MM-DD by shape (10 chars, dashes at positions 4 and 7).
    if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        let date: Date = s
            .parse()
            .map_err(|e| format!("invalid date '{s}': {e}"))?;
        return date
            .at(0, 0, 0, 0)
            .in_tz("UTC")
            .map(|z: jiff::Zoned| z.timestamp().as_second())
            .map_err(|e| format!("timezone error: {e}"));
    }

    // Fall back to raw UNIX timestamp.
    s.parse::<i64>()
        .map_err(|_| format!("expected YYYY-MM-DD or a UNIX timestamp, got '{s}'"))
}

/// Format a UNIX timestamp as `YYYY-MM-DD` (UTC) for display.
fn format_date(ts: i64) -> String {
    match Timestamp::from_second(ts) {
        Ok(t) => t.to_zoned(TimeZone::UTC).date().to_string(),
        Err(_) => ts.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yyyy_mm_dd_epoch() {
        assert_eq!(parse_date("1970-01-01").unwrap(), 0);
    }

    #[test]
    fn parse_yyyy_mm_dd_known() {
        assert_eq!(parse_date("2024-01-01").unwrap(), 1_704_067_200);
    }

    #[test]
    fn parse_yyyy_mm_dd_leap_day() {
        // jiff validates the calendar, so Feb 29 on a leap year must succeed.
        assert!(parse_date("2000-02-29").is_ok());
    }

    #[test]
    fn parse_raw_unix_timestamp() {
        assert_eq!(parse_date("1704067200").unwrap(), 1_704_067_200);
    }

    #[test]
    fn parse_zero_timestamp() {
        assert_eq!(parse_date("0").unwrap(), 0);
    }

    #[test]
    fn parse_negative_timestamp() {
        assert_eq!(parse_date("-86400").unwrap(), -86400);
    }

    #[test]
    fn parse_invalid_string_errors() {
        assert!(parse_date("not-a-date").is_err());
    }

    #[test]
    fn parse_month_out_of_range_errors() {
        assert!(parse_date("2024-13-01").is_err());
    }

    #[test]
    fn parse_day_out_of_range_errors() {
        assert!(parse_date("2024-01-32").is_err());
    }

    #[test]
    fn parse_feb_29_non_leap_year_errors() {
        assert!(parse_date("2023-02-29").is_err());
    }

    #[test]
    fn parse_short_date_falls_back_to_timestamp_error() {
        // "2024-1-1" doesn't match the 10-char YYYY-MM-DD shape so it tries
        // to parse as i64, which also fails.
        assert!(parse_date("2024-1-1").is_err());
    }
}
