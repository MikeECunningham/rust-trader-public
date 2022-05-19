mod create_order;
mod cancel_order;
mod handle_error;
mod account_info;
mod info;

use std::{sync::RwLock, time::{SystemTime, UNIX_EPOCH}};
use reqwest::{Client, Error};
use std::sync::PoisonError;
use std::time::SystemTimeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SetServerOffsetError {
    #[error("Failed to get system time")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("Poisoned rwlock when getting reader for server time offset")]
    RwLockPoisonedError(#[from] PoisonError<std::sync::RwLockWriteGuard<'static, i64>>)
}

#[derive(Error, Debug)]
pub enum CalculateServerTimeError {
    #[error("Failed to get system time")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("Poisoned rwlock when getting reader for server time offset")]
    RwLockPoisonedError
}

pub use self::create_order::*;

use super::types::BinanceAuth;

#[derive(Debug)]
pub struct Broker {
    server_timestamp_offset: RwLock<i64>,
    auth: BinanceAuth,
    client: Client,
}

impl Broker {
    pub fn new(url: String, key: String, secret: String) -> Result<Self, Error> {
        Ok(Broker {
            server_timestamp_offset: RwLock::new(-5000),
            auth: BinanceAuth { url, key, secret },
            client: reqwest::Client::builder().https_only(true).pool_max_idle_per_host(4).pool_idle_timeout(None).use_rustls_tls().build()?,
        })
    }

    /// Increments the offset from the server by the given value
    pub fn increment_server_offset(&'static self, increment: i64) -> Result<(), SetServerOffsetError> {
        *(self.server_timestamp_offset.write()?) = *(self.server_timestamp_offset.read().unwrap()) + increment;
        Ok(())
    }

    /// Sets the offset from the server to the given value
    pub fn set_server_offset(&'static self, offset: i64) -> Result<(), SetServerOffsetError> {
        *(self.server_timestamp_offset.write()?) = offset;
        Ok(())
    }

    /// Calculates the current time with the server offset accomodated
    pub fn calculate_server_time(&self) -> Result<u64, CalculateServerTimeError> {
        let server_time = match self.server_timestamp_offset.read() {
            Ok(v) => { Ok(*v)},
            Err(e) => { Err(CalculateServerTimeError::RwLockPoisonedError) },
        }?;
        let system_time: u64 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis().try_into().unwrap();
        return Ok(system_time.saturating_add_signed(server_time));
    }
}

lazy_static! {
    pub static ref BROKER: Broker = Broker::new(
        crate::config::CONFIG.binance_rest_url.clone(),
        crate::config::CONFIG.binance_key.clone(),
        crate::config::CONFIG.binance_secret.clone(),
    ).expect("Failed to create broker due to an issue building the request pool");
}