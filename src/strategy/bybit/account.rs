/// Bybit Account --> account interface --> position interface --> position --> orders
use std::{collections::HashMap};

use crate::backend::bybit::broker::{GetBalanceError};

use super::{Portfolio};

use thiserror::Error;


#[derive(Error, Debug)]
pub enum PopulateWalletsError {
    #[error("Failed to send request")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Failed to serialize response")]
    SerdeError(#[from] serde_json::Error),
    #[error("Failed to get Balances for account")]
    GetBalanceError(#[from] GetBalanceError)
}


/// An account is the top level structure for the private data of a relationship with an exchange.
/// The account primarily interfaces with an exchange by:
///     a) Managing a wallet of assets;
///     b) Managing positions in markets.
/// (See AssetPositionsPortfolio for further details)
/// The assets represented in this account are symbols of asset pairs
/// such as BTCUSDT.
/// The corresponding positions portfolio will handle rebalancing assets
/// and informing the strategy of relevant information regarding the success
/// or failure of events
// #[derive(Debug)]
pub struct Account {
    /// A mapping of asset pair names to the current state of the asset
    pub asset_pairs: HashMap<String, Portfolio>,
}


impl Account {

    pub fn new() -> Account {
        Account {
            asset_pairs: HashMap::new(),
        }
    }
}
