#![feature(mixed_integer_ops)]

use hmac::{Hmac, digest::InvalidLength};
use sha2::Sha256;
use thiserror::Error;

#[macro_use]
extern crate lazy_static;
extern crate proc_macros;
#[macro_use]
extern crate logging;

pub mod analysis;
pub mod orderbook;
pub mod backend;
pub mod tradeflow;
pub mod strategy;
pub mod stats;
pub mod signal_handler;
pub mod config;

// Generally useful type aliases
pub type HmacSha256 = Hmac<Sha256>;


/// Errors thrown when signing requests for bybit
#[derive(Error, Debug)]
pub enum SignRequestError {
    #[error("Invalid mac key length")]
    MacLengthError(#[from] InvalidLength),
    #[error("Failed to serialize response")]
    JSONSerdeError(#[from] serde_json::Error),
    #[error("Failed to serialize response")]
    URLSerdeError(#[from] serde_urlencoded::ser::Error)
}
