use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crossbeam_channel::{SendError, Sender};
use dec::{D128, Context};
use tokio::runtime::Handle;
use uuid::Uuid;
use thiserror::Error;

use crate::backend::bybit::broker::{OrderStatus, CreateOrderError, CancelOrderError, OrderResult, OrderType, Side};
use crate::backend::bybit::stream::BybitPositionData;
use crate::backend::bybit::broker::BROKER;
use crate::strategy::types::{Stage, OrderClassification};

use super::order_list::{OrderList, OrderListError, AllLiqs, OrderData};
use super::{StrategyMessage, Order, IncomingOrderREST, IncomingOrderWS, AccountMessage, OrderMessage, OrderResponse, CancelResponse, message, REBATE, OrderProgress};


#[derive(Clone, Copy, PartialEq)]
pub enum FindCancelRes {
    Found,
    Cancelled,
    NotFound
}

#[derive(Clone, Copy)]
pub struct FinData {
    pub inv: D128,
    pub liq: D128,
    pub cb: D128,
}

impl Display for FinData {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Fin Data: {{ inv: {}, liq: {}, cb: {} }}",
        self.inv, self.liq, self.cb)
    }
}

impl FinData {
    pub fn new() -> FinData {
        FinData {
            inv: D128::ZERO,
            liq: D128::ZERO,
            // Overloading problem here with OrderData's terminology because this is pre-rebate cb, TODO
            cb: D128::ZERO,
        }
    }

    pub fn patch(&mut self, inv: D128, liq: D128, rebate: D128) {
        self.inv += inv;
        self.liq += liq;
    }

    pub fn process(&mut self) {
        self.cb = self.liq / self.inv;
    }

    pub fn update(&mut self, inv: D128, liq: D128, rebate: D128) {
        self.patch(inv, liq, rebate);
        self.process();
    }
}

impl From<OrderData> for FinData {
    fn from(order_data: OrderData) -> Self {
        Self {
            inv: order_data.inv,
            liq: order_data.liq,
            cb: order_data.cb,
        }
    }
}

#[derive(Clone, Copy)]
pub struct PositionData {
    pub open_liqs: AllLiqs,
    pub close_liqs: AllLiqs,
    pub open_position: FinData,
    pub total_count: D128,
    pub remaining_margin: D128,
    pub remaining_count: D128,
    // pub active_delta: D128,
    // pub total_delta: D128,
}

impl Display for PositionData {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Position Data:\n    {{\n      open orders: {},\n      close orders: {},\n      open position: {}\n      total count: {}, remaining margin: {}, remaining count: {}\n    }}",
        self.open_liqs, self.close_liqs, self.open_position, self.total_count, self.remaining_margin, self.remaining_count)
    }
}

impl PositionData {
    pub fn new() -> PositionData {
        PositionData {
            open_liqs: AllLiqs::new(),
            close_liqs: AllLiqs::new(),
            open_position: FinData::new(),
            total_count: D128::ZERO,
            remaining_count: D128::ZERO,
            remaining_margin: D128::ZERO,
            // active_delta: D128::ZERO,
            // total_delta: D128::ZERO,
        }
    }

    pub fn neutral_cb(&self, rebate: D128, side: Side) -> D128 {
        match side {
            Side::Buy => self.open_liqs.neutral_cb(rebate).round_down(0),
            Side::Sell => self.open_liqs.neutral_cb(rebate).round_up(0),
        }
    }

    pub fn uncancelled_neutral_cb(&self, rebate: D128, side: Side) -> D128 {
        match side {
            Side::Buy => self.open_liqs.uncancelled_neutral_cb(rebate).round_down(0),
            Side::Sell => self.open_liqs.uncancelled_neutral_cb(rebate).round_up(0),
        }
    }
}

#[derive(Error, Debug)]
pub enum SendOrderPositionError {
    #[error("Failed to create the order")]
    CreateOrderError(#[from] CreateOrderError),
}

#[derive(Error, Debug)]
pub enum CancelOrderPositionError {
    #[error("Failed to cancel the order")]
    CancelOrderError(#[from] CancelOrderError),
    #[error("Failed to send over channel")]
    SendError(#[from] SendError<message::StrategyMessage>),
}

impl From<&BybitPositionData> for IncomingPosition {
    fn from(position: &BybitPositionData) -> Self {
        IncomingPosition {
            symbol: position.symbol.clone(),
            size: D128::from(position.size),
            side: Side::try_from(position.side.clone()).expect("incoming position"),
            liq: D128::from(position.position_value),
            price: D128::from(position.entry_price),
            leverage_multiplier: D128::from(position.leverage),
            order_margin_used: D128::from(position.order_margin),
            position_margin_available: D128::from(position.position_margin),
            realised_pnl: D128::from(position.realised_pnl),
            cum_realised_pnl: D128::from(position.cum_realized_pnl),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IncomingPosition {
    pub symbol: String,
    pub size: D128,
    pub side: Side,
    pub liq: D128,
    pub price: D128,
    pub leverage_multiplier: D128,
    pub order_margin_used: D128,
    pub position_margin_available: D128,
    pub realised_pnl: D128,
    pub cum_realised_pnl: D128,
}

#[derive(Debug)]
pub struct Position {
    pub symbol: String,
    pub opens: OrderList,
    pub closes: OrderList,
    pub side: Side,
    pub pos_max_orders: D128,
    pub pos_max_size: D128,
    pub known_size: D128,
    pub known_price: D128,
    pub known_liq: D128,
    pub known_available_liq: D128,
    pub known_prebate_pnl: D128,
    pub sequence: D128,
    pub pool: Handle
}

impl Position {
    pub fn new(pool: Handle, symbol: String, open_side: Side, max_size: D128, max_count: D128) -> Position {
        Position {
            symbol,
            side: open_side,
            opens: OrderList::new(),
            closes: OrderList::new(),
            sequence: D128::ZERO,
            pos_max_size: max_size,
            pos_max_orders: max_count,
            known_prebate_pnl: D128::ZERO,
            known_size: D128::ZERO,
            known_price: D128::ZERO,
            known_liq: D128::ZERO,
            known_available_liq: D128::ZERO,
            pool,
        }
    }

    pub fn get_top(&self, stage: Stage) -> Option<&Order> {
        match stage { Stage::Entry => self.opens.get_top(), Stage::Exit => self.closes.get_top(), }
    }

    pub fn get_top_data(&self, stage: Stage) -> Option<OrderData> {
        match stage { Stage::Entry => self.opens.get_top_data(), Stage::Exit => self.closes.get_top_data(), }
    }

    pub fn cancel_non_tops(&mut self, best: D128, stage: Stage, sender: Sender<StrategyMessage>) -> FindCancelRes {
        let mut found_top = FindCancelRes::NotFound;

        for (_, order)
        in match stage { Stage::Entry => &mut self.opens, Stage::Exit => &mut self.closes }
        .order_map.iter_mut()
        .filter(|(_, ord)| ord.order_class == OrderClassification::Top && ord.can_cancel()) {
            if found_top == FindCancelRes::NotFound { found_top = FindCancelRes::Found }
            if order.price != best {
                found_top = FindCancelRes::Cancelled;
                // info!("Cancelling a non-top {:?} at {}, best is {}", stage, order.price, best);
                Position::cancel_order(self.pool.clone(), order, self.side, stage, self.symbol.clone(), sender.clone());
            }
        }
        found_top
    }

    pub fn get_smallest_rebase_size(&self, stage: Stage) -> Option<D128> {
        match match stage { Stage::Entry => &self.opens, Stage::Exit => &self.closes }
        .order_map.iter()
        .filter(|(_, ord)| ord.order_class == OrderClassification::Rebase && ord.can_cancel())
        .min_by(|(_, x), (_, y)| x.size.min(y.size) ) {
            Some((_, order)) => Some(order.size),
            None => None,
        }
    }

    pub fn cancel_distant_rebases(&mut self, top: D128, limit: D128, stage: Stage, sender: Sender<StrategyMessage>) -> FindCancelRes {
        let mut found_rebases = FindCancelRes::NotFound;
        match self.get_best_rebase_price(stage) {
            Some(b) => {
                found_rebases = FindCancelRes::Found;
                if (b - top).abs() > limit {
                    for (_, order) in match stage { Stage::Entry => &mut self.opens, Stage::Exit => &mut self.closes }.order_map.iter_mut()
                    .filter(|(_, ord)| ord.order_class == OrderClassification::Rebase && ord.can_cancel()) {
                        found_rebases = FindCancelRes::Cancelled;

                        Position::cancel_order(self.pool.clone(), order, self.side, stage, self.symbol.clone(), sender.clone());
                    }
                }
                found_rebases
            },
            None => found_rebases,
        }
    }

    pub fn get_best_rebase_price(&self, stage: Stage) -> Option<D128> {
        /*
         * X closer to best than Y is X > Y for buy opens and sell closes, vice versa for vice versa.
         * Selects open or close list based on stage
         * Filters for rebase
         * Then gets max/min price
         */
        match if (self.side == Side::Buy && stage == Stage::Entry) || (self.side == Side::Sell && stage == Stage::Exit) {
            match stage { Stage::Entry => &self.opens, Stage::Exit => &self.closes }.order_map.iter()
            .filter(|(_, ord)| ord.order_class == OrderClassification::Rebase && ord.can_cancel())
            .max_by(|(_, x), (_, y)| x.price.max(y.price))
        } else {
            match stage { Stage::Entry => &self.opens, Stage::Exit => &self.closes }.order_map.iter()
            .filter(|(_, ord)| ord.order_class == OrderClassification::Rebase && ord.can_cancel())
            .min_by(|(_, x), (_, y)| x.price.min(y.price))
        } {
            Some((_, order)) => Some(order.price),
            None => None,
        }
    }

    pub fn data_refresh(&self) -> PositionData {
        let open_liqs = self.opens.all_liqs(false);
        let close_liqs = self.closes.all_liqs(true);
        PositionData {
            open_liqs: open_liqs,
            close_liqs: close_liqs,
            open_position: FinData::from(open_liqs.filled - close_liqs.filled),
            total_count: open_liqs.total_count + close_liqs.total_count,
            remaining_count: self.pos_max_orders - open_liqs.total_count,
            remaining_margin: self.pos_max_size - open_liqs.total_outstanding.inv,
            // active_delta: todo!(),
            // total_delta: todo!(),
        }
    }

    pub fn order_update(
        &mut self,
        order: IncomingOrderWS,
        sender: Sender<StrategyMessage>
    ) {
        match order.stage {
            Stage::Entry => self.opens.ws_order(order.id, order),
            Stage::Exit => {
                
                self.closes.ws_order(order.id, order);
                let pd = self.data_refresh();
                if pd.open_position.inv <= D128::ZERO {
                    let prebate = pd.open_liqs.filled.liq - pd.close_liqs.filled.liq;
                    let rebate = pd.open_liqs.filled.rebate + pd.close_liqs.filled.rebate;
                    let pnl = prebate - rebate;
                    info!("{}side CLOSED OUT: prebate pnl: {}, fee/rebates: {}, pnl: {}\n{}\n\n",
                    self.side, pd, prebate, rebate, pnl);
                    self.opens.clean();
                    self.closes.clean();
                    info!("post clean: {}", self.data_refresh());
                }
            },
        };
    }

    pub fn rest_cancel(&mut self, stage: Stage, id: Uuid, auto_id: Uuid, success: bool) {
        match stage {
            Stage::Entry => self.opens.rest_cancel(id, success),
            Stage::Exit => self.closes.rest_cancel(id, success),
        }
    }

    pub fn order_rest_response(&mut self, id: Uuid, stage: Stage, order: Option<IncomingOrderREST>) {
        match stage {
            Stage::Entry => self.opens.rest_order(id, order),
            Stage::Exit => self.closes.rest_order(id, order),
        }
    }

    pub fn position_update(&mut self, position: IncomingPosition) {
        self.known_size = position.size;
        self.known_price = position.price;
        self.known_liq = position.liq;
        self.known_available_liq = position.position_margin_available;
        self.known_prebate_pnl = position.realised_pnl;
    }

    pub fn new_limit(
        &mut self,
        id: Option<Uuid>, price: D128,
        size: D128,
        stage: Stage,
        class: OrderClassification,
        rem_margin: D128,
        rem_count: D128,
        sender: Sender<StrategyMessage>,
    ) -> bool {
        if stage == Stage::Entry && (size > rem_margin || D128::ONE > rem_count) {
            // debug!("posrej {} rem: {}, count: {}", self.side, rem_margin, rem_count);
            return false;
        }
        let ord = Order::new_rebate(id, price, size, class);
        match stage {
            Stage::Entry => {
                match self.opens.add_order(ord) {
                    Ok(order) => {
                        Position::send_order(self.pool.clone(), order, self.side, Stage::Entry, self.symbol.clone(), sender).unwrap();
                        true
                    },
                    Err(_) => panic!("Dupe order created"),
                }
            }
            Stage::Exit => {
                match self.closes.add_order(ord) {
                    Ok(order) => {
                        Position::send_order(self.pool.clone(), order, self.side, Stage::Exit, self.symbol.clone(), sender).unwrap();
                        true
                    }
                    Err(_) => panic!("Dupe order created"),
                }
            }
        }
    }

    pub fn new_market(
        &mut self,
        id: Option<Uuid>,
        expected_price: D128,
        size: D128,
        stage: Stage,
        class: OrderClassification,
        rem_margin: D128,
        rem_count: D128,
        sender: Sender<StrategyMessage>,
    ) -> bool {
        if stage == Stage::Entry && (size > rem_margin || D128::ONE >= rem_count) { return false; }
        let ord = Order::new_taker(id, expected_price, size, class);
        match stage {
            Stage::Entry => {
                match self.opens.add_order(ord) {
                    Ok(order) => {
                        Position::send_order(self.pool.clone(), order, self.side, Stage::Entry, self.symbol.clone(), sender).unwrap();
                        true
                    },
                    Err(_) => panic!("Dupe order created"),
                }
            }
            Stage::Exit => {
                match self.closes.add_order(ord) {
                    Ok(order) => {
                        Position::send_order(self.pool.clone(), order, self.side, Stage::Exit, self.symbol.clone(), sender).unwrap();
                        true
                    }
                    Err(_) => panic!("Dupe order created"),
                }
            }
        }
    }

    pub fn cancel_order(pool: Handle, order: &mut Order, side: Side, stage: Stage, symbol: String,  sender: Sender<StrategyMessage>) -> Result<(), CancelOrderPositionError>{
        // I don't know why i do this
        order.pre_cancel();
        let sender = sender.clone();
        // TODO Reduce this to not only the fields needed
        let order = order.clone();
        pool.spawn(async move {
            let cancel_result = BROKER.cancel_order(symbol, order.id, order.auto_id).await;
            //if let Ok(cancel) = cancel_result {
                sender.send(StrategyMessage::AccountMessage(
                    AccountMessage::OrderMessage(OrderMessage::CancelResult(CancelResponse {
                        id: order.id,
                        auto_id: order.auto_id,
                        side: side,
                        stage: stage,
                        rest_response: cancel_result.unwrap(),
                    })),
                )).unwrap();
            //}
        });
        
        return Ok(());
    }

    pub fn send_order(pool: Handle, order: &mut Order, side: Side, stage: Stage, symbol: String, sender: Sender<StrategyMessage>) -> Result<(), SendOrderPositionError> {
        let size = (order.size.to_float() * 1000.).round() / 1000.;
        // Fixing side/stage kabookie is more the Position's concern than the Broker's, probably
        // We'll keep it here for now anyway
        let broker_side = match stage {
            Stage::Entry => match side {
                Side::Buy => Side::Buy,
                Side::Sell => Side::Sell,
            },
            Stage::Exit => match side {
                Side::Buy => Side::Sell,
                Side::Sell => Side::Buy,
            },
        };
        let order_type = order.order_type;
        let order_class = order.order_class;
        let id = order.id;
        // info!("side: {:?}, stage: {:?}, size: {}", broker_side, stage, size);
        order.pre_flight();
        match order_type {
            OrderType::Limit => {
                let order = order.clone();
                // let timer = std::time::Instant::now();
                pool.spawn(async move {
                    // println!("thread spawn time: {}", timer.elapsed().as_nanos());
                    let order_result = BROKER.create_limit(
                        id, symbol, order.price,
                        size, broker_side, stage,
                    ).await;
                    // println!("[DEBUG] Limit order sent");
                    let order_result = order_result.unwrap();
                    //if let Ok(order) = order_result {
                    sender.send(StrategyMessage::AccountMessage(
                        AccountMessage::OrderMessage(OrderMessage::OrderResult(
                            OrderResponse::new(id, side, stage, order_result.clone().1, order_class, order_result.0),
                        )),
                    )).unwrap();
                    //}
                });
            }
            OrderType::Market => {
                pool.spawn(async move {

                    let order_result = BROKER.create_market(
                        id,
                        symbol, size,
                        broker_side, stage
                    ).await;
                    //if let Ok(order) = order_result {
                        sender.send(StrategyMessage::AccountMessage(
                            AccountMessage::OrderMessage(OrderMessage::OrderResult(
                                OrderResponse::new(id, side, stage, "TODO".to_string(), order_class, order_result.unwrap()),
                            )),
                        )).unwrap();
                    //}
                });
            }
        }
        return Ok(());
    }
}