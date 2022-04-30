
use async_tungstenite::tungstenite::Message;
use dec::D128;
use proc_macros::BinanceSignable;
use serde_repr::Deserialize_repr;
use crate::SignRequestError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::{iter::Map, fmt::{Formatter, Display}, time::Instant};

use crate::backend::types::{TimeInForce, Side};

use super::errors::ErrorCode;

#[derive(Deserialize, Debug)]
pub struct BestLevel {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "u")]
    pub update_id: u64,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bid_price: D128,
    #[serde(rename = "B")]
    pub bid_qty: D128,
    #[serde(rename = "a")]
    pub ask_price: D128,
    #[serde(rename = "A")]
    pub ask_qty: D128,
    #[serde(skip)]
    #[serde(default = "instant_default")]
    pub test_timer: Instant,
}
pub fn instant_default() -> Instant { Instant::now() }

#[derive(Deserialize, Debug)]
pub struct MarginOrders {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id: u64,
    #[serde(rename = "u")]
    pub final_update_id: u64,
    #[serde(rename = "b")]
    pub bids: Vec<(D128, D128)>,
    #[serde(rename = "a")]
    pub asks: Vec<(D128, D128)>,
}

// #[serde_as]
#[derive(Deserialize, Debug)]
pub struct Orders {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id: u64,
    #[serde(rename = "u")]
    pub last_update_id: u64,
    #[serde(rename = "pu")]
    pub last_stream_final_update_id: u64,
    #[serde(rename = "b")]
    pub bids: Vec<(D128, D128)>,
    #[serde(rename = "a")]
    pub asks: Vec<(D128, D128)>,
    #[serde(skip)]
    #[serde(default = "instant_default")]
    pub test_timer: Instant,
}

#[derive(Deserialize, Debug)]
pub struct Trades {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u128,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "t")]
    pub trade_id: u64,
    #[serde(rename = "p")]
    pub price: D128,
    #[serde(rename = "q")]
    pub quantity: D128,
    #[serde(rename = "b")]
    pub buyer_order_id: u64,
    #[serde(rename = "a")]
    pub seller_order_id: u64,
    #[serde(rename = "m")]
    pub buyer_maker: bool,
    #[serde(rename = "M")]
    pub ignore: bool,
}

#[derive(Deserialize, Debug)]
pub struct FuturesTrades {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "a")]
    pub aggregate_trade_id: u64,
    #[serde(rename = "f")]
    pub first_trade_id: u64,
    #[serde(rename = "l")]
    pub last_trade_id: u64,
    #[serde(rename = "p")]
    pub price: D128,
    #[serde(rename = "q")]
    pub quantity: D128,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "m")]
    pub buyer_maker: bool,
    #[serde(skip)]
    #[serde(default = "instant_default")]
    pub test_timer: Instant,
}

#[derive(Deserialize, Debug)]
pub struct Liquidation {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "o")]
    pub order: LiquidationOrder,
}

#[derive(Deserialize, Debug)]
pub struct LiquidationOrder {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "S")]
    pub side: BinanceSide,
    #[serde(rename = "o")]
    pub order_type: OrderType,
    #[serde(rename = "f")]
    pub time_in_force: TimeInForce,
    #[serde(rename = "q")]
    pub quantity: D128,
    #[serde(rename = "p")]
    pub price: D128,
    #[serde(rename = "ap")]
    pub average_price: D128,
    #[serde(rename = "X")]
    pub order_status: OrderStatus,
    #[serde(rename = "l")]
    pub last_quantity: D128,
    #[serde(rename = "z")]
    pub cum_filled_quantity: D128,
    #[serde(rename = "T")]
    pub trade_time: u64,
}

#[derive(Deserialize, Debug)]
pub struct BookRefresh {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    #[serde(rename = "E")]
    pub message_output_time: u64,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    pub bids: Vec<(D128, D128)>,
    pub asks: Vec<(D128, D128)>
}

#[derive(Deserialize, Debug)]
pub struct StreamExpired {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
}

#[derive(Deserialize, Debug)]
pub struct MarginCall {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "cw")]
    pub cross_wallet_balance: Option<D128>,
    #[serde(rename = "p")]
    pub positions: Vec<MarginCallPosition>,
}

#[derive(Deserialize, Debug)]
pub struct MarginCallPosition {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "ps")]
    pub position_side: BinanceSide,
    #[serde(rename = "mt")]
    pub margin_type: MarginType,
    #[serde(rename = "iw")]
    pub isolated_wallet: Option<u32>,
    #[serde(rename = "mp")]
    pub mark_price: D128,
    #[serde(rename = "up")]
    pub unrealized_pnl: D128,
    #[serde(rename = "mm")]
    pub maintenance_margin: D128,
}

#[derive(Deserialize, Debug)]
pub struct UserStreamWrapper<T> {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(alias = "a")]
    #[serde(alias = "o")]
    pub data: T
}

#[derive(Deserialize, Clone, Debug)]
pub struct OrderUpdateData {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "c")]
    pub id: String,
    #[serde(rename = "S")]
    pub side: BinanceSide,
    #[serde(rename = "o")]
    pub order_type: OrderType,
    #[serde(rename = "f")]
    pub time_in_force: TimeInForce,
    #[serde(rename = "q")]
    pub original_qty: D128,
    #[serde(rename = "p")]
    pub original_price: D128,
    #[serde(rename = "ap")]
    pub average_price: D128,
    #[serde(rename = "sp")]
    pub stop_price: D128,
    #[serde(rename = "x")]
    pub execution_type: ExecutionType,
    #[serde(rename = "X")]
    pub order_status: OrderStatus,
    #[serde(rename = "i")]
    pub auto_id: u64,
    #[serde(rename = "l")]
    pub last_filled_qty: D128,
    #[serde(rename = "z")]
    pub accumulated_filled_qty: D128,
    #[serde(rename = "L")]
    pub filled_price: D128,
    #[serde(rename = "N")]
    pub commission_asset: Option<String>,
    #[serde(rename = "n")]
    pub commission: Option<D128>,
    #[serde(rename = "b")]
    pub bid_notional: D128,
    #[serde(rename = "a")]
    pub ask_notional: D128,
    #[serde(rename = "m")]
    pub maker: bool,
    #[serde(rename = "R")]
    pub reduce_only: bool,
    #[serde(rename = "wt")]
    pub stop_price_working_type: WorkingType,
    #[serde(rename = "ot")]
    pub original_order_type: OrderType,
    #[serde(rename = "ps")]
    pub position_side: BinanceSide,
    #[serde(rename = "cp")]
    pub close_all: Option<bool>,
    #[serde(rename = "AP")]
    pub activation_price: Option<D128>,
    #[serde(rename = "cr")]
    pub callback_rate: Option<D128>,
    #[serde(rename = "rp")]
    pub realized_profit: D128,
    #[serde(rename = "pP")]
    pub ignore_pp: bool,
    #[serde(rename = "si")]
    pub ignore_si: u32,
    #[serde(rename = "ss")]
    pub ignore_ss: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PositionUpdateData {
    #[serde(rename = "m")]
    pub event_reason: PositionEventReason,
    #[serde(rename = "B")]
    pub balances: Vec<PositionUpdateBalance>,
    #[serde(rename = "P")]
    pub positions: Vec<PositionUpdatePosition>
}

#[derive(Deserialize, Clone, Debug)]
pub struct PositionUpdatePosition {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "pa")]
    pub quantity: D128,
    #[serde(rename = "ep")]
    pub price: D128,
    /// Pre-fee
    #[serde(rename = "cr")]
    pub accumulated_realized: D128,
    #[serde(rename = "up")]
    pub unrealized_pnl: D128,
    #[serde(rename = "mt")]
    pub margin_type: MarginType,
    #[serde(rename = "ps")]
    pub side: BinanceSide,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PositionUpdateBalance {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "wb")]
    pub wallet_balance: D128,
    #[serde(rename = "cw")]
    pub cross_wallet_balance: D128,
    /// Except PnL and Commission
    #[serde(rename = "bc")]
    pub balance_change: D128
}

#[derive(Deserialize, Clone, Debug)]
pub struct PositionUpdate {
    pub balance: PositionUpdateBalance,
    pub position: PositionUpdatePosition,
}

#[derive(Serialize, BinanceSignable, Debug)]
pub struct MarketOrderRequest {
    pub symbol: String,
    pub side: BinanceSide,
    #[serde(rename = "positionSide")]
    pub position_side: BinancePositionSide,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub quantity: f64,
    #[serde(rename = "newClientOrderId")]
    pub id: Uuid,
    #[serde(rename = "newOrderRespType")]
    pub order_response_type: OrderResponseType,
    #[serde(rename = "recvWindow")]
    pub receive_window: u64,
    pub timestamp: u64,
}

#[derive(Serialize, BinanceSignable, Debug)]
pub struct LimitOrderRequest {
    pub symbol: String,
    pub side: BinanceSide,
    #[serde(rename = "positionSide")]
    pub position_side: BinancePositionSide,
    // pub price: D128,
    pub price: f64,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub quantity: f64,
    #[serde(rename = "timeInForce")]
    pub time_in_force: BinanceTimeInForce,
    #[serde(rename = "newClientOrderId")]
    pub id: Uuid,
    #[serde(rename = "newOrderRespType")]
    pub order_response_type: OrderResponseType,
    #[serde(rename = "recvWindow")]
    pub receive_window: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Debug)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: Side,
    #[serde(rename = "positionSide")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<Side>,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    #[serde(rename = "timeInForce")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<D128>,
    #[serde(rename = "reduceOnly")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<D128>,
    #[serde(rename = "newClientOrderId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    #[serde(rename = "stopPrice")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<Uuid>,
    #[serde(rename = "closePosition")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_position: Option<bool>,
    #[serde(rename = "activationPrice")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_price: Option<D128>,
    #[serde(rename = "callbackRate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_rate: Option<D128>,
    #[serde(rename = "workingType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_type: Option<WorkingType>,
    #[serde(rename = "priceProtect")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_protect: Option<bool>,
    #[serde(rename = "newOrderRespType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_response_type: Option<OrderResponseType>,
    #[serde(rename = "recvWindow")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receive_window: Option<u64>,
    pub timestamp: u128,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    #[serde(alias = "clientOrderId")]
    pub id: Uuid,
    pub cum_qty: D128,
    pub cum_quote: D128,
    pub executed_qty: D128,
    #[serde(rename = "orderId")]
    pub auto_id: u64,
    pub avg_price: D128,
    pub orig_qty: D128,
    pub price: D128,
    pub reduce_only: Option<bool>,
    pub side: Side,
    pub position_side: BinanceSide,
    pub status: OrderStatus,
    pub stop_price: Option<D128>,
    pub close_position: Option<bool>,
    pub symbol: String,
    pub time_in_force: TimeInForce,
    #[serde(alias = "type")]
    pub order_type: OrderType,
    pub orig_type: OrderType,
    pub activate_price: Option<D128>,
    pub price_rate: Option<D128>,
    pub update_time: u64,
    pub working_type: WorkingType,
    pub price_protect: Option<bool>,
}

#[derive(Serialize, BinanceSignable, Debug)]
pub struct CancelRequest {
    pub symbol: String,
    #[serde(rename = "origClientOrderId")]
    pub id: Uuid,
    #[serde(rename = "recvWindow")]
    pub receive_window: u64,
    pub timestamp: u64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CancelResponse {
    #[serde(alias = "clientOrderId")]
    pub id: Uuid,
    pub cum_qty: D128,
    pub cum_quote: D128,
    pub executed_qty: D128,
    #[serde(rename = "orderId")]
    pub auto_id: u64,
    pub orig_qty: D128,
    pub orig_type: OrderType,
    pub price: D128,
    pub reduce_only: Option<bool>,
    pub side: Side,
    pub position_side: BinanceSide,
    pub status: OrderStatus,
    pub stop_price: Option<D128>,
    pub close_position: Option<bool>,
    pub symbol: String,
    pub time_in_force: TimeInForce,
    #[serde(alias = "type")]
    pub order_type: OrderType,
    pub activate_price: Option<D128>,
    pub price_rate: Option<D128>,
    pub update_time: u64,
    pub working_type: WorkingType,
    pub price_protect: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct WebsocketSubscribe {
    pub method: String,
    pub params: Vec<String>,
    pub id: u64,
}

#[derive(Deserialize, Debug)]
pub struct SubscribeResponse {
    pub result: Option<Vec<String>>,
    pub id: u64,
}

#[derive(Deserialize, Debug)]
pub struct StreamWrapper<T> {
    pub stream: String,
    pub data: T,
}

#[derive(Debug)]
pub struct BinanceAuth {
    /// Base URL for the data being accessed
    pub url: String,
    /// The public API key used for accessing the Bybit endpoint
    pub key: String,
    /// The private secret used for accessing the Bybit endpoint
    pub secret: String
}

#[derive(Deserialize, Clone, Debug)]
pub struct BinanceError {
    pub code: ErrorCode,
    pub msg: String,
}

#[derive(Deserialize, Copy, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerTimeResponse {
    pub server_time: i64,
}



/// TYPE ENUMS



#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum BinanceTimeInForce {
    #[serde(rename="GTC")]
    GoodTillCancel,
    #[serde(rename="IOC")]
    ImmediateOrCancel,
    #[serde(rename="FOK")]
    FillOrKill,
    #[serde(rename="GTX")]
    GoodTillCrossing,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderResponseType {
    Result,
    Ack
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum CreateOrderStatus {
    Created,
    Rejected,
    Active,
    Untrigerred,
    Triggered,
    Cancelled,
    Deactivated,
}

impl Display for OrderResponseType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", match self { OrderResponseType::Result => "RESULT", OrderResponseType::Ack => "ACK", })
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PositionEventReason {
    Deposit,
    Withdraw,
    Order,
    FundingFee,
    WithdrawReject,
    Adjustment,
    InsuranceClear,
    AdminDeposit,
    AdminWithdraw,
    MarginTransfer,
    MarginTypeChange,
    AssetTransfer,
    OptionsPremiumFee,
    OptionsSettleProfit,
    AutoExchange
}

#[derive(Deserialize, Clone, Copy, Serialize, Debug)]
pub enum BinanceSide {
    #[serde(alias="BUY")]
    #[serde(alias="buy")]
    #[serde(alias="LONG")]
    #[serde(alias="Long")]
    #[serde(alias="long")]
    #[serde(rename(serialize = "BUY"))]
    Buy,
    #[serde(alias="SELL")]
    #[serde(alias="sell")]
    #[serde(alias="SHORT")]
    #[serde(alias="Short")]
    #[serde(alias="short")]
    #[serde(rename(serialize = "SELL"))]
    Sell,
    #[serde(alias = "BOTH")]
    #[serde(alias = "both")]
    #[serde(rename(serialize = "BOTH"))]
    Both
}

impl Display for BinanceSide {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", match self { BinanceSide::Buy => "BUY", BinanceSide::Sell => "SELL", BinanceSide::Both => "BOTH" })
    }
}

impl From<Side> for BinanceSide {
    fn from(side: Side) -> Self {
        match side {
            Side::Buy => BinanceSide::Buy,
            Side::Sell => BinanceSide::Sell,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum BinancePositionSide {
    #[serde(alias="BUY")]
    #[serde(alias="buy")]
    #[serde(alias="LONG")]
    #[serde(alias="Long")]
    #[serde(alias="long")]
    #[serde(rename(serialize = "LONG"))]
    Long,
    #[serde(alias="SELL")]
    #[serde(alias="sell")]
    #[serde(alias="SHORT")]
    #[serde(alias="Short")]
    #[serde(alias="short")]
    #[serde(rename(serialize = "SHORT"))]
    Short,
    #[serde(alias = "BOTH")]
    #[serde(alias = "both")]
    #[serde(rename(serialize = "BOTH"))]
    Both
}
impl From<Side> for BinancePositionSide {
    fn from(side: Side) -> Self {
        match side {
            Side::Buy => BinancePositionSide::Long,
            Side::Sell => BinancePositionSide::Short,
        }
    }
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub enum MarginType {
    #[serde(alias = "ISOLATED")]
    #[serde(alias = "isolated")]
    Isolated,
    #[serde(alias = "CROSS")]
    #[serde(alias = "cross")]
    Cross
}

#[derive(Serialize, Debug)]
pub enum DepthLimit {
    Five = 5,
    Ten = 10,
    Twenty = 20,
    Fifty = 50,
    Hundred = 100,
    FiveHundred = 500,
    Thousand = 1000,
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    #[serde(rename = "CANCELED")]
    Cancelled,
    Expired,
    NewInsurance,
    NewADL,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    TakeProfit,
    TakeProfitMarket,
    Liquidation,
    TrailingStopMarket,
}

impl Display for OrderType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", match self {
            OrderType::Market => "MARKET",
            OrderType::Limit => "LIMIT",
            OrderType::Stop => "STOP",
            OrderType::TakeProfit => "TAKE_PROFIT",
            OrderType::Liquidation => "LIQUIDATION",
            OrderType::TakeProfitMarket => "TAKE_PROFIT_MARKET",
            OrderType::TrailingStopMarket => "TRAILING_STOP_MARKET",
        })
    }
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionType {
    New,
    #[serde(rename = "CANCELED")]
    Cancelled,
    Calculated,
    Expired,
    Trade
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkingType {
    MarkPrice,
    ContractPrice
}


/// ROUTING ENUMS


#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum OrderResponseWrapper {
    Order(OrderResponse),
    Error(BinanceError)
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum CancelResponseWrapper {
    Cancel(CancelResponse),
    Error(BinanceError)
}

pub enum WebsocketMessager {
    Message(Message),
    Ping(),
}

#[derive(Deserialize, Debug)]
// #[serde(tag = "e")]
#[serde(untagged)]
pub enum UserDataStreams {
    // #[serde(rename = "ORDER_TRADE_UPDATE")]
    OrderUpdate(UserStreamWrapper<OrderUpdateData>),
    // #[serde(rename = "ACCOUNT_UPDATE")]
    PositionUpdate(UserStreamWrapper<PositionUpdateData>),
    // #[serde(rename = "listenKeyExpired")]
    // StreamExpired(StreamExpired),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum TradeFlows {
    Trades(FuturesTrades),
    Liquidations(Liquidation),
}

#[derive(Debug)]
pub enum Signal {
    OrderBook(OrderBookSignal),
    TradeFlows(TradeFlows),
}

#[derive(Debug)]
pub enum OrderBookSignal {
    OrderBook(Orders),
    BestLevels(BestLevel),
    OrderBookSnap(BookRefresh),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum OrderbookResponse {
    Subscription(SubscribeResponse),
    Order(Orders)
}

