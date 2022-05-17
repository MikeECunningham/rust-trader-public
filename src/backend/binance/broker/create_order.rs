use std::time::{SystemTimeError, Instant, UNIX_EPOCH, SystemTime};

use dec::D128;
use uuid::Uuid;

use crate::{config::CONFIG, backend::{types::{Side}, binance::{types::{OrderType, MarketOrderRequest, OrderResponseType, BinanceSide, BinancePositionSide, OrderResponseWrapper, LimitOrderRequest, BinanceTimeInForce}, broker::BROKER}}, strategy::types::Stage};

use super::Broker;

impl Broker {
    pub async fn create_market(
        &self,
        id: Uuid,
        symbol: String,
        size: f64,
        side: Side,
        stage: Stage,
    ) -> OrderResponseWrapper {
        let ord_side =  match side {
            Side::Buy => match stage {
                Stage::Entry => BinanceSide::Buy,
                Stage::Exit => BinanceSide::Sell,
            },
            Side::Sell => match stage {
                Stage::Entry => BinanceSide::Sell,
                Stage::Exit => BinanceSide::Buy,
            },
        };

        let req = MarketOrderRequest {
            symbol,
            side: ord_side,
            position_side: BinancePositionSide::from(side),
            order_type: OrderType::Market,
            quantity: size,
            id,
            receive_window: 5000,
            timestamp: self.calculate_server_time().expect("Failed to calculate server time"),
            order_response_type: OrderResponseType::Result,
        }.get_signed_data(self.auth.secret.clone()).expect("Sign error");
        // info!("req: {}", req);
        let timer = Instant::now();
        let order_res = self.client
            .post(format!("{}/fapi/v1/order?{}", self.auth.url, req))
            .header("Content-Type", "application/json")
            .header("X-MBX-APIKEY", CONFIG.binance_key.clone())
            .send()
            .await
            .expect("error recv key response")
            .text()
            .await
            .expect("err");
        // info!("{}\n{}", order_res, timer.elapsed().as_millis());
        let wrapper = serde_json::from_str::<OrderResponseWrapper>(&order_res).expect("serde err binance market res");
        match &wrapper {
            OrderResponseWrapper::Order(_) => {},
            OrderResponseWrapper::Error(e) => BROKER.error(e),
        }
        wrapper
    }

    pub async fn create_limit(
        &self,
        id: Uuid,
        symbol: String,
        // price: D128,
        price: f64,
        size: f64,
        side: Side,
        stage: Stage,
    ) -> OrderResponseWrapper {

        let ord_side =  match side {
            Side::Buy => match stage {
                Stage::Entry => BinancePositionSide::Long,
                Stage::Exit => BinancePositionSide::Short,
            },
            Side::Sell => match stage {
                Stage::Entry => BinancePositionSide::Short,
                Stage::Exit => BinancePositionSide::Long,
            },
        };
        let hedge_side = BinanceSide::from(side);

        let req = LimitOrderRequest {
            symbol,
            side: hedge_side, // The direction of the positionw we're modifying
            position_side: ord_side, // The direction of the actual order being placed, binance is a mess
            price,
            order_type: OrderType::Limit,
            quantity: size,
            time_in_force: BinanceTimeInForce::GoodTillCrossing,
            id,
            order_response_type: OrderResponseType::Result,
            receive_window: 5000,
            timestamp: self.calculate_server_time().expect("Failed to calculate server time"),
        }.get_signed_data(self.auth.secret.clone()).expect("Sign error");

        info!("req: {}", req);
        let timer = Instant::now();
        let order_res = self.client
            .post(format!("{}/fapi/v1/order?{}", self.auth.url, req))
            .header("Content-Type", "application/json")
            .header("X-MBX-APIKEY", CONFIG.binance_key.clone())
            .send()
            .await
            .expect("error recv key response")
            .text()
            .await
            .expect("err");
        info!("order ping: {}", timer.elapsed().as_millis());
        let wrapper = serde_json::from_str::<OrderResponseWrapper>(&order_res).expect("serde err binance market res");
        match &wrapper {
            OrderResponseWrapper::Order(_) => {},
            OrderResponseWrapper::Error(e) => BROKER.error(e),
        }
        wrapper
    }
}