use crate::{config::CONFIG, backend::binance::types::{DepthLimit, BookRefresh}};

use super::{Market};

impl Market {
    pub async fn orderbook_snapshot(&self, symbol: String, limit: DepthLimit) -> BookRefresh {

        let snap = self.client
            .get(format!("{}/fapi/v1/depth?symbol={}&limit={}", CONFIG.binance_rest_url, symbol, (limit as u32)))
            .send()
            .await
            .expect("error receiving ob snapshot response")
            .text()
            .await
            .expect("error getting ob snap response text");
        serde_json::from_str::<BookRefresh>(&snap).expect("error deserializing ob snap res")
    }
}