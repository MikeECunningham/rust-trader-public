
use std::fmt::{Formatter, Display};
use std::ops::Not;

use dec::{D128};
use proc_macros::BybitSignable;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::backend::bybit::errors::PerpetualStatus;
use crate::SignRequestError;
use crate::backend::types::TimeInForce;

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum OrderType {
    Limit,
    Market,
}

impl TryFrom<String> for OrderType {
    type Error = &'static str;

    fn try_from(order_type: String) -> Result<Self, Self::Error> {
        let order_type = order_type.to_lowercase();
        if order_type == "limit" {
            Ok(OrderType::Limit)
        } else if order_type == "market" {
            Ok(OrderType::Market)
        } else {
            Err("Invalid Data: order_type must match a permutation of the OrderType enum")
        }
    }
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

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum OrderStatus {
    Created,
    Rejected,
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    PendingCancel,
}

#[derive(Debug)]
pub struct BybitAuth {
    /// Base URL for the data being accessed
    pub url: String,
    /// The public API key used for accessing the Bybit endpoint
    pub key: String,
    /// The private secret used for accessing the Bybit endpoint
    pub secret: String
}

#[derive(Serialize, Debug, BybitSignable)]
pub struct CancelJSON {
    pub api_key: String,
    pub order_link_id: String,
    pub symbol: String,
    pub timestamp: u128,
    pub sign: String,
}

#[derive(Serialize, Debug, BybitSignable)]
pub struct LimitOrderJSON {
    pub api_key: String,
    pub order_link_id: String,
    pub close_on_trigger: bool,
    pub order_type: String,
    pub price: String,
    pub qty: f64,
    pub side: String,
    pub symbol: String,
    pub reduce_only: bool,
    pub time_in_force: String,
    pub timestamp: u128,
    pub sign: String,
}

#[derive(Serialize, Debug, BybitSignable)]
pub struct MarketOrderJSON {
    pub api_key: String,
    pub order_link_id: String,
    pub close_on_trigger: bool,
    pub order_type: String,
    pub qty: f64,
    pub side: String,
    pub symbol: String,
    pub reduce_only: bool,
    pub time_in_force: String,
    pub timestamp: u128,
    pub sign: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CancelResult {
    order_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ReplaceResult {
    order_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrderResult {
    pub order_id: Uuid,
    pub user_id: u64,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<D128>,
    pub qty: D128,
    pub time_in_force: TimeInForce,
    pub order_status: CreateOrderStatus,
    pub last_exec_price: D128,
    #[serde(default)]
    pub leaves_qty: D128,
    pub reject_reason: Option<String>,
    pub cum_exec_qty: D128,
    pub cum_exec_value: D128,
    pub cum_exec_fee: D128,
    pub reduce_only: bool,
    pub close_on_trigger: bool,
    pub order_link_id: Uuid,
    pub created_time: String,
    pub updated_time: String,
    pub take_profit: D128,
    pub stop_loss: D128,
    pub tp_trigger_by: String,
    pub sl_trigger_by: String,
}

/// Represents complex balance information regarding
/// a portfolios balance for a given traded pair.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Balance {
    pub equity: D128,
    pub available_balance: D128,
    pub used_margin: D128,
    pub order_margin: D128,
    pub position_margin: D128,
    pub occ_closing_fee: D128,
    pub occ_funding_fee: D128,
    pub wallet_balance: D128,
    pub realised_pnl: D128,
    pub unrealised_pnl: D128,
    pub cum_realised_pnl: D128,
    pub given_cash: D128,
    pub service_cash: D128,
}

impl Balance {
    pub fn new() -> Balance {
        Balance {
            equity: D128::NAN,
            available_balance: D128::NAN,
            used_margin: D128::NAN,
            order_margin: D128::NAN,
            position_margin: D128::NAN,
            occ_closing_fee: D128::NAN,
            occ_funding_fee: D128::NAN,
            wallet_balance: D128::NAN,
            realised_pnl: D128::NAN,
            unrealised_pnl: D128::NAN,
            cum_realised_pnl: D128::NAN,
            given_cash: D128::NAN,
            service_cash: D128::NAN,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct QueryAllActiveOrdersResult {
    pub user_id: i64,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: f64,
    pub qty: f64,
    pub time_in_force: TimeInForce,
    pub order_status: OrderStatus,
    pub ext_fields: Option<ExtFields>,
    pub last_exec_time: Option<String>,
    pub reduce_only: bool,
    pub close_on_trigger: bool,
    pub last_exec_price: f64,
    pub leaves_qty: Option<f64>,
    pub leaves_value: Option<f64>,
    pub cum_exec_qty: f64,
    pub cum_exec_value: f64,
    pub cum_exec_fee: f64,
    pub reject_reason: Option<String>,
    pub cancel_type: Option<String>,
    pub order_link_id: String,
    pub order_id: String,
    pub take_profit: f64,
    pub stop_loss: f64,
    pub created_time: String,
    pub updated_time: String,
    pub tp_trigger_by: String,
    pub sl_trigger_by: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ExtFields {
    pub o_req_num: i64,
    pub xreq_type: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RestResponse<T> {
    pub ret_code: PerpetualStatus,
    pub ret_msg: String,
    pub ext_code: String,
    pub ext_info: String,
    pub result: Option<T>,
    pub time_now: String,
    pub rate_limit_status: Option<i32>,
    pub rate_limit_reset_ms: Option<u64>,
    pub rate_limit: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
pub enum Side {
    #[serde(alias="BUY")]
    #[serde(alias="buy")]
    #[serde(alias="LONG")]
    #[serde(alias="Long")]
    #[serde(alias="long")]
    Buy,
    #[serde(alias="SELL")]
    #[serde(alias="sell")]
    #[serde(alias="SHORT")]
    #[serde(alias="Short")]
    #[serde(alias="short")]
    Sell,
}

impl Not for Side {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", match self { Side::Buy => "Buy", Side::Sell => "Sell", })
    }
}

impl TryFrom<String> for Side {
    type Error = &'static str;

    fn try_from(side: String) -> Result<Self, Self::Error> {
        let side = side.to_lowercase();
        if side == "buy" || side == "long" || side == "bid" {
            Ok(Side::Buy)
        } else if side == "sell" || side == "short" || side == "ask" {
            Ok(Side::Sell)
        } else {
            Err("Invalid Data: side must match a permutation of the Side enum")
        }
    }
}

impl Side {
    /// Helps you deside
    pub fn deside<T>(&self, buy: T, sell: T) -> T {
        match self {
            Side::Buy => buy,
            Side::Sell => sell,
        }
    }
}