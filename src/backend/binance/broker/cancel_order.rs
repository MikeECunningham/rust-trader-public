use std::time::Instant;
use uuid::Uuid;
use crate::{config::CONFIG, backend::{binance::{types::{CancelRequest, CancelResponseWrapper}, broker::BROKER}}};
use super::Broker;

impl Broker {
    pub async fn cancel_order(
        &self,
        id: Uuid,
        symbol: String,
    ) -> CancelResponseWrapper {

        let req = CancelRequest {
            symbol,
            id,
            receive_window: 5000,
            timestamp: self.calculate_server_time().expect("Failed to calculate server time"),
        }.get_signed_data(self.auth.secret.clone()).expect("Sign error");
        info!("can req: {}", req);
        let timer = Instant::now();
        let cancel_res = self.client
            .delete(format!("{}/fapi/v1/order?{}", self.auth.url, req))
            .header("Content-Type", "application/json")
            .header("X-MBX-APIKEY", CONFIG.binance_key.clone())
            .send()
            .await
            .expect("error recv key response")
            .text()
            .await
            .expect("err");
        info!("can res: {}\ncan ping: {}", cancel_res, timer.elapsed().as_millis());
        let wrapper = serde_json::from_str::<CancelResponseWrapper>(&cancel_res).expect("serde err binance market res");
        match &wrapper {
            CancelResponseWrapper::Cancel(_) => {},
            CancelResponseWrapper::Error(e) => BROKER.error(e),
        }
        wrapper
    }
}