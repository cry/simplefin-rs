# simplefin-rs

A Rust client library and CLI for the [SimpleFIN protocol](https://www.simplefin.org/protocol.html) — read-only access to financial account and transaction data.

## What is SimpleFIN?

SimpleFIN is an open protocol that lets users share their financial data (balances, transactions) with applications without giving those applications their banking credentials. The application only ever receives a read-only Access URL, scoped to the data the user authorises.

---

## Library

### Add to your project

```toml
[dependencies]
simplefin-rs = { path = "../simplefin-rs" }  # or crates.io once published
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### Authentication

SimpleFIN uses a two-step setup. The setup token is **single-use**; the resulting Access URL is your long-lived credential.

**Step 1 — first-time claim**

Direct the user to the SimpleFIN server's `/create` endpoint to get a Base64 setup token, then exchange it:

```rust
use simplefin::SimpleFINClient;

let client = SimpleFINClient::claim(&setup_token).await?;

// Store this securely — it is equivalent to a password.
let access_url = client.access_url_str();
println!("Save this: {access_url}");
```

> If `claim` returns `Error::TokenClaimFailed` (HTTP 403), warn the user that the token may have already been used and could be compromised.

**Step 2 — subsequent runs**

```rust
let client = SimpleFINClient::from_access_url(&stored_access_url)?;
```

### Fetching accounts

```rust
use simplefin::{AccountsRequest, SimpleFINClient};

// All accounts with all available transactions
let account_set = client.get_accounts(AccountsRequest::default()).await?;

// Filtered request
let account_set = client.get_accounts(AccountsRequest {
    start_date:    Some(1_700_000_000),  // UNIX timestamp, inclusive
    end_date:      Some(1_710_000_000),  // UNIX timestamp, exclusive
    pending:       true,                 // include unposted transactions
    accounts:      vec!["account-id".into()],  // empty = all accounts
    balances_only: false,                // true = skip transactions
}).await?;
```

### Working with the response

```rust
// Server-side partial errors (accounts may still be populated)
for err in &account_set.errors {
    // err.code is "prefix" or "prefix.subcode" — e.g. "gen.auth", "con.auth"
    // Treat unknown subcodes as the bare prefix (forward-compatible).
    //
    // SECURITY: sanitize err.message before displaying — it originates from
    // the financial institution and is not under your control.
    eprintln!("server error [{}]", err.code);
}

for account in &account_set.accounts {
    println!("{}: {} {}", account.name, account.balance, account.currency);

    if let Some(txns) = &account.transactions {
        for t in txns {
            // t.posted == 0  →  transaction is still pending
            // t.amount is a numeric string; positive = deposit/credit
            println!("  {} {}", t.amount, t.description);
        }
    }
}
```

### Data types

| Type | Description |
|---|---|
| `AccountSet` | Top-level response: `errors`, `connections`, `accounts` |
| `Account` | `id`, `name`, `currency`, `balance`, `available_balance`, `balance_date`, `transactions` |
| `Transaction` | `id`, `posted` (0 if pending), `amount`, `description`, `transacted_at`, `pending` |
| `Connection` | Institution connection metadata (`conn_id`, `name`, `org_url`, `sfin_url`) |
| `SfinError` | Server-reported error: `code`, `message` (sanitize before display), `conn_id`, `account_id` |

### Error handling

All public methods return `simplefin::Result<T>`.

| Variant | Cause |
|---|---|
| `Error::TokenClaimFailed` | HTTP 403 on `/claim` — token may be compromised |
| `Error::Unauthorized` | HTTP 403 on `/accounts` — access URL credentials rejected |
| `Error::PaymentRequired` | HTTP 402 on `/accounts` |
| `Error::InvalidToken(msg)` | Setup token is not valid Base64 or a URL |
| `Error::Http(e)` | Any other HTTP-level failure |
| `Error::UrlParse(e)` | Malformed URL |
| `Error::Json(e)` | Response body failed to deserialize |

---

## CLI

### Setup

```sh
cargo build --release
# binary at: target/release/simplefin
```

### First-time: claim a setup token

Get a token by visiting your SimpleFIN server's `/create` endpoint (e.g. the [SimpleFIN Bridge sandbox](https://beta-bridge.simplefin.org/simplefin/create)).

```sh
SIMPLEFIN_TOKEN=<base64-token> simplefin
```

The Access URL is printed on success — save it somewhere secure. You will use it for all future runs.

### Subsequent runs

```sh
SIMPLEFIN_ACCESS_URL=https://user:pass@bridge.simplefin.org/simplefin simplefin
```

### Options

```
Usage: simplefin [OPTIONS]

Options:
      --start <DATE>     Start of transaction range, inclusive (YYYY-MM-DD or UNIX timestamp)
      --end <DATE>       End of transaction range, exclusive  (YYYY-MM-DD or UNIX timestamp)
      --pending          Include pending (unposted) transactions
      --balances-only    Return balances only, skip transactions
      --account <ID>     Restrict to a specific account ID (repeatable)
  -h, --help             Print help
  -V, --version          Print version
```

### Examples

```sh
# All accounts, last quarter
SIMPLEFIN_ACCESS_URL=... simplefin --start 2025-01-01 --end 2025-04-01

# One account, balances only
SIMPLEFIN_ACCESS_URL=... simplefin --account acct-abc123 --balances-only

# Include pending transactions
SIMPLEFIN_ACCESS_URL=... simplefin --start 2025-03-01 --pending
```

---

## Security

- **HTTPS only.** The HTTP client rejects plain HTTP URLs at the client level (`https_only(true)`).
- **TLS certificate verification** is always enabled (rustls defaults — no opt-out).
- **Store the Access URL as securely as financial data.** Anyone with it can read the user's accounts.
- **Sanitize server error messages.** `SfinError::message` and custom currency strings originate from the financial institution and must be sanitized before display.
- **403 on `/claim`** means the token may have already been used. Alert the user — it could be compromised.

---

## Building and testing

```sh
cargo build
cargo test
cargo run -- --help
```
