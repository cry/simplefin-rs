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

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{Expect, expect};

    fn check(json: &str, expect: Expect) {
        let set: AccountSet = serde_json::from_str(json).unwrap();
        expect.assert_debug_eq(&set);
    }

    fn minimal_account(extra: &str) -> String {
        format!(
            r#"{{"accounts": [{{"id": "a", "name": "Checking", "currency": "USD", "balance": "1234.56", "balance-date": 1700000000 {extra}}}]}}"#
        )
    }

    // --- AccountSet defaults ---

    #[test]
    fn empty_account_set() {
        check(
            r#"{"accounts": [], "connections": []}"#,
            expect![[r#"
                AccountSet {
                    errors: [],
                    connections: [],
                    accounts: [],
                }
            "#]],
        );
    }

    #[test]
    fn missing_optional_fields_default_to_empty() {
        // errlist and connections are absent — both must default to [].
        check(
            r#"{"accounts": []}"#,
            expect![[r#"
                AccountSet {
                    errors: [],
                    connections: [],
                    accounts: [],
                }
            "#]],
        );
    }

    // --- SfinError: "errlist" and "msg" renames, optional ids ---

    #[test]
    fn errlist_rename_and_bare_error() {
        check(
            r#"{"errlist": [{"code": "gen.auth", "msg": "Not authorised"}], "accounts": []}"#,
            expect![[r#"
                AccountSet {
                    errors: [
                        SfinError {
                            code: "gen.auth",
                            message: "Not authorised",
                            conn_id: None,
                            account_id: None,
                        },
                    ],
                    connections: [],
                    accounts: [],
                }
            "#]],
        );
    }

    #[test]
    fn sfin_error_with_optional_ids() {
        check(
            r#"{"errlist": [{"code": "con.auth", "msg": "Bad creds", "conn_id": "c1", "account_id": "a1"}], "accounts": []}"#,
            expect![[r#"
                AccountSet {
                    errors: [
                        SfinError {
                            code: "con.auth",
                            message: "Bad creds",
                            conn_id: Some(
                                "c1",
                            ),
                            account_id: Some(
                                "a1",
                            ),
                        },
                    ],
                    connections: [],
                    accounts: [],
                }
            "#]],
        );
    }

    // --- Account: field renames and optional fields ---

    #[test]
    fn account_minimal_no_transactions() {
        check(
            &minimal_account(""),
            expect![[r#"
                AccountSet {
                    errors: [],
                    connections: [],
                    accounts: [
                        Account {
                            id: "a",
                            name: "Checking",
                            conn_id: None,
                            currency: "USD",
                            balance: "1234.56",
                            available_balance: None,
                            balance_date: 1700000000,
                            transactions: None,
                            extra: None,
                        },
                    ],
                }
            "#]],
        );
    }

    #[test]
    fn account_with_available_balance() {
        check(
            &minimal_account(r#", "available-balance": "800.00""#),
            expect![[r#"
                AccountSet {
                    errors: [],
                    connections: [],
                    accounts: [
                        Account {
                            id: "a",
                            name: "Checking",
                            conn_id: None,
                            currency: "USD",
                            balance: "1234.56",
                            available_balance: Some(
                                "800.00",
                            ),
                            balance_date: 1700000000,
                            transactions: None,
                            extra: None,
                        },
                    ],
                }
            "#]],
        );
    }

    #[test]
    fn account_with_empty_transactions() {
        check(
            &minimal_account(r#", "transactions": []"#),
            expect![[r#"
                AccountSet {
                    errors: [],
                    connections: [],
                    accounts: [
                        Account {
                            id: "a",
                            name: "Checking",
                            conn_id: None,
                            currency: "USD",
                            balance: "1234.56",
                            available_balance: None,
                            balance_date: 1700000000,
                            transactions: Some(
                                [],
                            ),
                            extra: None,
                        },
                    ],
                }
            "#]],
        );
    }

    // --- Transaction variants ---

    #[test]
    fn posted_transaction() {
        check(
            &minimal_account(
                r#", "transactions": [{"id": "t1", "posted": 1700000000, "amount": "-42.00", "description": "Coffee shop"}]"#,
            ),
            expect![[r#"
                AccountSet {
                    errors: [],
                    connections: [],
                    accounts: [
                        Account {
                            id: "a",
                            name: "Checking",
                            conn_id: None,
                            currency: "USD",
                            balance: "1234.56",
                            available_balance: None,
                            balance_date: 1700000000,
                            transactions: Some(
                                [
                                    Transaction {
                                        id: "t1",
                                        posted: 1700000000,
                                        amount: "-42.00",
                                        description: "Coffee shop",
                                        transacted_at: None,
                                        pending: None,
                                        extra: None,
                                    },
                                ],
                            ),
                            extra: None,
                        },
                    ],
                }
            "#]],
        );
    }

    #[test]
    fn pending_transaction_posted_zero() {
        check(
            &minimal_account(
                r#", "transactions": [{"id": "t2", "posted": 0, "amount": "-5.00", "description": "Pending charge", "pending": true}]"#,
            ),
            expect![[r#"
                AccountSet {
                    errors: [],
                    connections: [],
                    accounts: [
                        Account {
                            id: "a",
                            name: "Checking",
                            conn_id: None,
                            currency: "USD",
                            balance: "1234.56",
                            available_balance: None,
                            balance_date: 1700000000,
                            transactions: Some(
                                [
                                    Transaction {
                                        id: "t2",
                                        posted: 0,
                                        amount: "-5.00",
                                        description: "Pending charge",
                                        transacted_at: None,
                                        pending: Some(
                                            true,
                                        ),
                                        extra: None,
                                    },
                                ],
                            ),
                            extra: None,
                        },
                    ],
                }
            "#]],
        );
    }

    #[test]
    fn transaction_with_transacted_at() {
        check(
            &minimal_account(
                r#", "transactions": [{"id": "t3", "posted": 1700000100, "amount": "10.00", "description": "ATM", "transacted_at": 1700000000}]"#,
            ),
            expect![[r#"
                AccountSet {
                    errors: [],
                    connections: [],
                    accounts: [
                        Account {
                            id: "a",
                            name: "Checking",
                            conn_id: None,
                            currency: "USD",
                            balance: "1234.56",
                            available_balance: None,
                            balance_date: 1700000000,
                            transactions: Some(
                                [
                                    Transaction {
                                        id: "t3",
                                        posted: 1700000100,
                                        amount: "10.00",
                                        description: "ATM",
                                        transacted_at: Some(
                                            1700000000,
                                        ),
                                        pending: None,
                                        extra: None,
                                    },
                                ],
                            ),
                            extra: None,
                        },
                    ],
                }
            "#]],
        );
    }

    // --- Connection ---

    #[test]
    fn connection_fields() {
        check(
            r#"{
                "accounts": [],
                "connections": [{"conn_id": "c1", "name": "My Bank", "org_url": "https://mybank.example"}]
            }"#,
            expect![[r#"
                AccountSet {
                    errors: [],
                    connections: [
                        Connection {
                            conn_id: "c1",
                            name: "My Bank",
                            org_id: None,
                            org_url: Some(
                                "https://mybank.example",
                            ),
                            sfin_url: None,
                        },
                    ],
                    accounts: [],
                }
            "#]],
        );
    }

    // --- Bad JSON is rejected ---

    #[test]
    fn missing_required_field_errors() {
        // `balance` is required on Account — deserializing without it must fail.
        let json = r#"{"accounts": [{"id": "a", "name": "n", "currency": "USD", "balance-date": 0}]}"#;
        assert!(serde_json::from_str::<AccountSet>(json).is_err());
    }
}
