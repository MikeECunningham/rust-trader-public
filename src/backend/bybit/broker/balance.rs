use std::{collections::HashMap, time::SystemTimeError};
use hmac::{Mac, digest::InvalidLength};
use thiserror::Error;

use crate::{config::CONFIG, HmacSha256};

use super::{Broker, Balance, RestResponse, CalculateServerTimeError};


#[derive(Error, Debug)]
pub enum GetBalanceError {
    #[error("Invalid mac key length")]
    MacLengthError(#[from] InvalidLength),
    #[error("Failed to send request")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Failed to serialize response")]
    SerdeError(#[from] serde_json::Error),
    #[error("Failed to get system time")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("Failed to calculate server time")]
    CalculateServerTimeError(#[from] CalculateServerTimeError)
}

impl Broker {

    /// Gets the current balance of the user for a given symbol
    pub async fn get_balance(&self, symbol: String) -> Result<RestResponse<HashMap<String, Balance>>, GetBalanceError> {
        let timestamp = self.calculate_server_time()?;
        let mut mac = HmacSha256::new_from_slice(self.auth.secret.as_bytes())?;
        mac.update(format!(
            "api_key={}&coin={}&timestamp={}",
            self.auth.key, symbol, timestamp
        ).as_bytes());
        let signature = format!("{:X}", mac.finalize().into_bytes());
        let balance_res = self.client
            .get(
                    &format!(
                    "{}/v2/private/wallet/balance?api_key={}&coin={}&timestamp={}&sign={}",
                    CONFIG.bybit_rest_url ,self.auth.key, symbol, timestamp, signature
                ),
            )
            .send()
            .await?
            .text()
            .await?;
        let res = serde_json::from_str::<RestResponse<HashMap<String, Balance>>>(&balance_res)?;
        return Ok(res)
    }

    pub async fn get_all_balances(&self) -> Result<RestResponse<HashMap<String, Balance>>, GetBalanceError> {
        let timestamp = self.calculate_server_time()?;
        let mut mac = HmacSha256::new_from_slice(self.auth.secret.as_bytes())?;
        mac.update(format!("api_key={}&timestamp={}", self.auth.key, timestamp).as_bytes());
        let signature = format!("{:X}", mac.finalize().into_bytes());
        let balance_res = self.client
            .get(
            &format!(
                    "{}/v2/private/wallet/balance?api_key={}&timestamp={}&sign={}",
                    self.auth.url, self.auth.key, timestamp, signature
                ),
            )
            .send()
            .await?
            .text()
            .await?;
        let res = serde_json::from_str::<RestResponse<HashMap<String, Balance>>>(&balance_res)?;
        return Ok(res);
    }
}