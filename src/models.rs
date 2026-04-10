use serde::Deserialize;

/// Top-level response from `GET /accounts`.
#[derive(Debug, Clone, Deserialize)]
pub struct AccountSet {
    /// Structured errors reported by the server.
    #[serde(default, rename = "errlist")]
    pub errors: Vec<SfinError>,

    /// Financial institution connections associated with accounts.
    #[serde(default)]
    pub connections: Vec<Connection>,

    /// Financial accounts with optional transaction history.
    #[serde(default)]
    pub accounts: Vec<Account>,
}

/// A financial account at an institution.
#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    /// Unique, non-sensitive identifier for this account.
    pub id: String,

    /// Human-readable account name.
    pub name: String,

    /// Connection this account belongs to.
    pub conn_id: Option<String>,

    /// ISO 4217 currency code (e.g. `"USD"`) or a URL for custom currencies.
    pub currency: String,

    /// Current balance as a numeric string. Positive = credit.
    pub balance: String,

    /// Spendable balance, if different from `balance`.
    #[serde(rename = "available-balance")]
    pub available_balance: Option<String>,

    /// UNIX timestamp indicating when `balance` was accurate.
    #[serde(rename = "balance-date")]
    pub balance_date: i64,

    /// Transactions within the requested date range, if not using `balances-only`.
    pub transactions: Option<Vec<Transaction>>,

    /// Institution-specific additional fields.
    pub extra: Option<serde_json::Value>,
}

/// A single financial transaction on an account.
#[derive(Debug, Clone, Deserialize)]
pub struct Transaction {
    /// Unique identifier for this transaction within its account.
    pub id: String,

    /// UNIX timestamp when the transaction posted. `0` if still pending.
    pub posted: i64,

    /// Transaction amount as a numeric string. Positive = deposit/credit.
    pub amount: String,

    /// Human-readable description of the transaction.
    pub description: String,

    /// When the transaction originally occurred, if known.
    pub transacted_at: Option<i64>,

    /// `true` if the transaction has not yet settled.
    pub pending: Option<bool>,

    /// Institution-specific additional fields.
    pub extra: Option<serde_json::Value>,
}

/// A financial institution connection (SimpleFIN v2).
#[derive(Debug, Clone, Deserialize)]
pub struct Connection {
    /// Unique identifier for this connection.
    pub conn_id: String,

    /// Human-friendly name including institution name.
    pub name: String,

    /// Institution identifier — unique per SimpleFIN server.
    pub org_id: Option<serde_json::Value>,

    /// Institution domain.
    pub org_url: Option<String>,

    /// Root URL of the institution's SimpleFIN server.
    pub sfin_url: Option<String>,
}

/// A structured error reported by the SimpleFIN server.
///
/// `code` uses a `prefix.subcode` format (e.g. `gen.auth`, `con.auth`).
/// Unknown subcodes should be treated as the bare prefix for forward compatibility.
///
/// # Security
/// Display `message` only after sanitizing — it originates from the financial institution.
#[derive(Debug, Clone, Deserialize)]
pub struct SfinError {
    /// Structured error code in `prefix[.subcode]` format.
    pub code: String,

    /// Human-readable error message. **Must be sanitized before display.**
    #[serde(rename = "msg")]
    pub message: String,

    /// Connection this error relates to, if any.
    pub conn_id: Option<String>,

    /// Account this error relates to, if any.
    pub account_id: Option<String>,
}
