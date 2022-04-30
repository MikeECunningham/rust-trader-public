use std::time::SystemTimeError;

use dec::D128;
use hmac::digest::InvalidLength;
use uuid::Uuid;
use thiserror::Error;

use crate::{backend::bybit::broker::MarketOrderJSON, strategy::types::Stage};
use crate::config::CONFIG;
use crate::SignRequestError;

use super::{Broker, OrderResult, LimitOrderJSON, RestResponse, CalculateServerTimeError, Side};

#[derive(Error, Debug)]
pub enum CreateOrderError {
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

/// Side should have been corrected for Stage by the time this reaches Broker.
/// Side means the literal position side, stage means reduce-only or not
impl Broker {
    pub async fn create_limit(
        &self,
        id: Uuid,
        symbol: String,
        price: D128,
        size: f64,
        side: Side,
        stage: Stage
    ) -> Result<(RestResponse<OrderResult>, String), CreateOrderError> {
        // println!("[DEBUG] Sending limit order");
        let timestamp = self.calculate_server_time()?;
        let side = match side {
            Side::Buy => "Buy",
            Side::Sell => "Sell",
        };
        let stage = match stage {
            Stage::Entry => false,
            Stage::Exit => true,
        };
        let time_in_force = "PostOnly".to_string();
        let ord = LimitOrderJSON {
            api_key: self.auth.key.clone(),
            close_on_trigger: false,
            order_type: "Limit".to_string(),
            price: price.to_string(),
            qty: size,
            reduce_only: stage,
            side: side.to_string(),
            symbol: symbol.clone(),
            time_in_force,
            timestamp,
            sign: String::default(),
            order_link_id: id.to_string(),
        }.get_signed_data(self.auth.secret.clone(), self.auth.key.clone())?;
        // info!("limit ord: {:?}", ord);
        let order_res = self.client
            .post(format!("{}/private/linear/order/create", CONFIG.bybit_rest_url))
            .header("Content-Type", "application/json")
            .body(ord.clone())
            .send()
            .await?
            .text()
            .await?;
        // info!("{} {} limit res: {}", side, stage, order_res);
        let ret =  serde_json::from_str::<RestResponse<OrderResult>>(&order_res)?;
        return Ok((ret, ord));
    }
    
    pub async fn create_market(
        &self,
        id: Uuid,
        symbol: String,
        size: f64,
        side: Side,
        stage: Stage
    ) -> Result<RestResponse<OrderResult>, CreateOrderError> {
        // println!("[DEBUG] Sending market order");
        let timestamp = self.calculate_server_time()?;
        let side = match side {
            Side::Buy => "Buy",
            Side::Sell => "Sell",
        };
        let close = match stage {
            Stage::Entry => false,
            Stage::Exit => true,
        };
        let ord = MarketOrderJSON {
            api_key: self.auth.key.clone(),
            close_on_trigger: false,
            order_type: "Market".to_string(),
            qty: size,
            reduce_only: close,
            side: side.to_string(),
            symbol: symbol.clone(),
            time_in_force: "GoodTillCancel".to_string(),
            order_link_id: id.to_string(),
            timestamp,
            sign: String::default(),
        }.get_signed_data(self.auth.secret.clone(), self.auth.key.clone())?;
        let order_res = self.client
            .post(format!("{}/private/linear/order/create", CONFIG.bybit_rest_url))
            .header("Content-Type", "application/json")
            .body(ord)
            .send()
            .await?
            .text()
            .await?;
        // info!("market res: {}", order_res);
        let ret = serde_json::from_str::<RestResponse<OrderResult>>(&order_res)?;
        return Ok(ret);
    }
}

