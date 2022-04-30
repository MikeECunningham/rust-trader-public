use crate::{backend::binance::types::{PositionUpdateData, OrderUpdateData, OrderResponse}, analysis::{BookResult, TradeResult}, orderbook::Tops};

use super::{OrderResponseContext, CancelResponseContext};

#[derive(Clone, Debug)]
pub enum AccountMessage {
    PositionUpdate(PositionUpdateData),
    OrderUpdate(OrderUpdateData),
    OrderResponse(OrderResponseContext),
    CancelResponse(CancelResponseContext),

}

#[derive(Clone, Debug)]
pub enum ModelMessage {
    TradeFlowMessage(TradeResult),
    OrderBookMessage(BookResult),
    TopsMessage(Tops),
}

#[derive(Clone, Debug)]
pub enum StrategyMessage {
    ModelMessage(ModelMessage),
    AccountMessage(AccountMessage)
}