use std::time::{SystemTimeError};

use hmac::Mac;
use hmac::digest::InvalidLength;
use thiserror::Error;

use crate::{config::CONFIG, HmacSha256};
use crate::SignRequestError;

use super::{RestResponse, QueryAllActiveOrdersResult, Broker, CalculateServerTimeError};

#[derive(Error, Debug)]
pub enum GetOrderError {
    #[error("Invalid mac key length")]
    MacLengthError(#[from] InvalidLength),
    #[error("Failed to send request")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Failed to serialize response")]
    SerdeError(#[from] serde_json::Error),
    #[error("Failed to sign the request")]
    SignRequestError(#[from] SignRequestError),
    #[error("Failed to get system time")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("Failed to calculate server time")]
    CalculateServerTimeError(#[from] CalculateServerTimeError)
}


impl Broker {
    pub async fn get_all_active_orders(&self, symbol: &String) -> Result<RestResponse<Vec<QueryAllActiveOrdersResult>>, GetOrderError> {
        // println!("[DEBUG] Getting all active orders");
        let timestamp = self.calculate_server_time()?;
        let mut mac = HmacSha256::new_from_slice(self.auth.secret.as_bytes())?;
        mac.update(
            format!(
                "api_key={}&symbol={}&timestamp={}",
                self.auth.key,
                symbol,
                timestamp,
            )
            .as_bytes(),
        );
        let signature = format!("{:x}", mac.finalize().into_bytes());
        let orders_res = self.client
            .get(&format!(
                        "{}/private/linear/order/search?api_key={}&symbol={}&timestamp={}&sign={}",
                        CONFIG.bybit_rest_url, self.auth.key, symbol, timestamp, signature
                ),
            )
            .send()
            .await?
            .text()
            .await?;
        let ret = serde_json::from_str::<RestResponse<Vec<QueryAllActiveOrdersResult>>>(&orders_res)?;
        return Ok(ret);
    }    
}

