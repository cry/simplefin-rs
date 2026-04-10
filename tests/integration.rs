/// Integration tests against the SimpleFIN Bridge demo sandbox.
///
/// Each test returns early when the `CI` environment variable is absent so
/// that `cargo test` stays fast and offline-safe locally.  On GitHub Actions
/// `CI=true` is set automatically, so the full suite runs there.
///
/// Run locally with:
///   CI=1 cargo test --test integration
use simplefin::{AccountsRequest, SimpleFINClient};

// Static demo access URL — safe to use from many tests in parallel.
const DEMO_ACCESS_URL: &str = "https://demo:demo@beta-bridge.simplefin.org/simplefin";

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before UNIX epoch")
        .as_secs() as i64
}

/// Skip this test when not running in CI.
macro_rules! ci_only {
    () => {
        if std::env::var("CI").is_err() {
            return;
        }
    };
}

fn demo_client() -> SimpleFINClient {
    SimpleFINClient::from_access_url(DEMO_ACCESS_URL).expect("failed to build demo client")
}

// --- get_accounts: basic structure ---

#[tokio::test]
async fn get_accounts_default_has_required_fields() {
    ci_only!();
    let account_set = demo_client()
        .get_accounts(AccountsRequest::default())
        .await
        .expect("get_accounts failed");

    assert!(
        !account_set.accounts.is_empty(),
        "expected demo data to contain at least one account"
    );

    for account in &account_set.accounts {
        assert!(!account.id.is_empty(), "account id must not be empty");
        assert!(!account.name.is_empty(), "account name must not be empty");
        assert!(!account.currency.is_empty(), "account currency must not be empty");
        account
            .balance
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("balance '{}' must parse as a number", account.balance));
    }
}

#[tokio::test]
async fn get_accounts_transactions_present_by_default() {
    ci_only!();
    let account_set = demo_client()
        .get_accounts(AccountsRequest::default())
        .await
        .expect("get_accounts failed");

    let has_transactions = account_set
        .accounts
        .iter()
        .any(|a| a.transactions.as_ref().is_some_and(|t| !t.is_empty()));
    assert!(
        has_transactions,
        "expected at least one account to have transactions in the default response"
    );

    for account in &account_set.accounts {
        if let Some(transactions) = &account.transactions {
            for txn in transactions {
                assert!(!txn.id.is_empty(), "transaction id must not be empty");
                assert!(
                    !txn.description.is_empty(),
                    "transaction description must not be empty"
                );
                txn.amount.parse::<f64>().unwrap_or_else(|_| {
                    panic!("transaction amount '{}' must parse as a number", txn.amount)
                });
            }
        }
    }
}

// --- get_accounts: query parameters ---

#[tokio::test]
async fn get_accounts_balances_only_omits_transactions() {
    ci_only!();
    let account_set = demo_client()
        .get_accounts(AccountsRequest {
            balances_only: true,
            ..Default::default()
        })
        .await
        .expect("get_accounts failed");

    assert!(
        !account_set.accounts.is_empty(),
        "expected accounts even with balances_only"
    );
    for account in &account_set.accounts {
        let empty = account
            .transactions
            .as_ref()
            .map_or(true, |t| t.is_empty());
        assert!(
            empty,
            "account '{}' should have no transactions when balances_only=true",
            account.id
        );
    }
}

#[tokio::test]
async fn get_accounts_date_range_bounds_transactions() {
    ci_only!();
    let now = now_secs();
    let start = now - 365 * 86400; // one year ago
    let end = now + 365 * 86400;   // one year from now

    let account_set = demo_client()
        .get_accounts(AccountsRequest {
            start_date: Some(start),
            end_date: Some(end),
            ..Default::default()
        })
        .await
        .expect("get_accounts failed");

    for account in &account_set.accounts {
        if let Some(transactions) = &account.transactions {
            for txn in transactions {
                if txn.posted > 0 {
                    assert!(
                        txn.posted >= start,
                        "transaction '{}' posted={} is before start_date={}",
                        txn.id, txn.posted, start
                    );
                    assert!(
                        txn.posted < end,
                        "transaction '{}' posted={} is on or after end_date={}",
                        txn.id, txn.posted, end
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn get_accounts_without_pending_flag_excludes_pending_transactions() {
    ci_only!();
    // pending: false is the default — server should not return posted==0 transactions.
    let account_set = demo_client()
        .get_accounts(AccountsRequest::default())
        .await
        .expect("get_accounts failed");

    for account in &account_set.accounts {
        if let Some(transactions) = &account.transactions {
            for txn in transactions {
                assert!(
                    txn.posted != 0,
                    "account '{}' has pending transaction '{}' (posted=0) without pending flag",
                    account.id,
                    txn.id
                );
            }
        }
    }
}

#[tokio::test]
async fn get_accounts_account_filter_returns_only_requested_account() {
    ci_only!();
    let all = demo_client()
        .get_accounts(AccountsRequest::default())
        .await
        .expect("get_accounts failed");

    if all.accounts.is_empty() {
        return;
    }
    let target_id = all.accounts[0].id.clone();

    let filtered = demo_client()
        .get_accounts(AccountsRequest {
            accounts: vec![target_id.clone()],
            ..Default::default()
        })
        .await
        .expect("get_accounts with account filter failed");

    assert_eq!(
        filtered.accounts.len(),
        1,
        "expected exactly one account when filtering by id"
    );
    assert_eq!(
        filtered.accounts[0].id, target_id,
        "returned account id does not match the requested filter"
    );
}

// --- get_accounts: structural invariants ---

#[tokio::test]
async fn get_accounts_no_errors_for_valid_demo_credentials() {
    ci_only!();
    let account_set = demo_client()
        .get_accounts(AccountsRequest::default())
        .await
        .expect("get_accounts failed");

    assert!(
        account_set.errors.is_empty(),
        "expected no server errors for demo credentials, got: {:?}",
        account_set.errors.iter().map(|e| &e.code).collect::<Vec<_>>()
    );
}
