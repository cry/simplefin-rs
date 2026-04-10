pub mod client;
pub mod error;
pub mod models;

pub use client::{AccountsRequest, SimpleFINClient};
pub use error::Error;
pub use models::{Account, AccountSet, Connection, SfinError, Transaction};

pub type Result<T> = std::result::Result<T, Error>;
