use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest::{Client, StatusCode};
use url::Url;

use crate::{
    error::Error,
    models::AccountSet,
    Result,
};

/// Parameters for `GET /accounts`.
#[derive(Debug, Clone, Default)]
pub struct AccountsRequest {
    /// Include transactions on or after this UNIX timestamp (inclusive).
    pub start_date: Option<i64>,

    /// Include transactions before this UNIX timestamp (exclusive).
    pub end_date: Option<i64>,

    /// Include pending (unposted) transactions.
    pub pending: bool,

    /// Restrict to specific account IDs. Empty = all accounts.
    pub accounts: Vec<String>,

    /// Skip transaction data and return balances only.
    pub balances_only: bool,
}

/// Client for the SimpleFIN protocol.
///
/// # Setup flow
/// 1. Direct the user to your SimpleFIN server's `/create` endpoint to get a setup token.
/// 2. Call [`SimpleFINClient::claim`] to exchange the token for an Access URL.
/// 3. **Store the Access URL securely** (treat it like credentials).
/// 4. On subsequent runs, restore the client with [`SimpleFINClient::from_access_url`].
pub struct SimpleFINClient {
    access_url: Url,
    inner: Client,
}

impl SimpleFINClient {
    fn build_http_client() -> Result<Client> {
        Ok(Client::builder()
            .https_only(true)
            .build()?)
    }

    /// Exchange a setup token for an Access URL and return a ready client.
    ///
    /// The setup token is a Base64-encoded claim URL provided by the SimpleFIN server.
    /// After claiming, store the returned [`SimpleFINClient::access_url_str`] securely
    /// and use [`SimpleFINClient::from_access_url`] on future runs.
    pub async fn claim(setup_token: &str) -> Result<Self> {
        let decoded = BASE64
            .decode(setup_token.trim())
            .map_err(|e| Error::InvalidToken(e.to_string()))?;

        let claim_url_str =
            String::from_utf8(decoded).map_err(|e| Error::InvalidToken(e.to_string()))?;

        let claim_url =
            Url::parse(&claim_url_str).map_err(|e| Error::InvalidToken(e.to_string()))?;

        let http = Self::build_http_client()?;
        let response = http.post(claim_url).send().await?;

        match response.status() {
            StatusCode::FORBIDDEN => return Err(Error::TokenClaimFailed),
            s if !s.is_success() => {
                return Err(Error::Http(
                    response.error_for_status().unwrap_err(),
                ));
            }
            _ => {}
        }

        let access_url_str = response.text().await?;
        let access_url = Url::parse(access_url_str.trim())?;

        Ok(Self { access_url, inner: http })
    }

    /// Restore a client from a previously claimed Access URL.
    ///
    /// Use this on subsequent runs after storing the URL from [`SimpleFINClient::claim`].
    pub fn from_access_url(access_url: &str) -> Result<Self> {
        let access_url = Url::parse(access_url.trim())?;
        let inner = Self::build_http_client()?;
        Ok(Self { access_url, inner })
    }

    /// The Access URL for this client. Store this securely after claiming.
    pub fn access_url_str(&self) -> &str {
        self.access_url.as_str()
    }

    /// Fetch accounts (and optionally transactions) from the SimpleFIN server.
    pub async fn get_accounts(&self, params: AccountsRequest) -> Result<AccountSet> {
        let username = self.access_url.username().to_owned();
        let password = self.access_url.password().unwrap_or("").to_owned();

        // Build the /accounts URL from the access URL base, stripping credentials.
        let mut accounts_url = self.access_url.clone();
        accounts_url.set_username("").ok();
        accounts_url.set_password(None).ok();
        accounts_url.set_path(&format!(
            "{}/accounts",
            self.access_url.path().trim_end_matches('/')
        ));

        let mut query: Vec<(&str, String)> = Vec::new();

        if let Some(start) = params.start_date {
            query.push(("start-date", start.to_string()));
        }
        if let Some(end) = params.end_date {
            query.push(("end-date", end.to_string()));
        }
        if params.pending {
            query.push(("pending", "1".into()));
        }
        if params.balances_only {
            query.push(("balances-only", "1".into()));
        }
        for account_id in &params.accounts {
            query.push(("account", account_id.clone()));
        }

        let response = self
            .inner
            .get(accounts_url)
            .basic_auth(&username, Some(&password))
            .query(&query)
            .send()
            .await?;

        match response.status() {
            StatusCode::FORBIDDEN => return Err(Error::Unauthorized),
            StatusCode::PAYMENT_REQUIRED => return Err(Error::PaymentRequired),
            s if !s.is_success() => {
                return Err(Error::Http(response.error_for_status().unwrap_err()));
            }
            _ => {}
        }

        let account_set: AccountSet = response.json().await?;
        Ok(account_set)
    }
}
