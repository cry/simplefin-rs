use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Invalid setup token (not valid base64 or URL): {0}")]
    InvalidToken(String),

    #[error("Token claim failed (403) — token may be compromised")]
    TokenClaimFailed,

    #[error("Unauthorized (403) — access URL credentials rejected")]
    Unauthorized,

    #[error("Payment required (402)")]
    PaymentRequired,

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}
