mod types;
mod balance;
mod cancel_order;
mod create_order;
mod get_order;
pub mod ping;

use reqwest::{Client, Error};
use std::sync::PoisonError;
use std::sync::RwLock;
use std::time::SystemTime;
use std::time::SystemTimeError;
use std::time::UNIX_EPOCH;
use thiserror::Error;

pub use self::types::*;
pub use self::balance::*;
pub use self::cancel_order::*;
pub use self::create_order::*;
pub use self::get_order::*;
pub use self::ping::*;

#[derive(Error, Debug)]
pub enum SetServerOffsetError {
    #[error("Failed to get system time")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("Poisoned rwlock when getting reader for server time offset")]
    RwLockPoisonedError(#[from] PoisonError<std::sync::RwLockWriteGuard<'static, i128>>)
}

#[derive(Error, Debug)]
pub enum CalculateServerTimeError {
    #[error("Failed to get system time")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("Poisoned rwlock when getting reader for server time offset")]
    RwLockPoisonedError
}

/// A broker represents an interactable endpoint for making trade requests
/// A broker is not responsible for realtime data.
/// A stream is instead used for that purpose.
#[derive(Debug)]
pub struct Broker {
    /// User authentication object used to place orders
    auth: BybitAuth,
    /// Pool of clients the broker can pull from when making requests
    client: Client,
    /// The timestamp offset from the current time
    server_timestamp_offset: RwLock<i128>,
}

impl Broker {

    pub fn new(url: String, key: String, secret: String) -> Result<Self, Error>{
        Ok(Broker {
            server_timestamp_offset: RwLock::new(0),
            auth: BybitAuth { url, key, secret },
            client: reqwest::Client::builder().https_only(true).pool_max_idle_per_host(4).pool_idle_timeout(None).use_rustls_tls().build()?,
        })
    }

    /// Sets the offset from the server to the given value
    pub fn set_server_offset(&'static self, offset: i128) -> Result<(), SetServerOffsetError> {
        *(self.server_timestamp_offset.write()?) = offset;
        Ok(())
    }

    /// Calculates the current time with the server offset accomodated
    pub fn calculate_server_time(&self) -> Result<u128, CalculateServerTimeError> {
        let server_time = match self.server_timestamp_offset.read() {
            Ok(v) => { Ok(*v)},
            Err(e) => { Err(CalculateServerTimeError::RwLockPoisonedError) },
        }?;
        return Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis().saturating_add_signed(server_time));
    }
}

lazy_static! {
    pub static ref BROKER: Broker = Broker::new(
        crate::config::CONFIG.bybit_rest_url.clone(),
        crate::config::CONFIG.bybit_key.clone(),
        crate::config::CONFIG.bybit_secret.clone(),
    ).expect("Failed to create broker due to an issue building the request pool");
}