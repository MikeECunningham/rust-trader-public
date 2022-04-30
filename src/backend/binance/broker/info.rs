use std::time::Instant;

use crate::backend::binance::types::ServerTimeResponse;

use super::Broker;

impl Broker {
    pub async fn ping(&self) -> u128 {
        let ping = Instant::now();
        self.client
            .get(format!("{}/fapi/v1/ping", self.auth.url))
            .send()
            .await
            .expect("binance ping error")
            .text()
            .await
            .unwrap();
        ping.elapsed().as_millis()
    }

    pub async fn time(&self) -> i64 {
        let time = self.client
            .get(format!("{}/fapi/v1/time", self.auth.url))
            .send()
            .await
            .expect("binance ping error")
            .text()
            .await
            .unwrap();
        info!("time {}", time);
        serde_json::from_str::<ServerTimeResponse>(&time).expect("server time error").server_time
    }
}