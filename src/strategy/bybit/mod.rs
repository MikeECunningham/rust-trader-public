mod account;
mod order;
mod position;
mod message;
mod portfolio;
mod order_list;
pub mod strategy;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use dec::D128;
use std::num::ParseIntError;
use std::time::Instant;
use std::time::SystemTimeError;
use thiserror::Error;


use crate::backend::bybit::broker::SetServerOffsetError;

pub use self::account::*;
pub use self::order::*;
pub use self::position::*;
pub use self::message::*;
pub use self::portfolio::*;


pub const RISK: usize = 10;
pub const SCALE_RISK: usize = 0;
pub const SCALE: i32 = 2;
pub const RATE_CAP: i32 = 10;

pub const CHIRP: bool = false;
pub const CHIRP_ON_FLIP: bool = true;

lazy_static! {
    pub static ref REBATE: D128 = D128::from(0.00025);
    pub static ref MAX_OPEN_DIST: D128 = D128::from(30);
    pub static ref TOP_OPEN_DIST: D128 = D128::from(6);
}

#[derive(Error, Debug)]
pub enum StrategyRuntimeError {
    #[error("An endpoint returned a contact support response.")]
    ContactSupportError(String),
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum StratBranch {
    NNN,
    NNS,
    NSN,
    NSS,
    SNN,
    SNS,
    SSN,
    SSS
}

#[derive(Error, Debug)]
enum ApplyBookResultError {
    #[error("Failed to get system time")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("One of the rate limits was hit")]
    RateLimitError
}

#[derive(Error, Debug)]
enum CancelOrderResponseError {
    #[error("Request unauthorized")]
    UnauthorizedRequestError(#[from] UnauthorizedRequestError),
    #[error("Error writing to server offset")]
    SetServerOffsetError(#[from] SetServerOffsetError),
    #[error("An endpoint returned a contact support response.")]
    ContactSupportError(String),
}

#[derive(Error, Debug)]
enum OrderResponseError {
    #[error("Failed to get system time")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("Request unauthorized")]
    UnauthorizedRequestError(#[from] UnauthorizedRequestError),
    #[error("Error writing to server offset")]
    SetServerOffsetError(#[from] SetServerOffsetError),
    #[error("One of the rate limits was hit")]
    RateLimitError,
    #[error("An endpoint returned a contact support response.")]
    ContactSupportError(String),
}

#[derive(Error, Debug)]
enum UnauthorizedRequestError {
    #[error("Failed to get system time")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("Failed to parse unauthorized request response")]
    ParseResponseError(String),
    #[error("Failed to parse numbers in unauthorized request response")]
    ParseIntError(#[from] ParseIntError),
    #[error("Error writing to server offset")]
    SetServerOffsetError(#[from] SetServerOffsetError),
    #[error("One of the rate limits was hit")]
    RateLimitError,
}