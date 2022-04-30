
pub mod orderbook;
pub mod private;
pub mod trade;

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;
// use tokio::sync::mpsc::Sender;

use async_tungstenite::tungstenite::protocol::Message;
use serde;

use super::broker::OrderStatus;


#[derive(Deserialize, Debug)]
pub struct TickLevel {
    pub price: String,
    pub symbol: String,
    pub id: String,
    pub side: String,
    pub size: f32,
}

#[derive(Deserialize, Debug)]
pub struct TickDelete {
    pub price: String,
    pub symbol: String,
    pub id: String,
    pub side: String,
}

#[derive(Deserialize, Debug)]
pub struct OBTickData {
    pub delete: Vec<TickDelete>,
    pub update: Vec<TickLevel>,
    pub insert: Vec<TickLevel>,
    // #[serde(rename = "transactTimeE6")]
    // pub transaction_time: i64,
}

#[derive(Deserialize, Debug)]
pub enum BybitTickTypes {
    Delete(TickDelete),
    Update(TickLevel),
}

#[derive(Deserialize, Debug)]
pub struct OBTick {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(rename = "topic")]
    pub channel: String,
    #[serde(rename = "timestamp_e6")]
    pub timestamp: String,
    pub cross_seq: String,
    pub data: OBTickData,
}

#[derive(Deserialize, Debug)]
pub struct InitBook {
    pub order_book: Vec<TickLevel>,
}

#[derive(Deserialize, Debug)]
pub struct TradeData {
    pub symbol: String,
    pub tick_direction: String,
    pub price: String,
    pub size: f64,
    pub timestamp: String,
    pub trade_time_ms: String,
    pub side: String,
    pub trade_id: String,
}

#[derive(Deserialize, Debug)]
pub struct TradeTick {
    pub topic: String,
    pub data: Vec<TradeData>,
}

#[derive(Deserialize, Debug)]
pub struct BybitOBInit {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(rename = "topic")]
    pub channel: String,
    #[serde(rename = "timestamp_e6")]
    pub timestamp: String,
    pub cross_seq: String,
    pub data: InitBook,
}

#[derive(Deserialize, Debug)]
pub struct BybitWalletTick {
    pub topic: String,
    pub data: Vec<BybitWalletData>,
}

#[derive(Deserialize, Debug)]
pub struct BybitWalletData {
    pub wallet_balance: f64,
    pub available_balance: f64,
}

#[derive(Deserialize, Debug)]
pub struct BybitPositionTick {
    pub topic: String,
    pub action: String,
    pub data: Vec<BybitPositionData>,
}

#[derive(Deserialize, Debug)]
pub struct BybitPositionData {
    pub user_id: String,
    pub symbol: String,
    pub size: f64,
    pub side: String,
    pub position_value: f64,
    pub entry_price: f64,
    pub bust_price: f64,
    pub leverage: f64,
    pub order_margin: f64,
    pub position_margin: f64,
    pub occ_closing_fee: f64,
    pub take_profit: f64,
    #[serde(default)]
    pub tp_trigger_by: String,
    pub stop_loss: f64,
    #[serde(default)]
    pub s1_trigger_by: String,
    #[serde(default)]
    pub realised_pnl: f64,
    #[serde(default)]
    pub cum_realized_pnl: f64,
    pub position_seq: String,
}

#[derive(Deserialize, Debug)]
pub struct BybitStopOrderTick {
    pub topic: String,
    pub data: Vec<BybitStopOrderData>,
}

#[derive(Deserialize, Debug)]
pub struct BybitStopOrderData {
    pub stop_order_id: String,
    pub order_link_id: String,
    pub user_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: f64,
    pub qty: f64,
    pub time_in_force: String,
    pub order_status: String,
    pub stop_order_type: String,
    #[serde(default)]
    pub trigger_by: String,
    #[serde(default)]
    pub trigger_price: f64,
    pub reduce_only: bool,
    pub close_on_trigger: bool,
    pub create_time: String,
    pub update_time: String,
}

#[derive(Deserialize, Debug)]
pub struct BybitOrderTick {
    pub topic: String,
    pub action: String,
    pub data: Vec<BybitOrderData>,
}

#[derive(Deserialize, Debug)]
pub struct BybitOrderData {
    pub order_id: String,
    pub order_link_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: f64,
    pub qty: f64,
    pub leaves_qty: f64,
    pub last_exec_price: f64,
    pub cum_exec_qty: f64,
    pub cum_exec_value: f64,
    pub cum_exec_fee: f64,
    pub time_in_force: String,
    pub create_type: String,
    pub cancel_type: String,
    pub order_status: OrderStatus,
    pub take_profit: f64,
    pub stop_loss: f64,
    pub trailing_stop: f64,
    pub reduce_only: bool,
    pub close_on_trigger: bool,
    pub create_time: String,
    pub update_time: String,
}

#[derive(Deserialize, Debug)]
pub struct BybitExecutionTick {
    pub topic: String,
    pub data: Vec<BybitExecutionData>,
}

#[derive(Deserialize, Debug)]
pub struct BybitExecutionData {
    pub symbol: String,
    pub side: String,
    pub order_id: String,
    pub exec_id: String,
    pub order_link_id: String,
    pub price: f64,
    pub order_qty: f64,
    pub exec_type: String,
    pub exec_fee: f64,
    pub exec_qty: f64,
    pub leaves_qty: f64,
    pub is_maker: bool,
    pub trade_time: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum PrivateTicks {
    PositionTick(BybitPositionTick),
    ExecutionTick(BybitExecutionTick),
    OrderTick(BybitOrderTick),
    StopOrderTick(BybitStopOrderTick),
    WalletTick(BybitWalletTick),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum WSPrivateTicks {
    PrivateTicks(PrivateTicks),
    WebsocketSuccessTick(WebsocketSuccessTick),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum OrderBookTicks {
    OBTick(OBTick),
    BybitOBInit(BybitOBInit),
    WebsocketSuccessTick(WebsocketSuccessTick),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum TradeTicks {
    TradeTick(TradeTick),
    WebsocketSuccessTick(WebsocketSuccessTick),
}

pub enum WebsocketMessager {
    Message(Message),
    Ping(),
}

#[derive(Deserialize, Debug)]
pub enum ArgType {
    String(String),
    U64(u64),
}

impl Serialize for ArgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ArgType::String(str) => serializer.serialize_str(str),
            ArgType::U64(i) => serializer.serialize_u64(*i),
        }
    }
}

/// Should be sent to the server to request orderbook data
#[derive(Serialize, Debug)]
struct WebsocketSubscribe {
    #[serde(rename = "op")]
    message_type: String,
    args: Vec<ArgType>,
}

#[derive(Deserialize, Serialize, Debug)]
struct WebsocketPing {
    #[serde(rename = "op")]
    message_type: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct WebsocketPrivate {
    order_id: String,
    sybmol: String,
    timestamp: u128,
    api_key: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BybitTimeTick {
    pub ret_code: i64,
    pub ret_msg: String,
    pub ext_code: String,
    pub ext_info: String,
    pub result: BybitTimeResult,
    pub time_now: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct WebsocketSuccessTick {
    pub success: bool,
    pub ret_msg: String,
    pub conn_id: String,
    pub request: WebsocketSuccessRequest,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct WebsocketSuccessRequest {
    pub op: String,
    pub args: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BybitTimeResult {}

#[derive(Deserialize, Serialize, Debug)]
struct RestWallet {
    pub ret_code: i64,
    pub ret_msg: String,
    pub ext_code: String,
    pub ext_info: String,
    pub time_now: String,
    pub result: Option<HashMap<String, RestWalletResultLevel>>,
    #[serde(default)]
    pub rate_limit_status: i64,
    #[serde(default)]
    pub rate_limit_reset_ms: i64,
    #[serde(default)]
    pub rate_limit: i64,
}

#[derive(Deserialize, Serialize, Debug)]
struct RestWalletResultLevel {
    pub equity: f64,
    pub available_balance: f64,
    pub used_margin: f64,
    pub order_margin: f64,
    pub position_margin: f64,
    pub occ_closing_fee: f64,
    pub wallet_balance: f64,
    pub realised_pnl: f64,
    pub unrealised_pnl: f64,
    pub cum_realised_pnl: f64,
    pub given_cash: f64,
    pub service_cash: f64,
}


#[derive(Deserialize, Debug)]
pub enum Signal {
    Orderbook(OBTick),
    Tradeflow(TradeTick),
    PrivateTicks(PrivateTicks),
}

pub struct BybitStream {
    pub server_time: Instant,
    pub orderbook_activated: bool,
    pub trades_activated: bool,
}

// Allegedly this must be here, and allegedy it must be unsafe
unsafe impl Sync for BybitStream {}

impl BybitStream {

    pub fn new() -> Self {
        BybitStream {
            server_time: Instant::now(),
            orderbook_activated: true,
            trades_activated: true,
        }
    }
}