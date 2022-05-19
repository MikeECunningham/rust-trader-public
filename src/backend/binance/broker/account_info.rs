use std::time::{SystemTimeError, Instant, UNIX_EPOCH, SystemTime};

use dec::D128;
use uuid::Uuid;

use crate::{config::CONFIG, backend::{types::{Side}, binance::{types::{OrderType, MarketOrderRequest, OrderResponseType, BinanceSide, BinancePositionSide, OrderResponseWrapper, LimitOrderRequest, BinanceTimeInForce, AccountBalanceRequest, AccountBalance, AccountBalanceWrapper}, broker::BROKER}}, strategy::types::Stage};

use super::Broker;

impl Broker {
    pub async fn account_balance(&self) -> AccountBalanceWrapper {
        let req = AccountBalanceRequest {
            receive_window: 5000,
            timestamp: self.calculate_server_time().expect("Failed to calculate server time"),
        }.get_signed_data(self.auth.secret.clone()).expect("Sign error");

        let balance_res = self.client
            .get(format!("{}/fapi/v2/balance?{}", self.auth.url, req))
            .header("Content-Type", "application/json")
            .header("X-MBX-APIKEY", CONFIG.binance_key.clone())
            .send()
            .await
            .expect("error recv key response")
            .text()
            .await
            .expect("err");
        info!("acc bal {}", balance_res);
        let balances = serde_json::from_str::<AccountBalanceWrapper>(&balance_res).expect("err deser acc bal");
        match &balances {
            AccountBalanceWrapper::Balance(_) => {},
            AccountBalanceWrapper::Error(e) => BROKER.error(e),
        }
        balances
    }
}