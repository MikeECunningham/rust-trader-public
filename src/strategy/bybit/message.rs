use std::collections::HashMap;

use dec::D128;

use crate::{analysis::BookResult, backend::bybit::{stream::{BybitOrderTick, BybitStopOrderTick, BybitExecutionTick, BybitPositionTick, BybitWalletTick}, broker::{RestResponse, Balance}}};

use super::{OrderResponse, CancelResponse};

pub struct Timestamps {
    pub init: D128,
    pub send: D128,
    pub last_update: D128,
    pub end: D128,
}

pub struct OrderBookMessage {
    pub orderbook_analysis: BookResult
}

pub struct TradeFlowMessage {
    pub timestamp: u128,
}

#[derive(Debug)]
pub struct BybitOrderTickSignal {
    pub order_tick: BybitOrderTick
}

pub enum ModelMessage {
    OrderBookMessage(OrderBookMessage),
    TradeFlowMessage(TradeFlowMessage),
}

pub enum OrderMessage {
    OrderResult(OrderResponse),
    CancelResult(CancelResponse),
    OrderUpdate(BybitOrderTickSignal),
    StopOrderUpdate(BybitStopOrderTick),
    ExecutionUpdate(BybitExecutionTick),
}

pub enum PositionMessage {
    PositionUpdate(BybitPositionTick),
}

pub enum WalletMessage {
    BalanceRefresh(RestResponse<HashMap<String, Balance>>),
    WalletUpdate(BybitWalletTick),
}

pub enum AccountMessage {
    OrderMessage(OrderMessage),
    PositionMessage(PositionMessage),
}

pub enum OpMessage {
    Init(u128),
    Timer(u128),
}

pub enum StrategyMessage {
    ModelMessage(ModelMessage),
    AccountMessage(AccountMessage),
}

