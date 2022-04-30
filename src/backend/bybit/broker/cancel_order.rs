use std::time::SystemTimeError;

use hmac::digest::InvalidLength;
use uuid::Uuid;
use thiserror::Error;

use crate::{config::CONFIG};
use crate::SignRequestError;

use super::CalculateServerTimeError;
use super::{Broker, types::{RestResponse, CancelResult, CancelJSON}};

#[derive(Error, Debug)]
pub enum CancelOrderError {
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
    pub async fn cancel_order(&self, symbol: String, order_link_id: Uuid, auto_report_id: Uuid) -> Result<RestResponse<CancelResult>, CancelOrderError> {
        // info!("cancelling: {}, {}", order_link_id.to_string(), auto_report_id);
        let timestamp = self.calculate_server_time()?;
        let can = CancelJSON {
            api_key: self.auth.key.clone(),
            order_link_id: order_link_id.to_string(),
            symbol,
            timestamp,
            sign: String::default(),
        }.get_signed_data(self.auth.secret.clone(), self.auth.key.clone())?;
        let cancel_res = self.client
            .post(format!("{}/private/linear/order/cancel", CONFIG.bybit_rest_url))
            .header("Content-Type", "application/json")
            .body(can)
            .send()
            .await?
            .text()
            .await?;
        // debug!("Cancel rest: {}", cancel_res);
        let ret = serde_json::from_str::<RestResponse<CancelResult>>(&cancel_res)?;
        return Ok(ret);
    }
}

