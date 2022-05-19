use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crossbeam_channel::{SendError, Sender};
use dec::{D128, Context};
use tokio::runtime::Handle;
use uuid::Uuid;
use thiserror::Error;

use crate::backend::binance::broker::BROKER;
use crate::backend::binance::types::{OrderType, PositionUpdateData, PositionUpdatePosition, PositionUpdateBalance, OrderUpdateData, CancelResponse, OrderResponseWrapper, CancelResponseWrapper};
use crate::backend::types::Side;
use crate::strategy::types::{Stage, OrderClassification};

use super::order_list::{OrderList, OrderListError, AllLiqs, OrderData};
use super::{StrategyMessage, Order, AccountMessage, OrderResponseContext, CancelResponseContext, message, OrderProgress};


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
    pub known_prebate_pnl: D128,
    pub known_prebate_unrealized: D128,
    pub sequence: D128,
    pub pool: Handle,
    pub strat_tx: Sender<StrategyMessage>,
}

impl Position {
    pub fn new(pool: Handle, sender: Sender<StrategyMessage>, symbol: String, open_side: Side, max_size: D128, max_count: D128) -> Position {
        Position {
            symbol,
            side: open_side,
            opens: OrderList::new(),
            closes: OrderList::new(),
            sequence: D128::ZERO,
            pos_max_size: max_size,
            pos_max_orders: max_count,
            known_prebate_pnl: D128::ZERO,
            known_prebate_unrealized: D128::ZERO,
            known_size: D128::ZERO,
            known_price: D128::ZERO,
            known_liq: D128::ZERO,
            pool,
            strat_tx: sender,
        }
    }

    pub fn get_top(&self, stage: Stage) -> Option<&Order> {
        match stage { Stage::Entry => self.opens.get_top(), Stage::Exit => self.closes.get_top(), }
    }

    pub fn get_top_data(&self, stage: Stage) -> Option<OrderData> {
        match stage { Stage::Entry => self.opens.get_top_data(), Stage::Exit => self.closes.get_top_data(), }
    }

    pub fn cancel_non_tops(&mut self, best: D128, stage: Stage) -> FindCancelRes {
        let mut found_top = FindCancelRes::NotFound;

        for (_, order)
        in stage.aggress_mut(&mut self.opens, &mut self.closes)
        .order_map.iter_mut()
        .filter(|(_, ord)| ord.order_class == OrderClassification::Top) {
            if found_top == FindCancelRes::NotFound { found_top = FindCancelRes::Found }
            if order.can_cancel() {
                if *self.side.deside(
                    stage.aggress(&(order.orig_price < best), &(order.orig_price > best)),
                    stage.aggress(&(order.orig_price > best), &(order.orig_price < best))
                ) {
                    found_top = FindCancelRes::Cancelled;
                    // info!("Cancelling a non-top {:?} at {}, best is {}", stage, order.price, best);
                    Position::send_cancel(self.pool.clone(), order, self.side, stage, self.symbol.clone(), self.strat_tx.clone());
                }
            }
        }
        found_top
    }

    pub fn get_smallest_rebase_size(&self, stage: Stage) -> Option<D128> {
        match stage.aggress(&self.opens, &self.closes)
        .order_map.iter()
        .filter(|(_, ord)| ord.order_class == OrderClassification::Rebase && ord.can_cancel())
        .min_by(|(_, x), (_, y)| x.orig_size.min(y.orig_size) ) {
            Some((_, order)) => Some(order.orig_size),
            None => None,
        }
    }

    pub fn cancel_distant_rebases(&mut self, top: D128, limit: D128, stage: Stage) -> FindCancelRes {
        let mut found_rebases = FindCancelRes::NotFound;
        match self.get_best_rebase_price(stage) {
            Some(b) => {
                found_rebases = FindCancelRes::Found;
                if (b - top).abs() > limit {
                    for (_, order) in stage.aggress_mut(&mut self.opens, &mut self.closes).order_map.iter_mut() {
                        if order.order_class == OrderClassification::Rebase && order.can_cancel() {
                            found_rebases = FindCancelRes::Cancelled;
                            Position::send_cancel(self.pool.clone(), order, self.side, stage, self.symbol.clone(), self.strat_tx.clone());
                        }
                    }
                }
                found_rebases
            },
            None => found_rebases,
        }
    }

    pub fn cancel_order(&mut self, id: Uuid, stage: Stage) -> bool {
        match stage {
            Stage::Entry => match self.opens.order_map.get_mut(&id) {
                Some(order) => {
                    Position::send_cancel(self.pool.clone(), order, self.side, stage, self.symbol.clone(), self.strat_tx.clone());
                    true
                },
                None => false,
            },
            Stage::Exit => match self.closes.order_map.get_mut(&id) {
                Some(order) => {
                    Position::send_cancel(self.pool.clone(), order, self.side, stage, self.symbol.clone(), self.strat_tx.clone());
                    true
                },
                None => false,
            },
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
            stage.aggress(&self.opens, &self.closes).order_map.iter()
            .filter(|(_, ord)| ord.order_class == OrderClassification::Rebase && ord.can_cancel())
            .max_by(|(_, x), (_, y)| x.orig_price.max(y.orig_price))
        } else {
            stage.aggress(&self.opens, &self.closes).order_map.iter()
            .filter(|(_, ord)| ord.order_class == OrderClassification::Rebase && ord.can_cancel())
            .min_by(|(_, x), (_, y)| x.orig_price.min(y.orig_price))
        } {
            Some((_, order)) => Some(order.orig_price),
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
            remaining_count: self.pos_max_orders - open_liqs.total_reserved.count,
            remaining_margin: self.pos_max_size - open_liqs.total_outstanding.inv,
            // active_delta: todo!(),
            // total_delta: todo!(),
        }
    }

    pub fn order_update(
        &mut self,
        order: OrderUpdateData
    ) {
        match Stage::from_binance_side(order.side, order.position_side) {
            Stage::Entry => self.opens.ws_order(order),
            Stage::Exit => {
                self.closes.ws_order(order);
                let pd = self.data_refresh();
                if pd.open_position.inv <= D128::ZERO {
                    let prebate = pd.open_liqs.filled.liq - pd.close_liqs.filled.liq;
                    let rebate = pd.open_liqs.filled.rebate + pd.close_liqs.filled.rebate;
                    let pnl = prebate - rebate;
                    // info!("{}side CLOSED OUT: prebate pnl: {}, fee/rebates: {}, pnl: {}\n",
                    // self.side, prebate, rebate, pnl);
                    self.opens.clean();
                    self.closes.clean();
                    // info!("post clean: {}", self.data_refresh());
                }
            },
        };
    }

    pub fn rest_cancel(&mut self, stage: Stage, id: Uuid, cancel: CancelResponseWrapper) {
        match stage {
            Stage::Entry => self.opens.rest_cancel(id, cancel),
            Stage::Exit => self.closes.rest_cancel(id, cancel),
        }
    }

    pub fn order_rest_response(&mut self, id: Uuid, stage: Stage, order: OrderResponseWrapper) {
        match stage {
            Stage::Entry => self.opens.rest_order(id, order),
            Stage::Exit => self.closes.rest_order(id, order),
        }
    }

    pub fn position_update(&mut self, position: PositionUpdatePosition) {
        self.known_size = position.quantity;
        self.known_price = position.price;
        self.known_liq = position.quantity * position.price;
        self.known_prebate_unrealized = position.unrealized_pnl;
        self.known_prebate_pnl = position.accumulated_realized;
    }

    pub fn balance_update(&mut self, balance: D128) {
        self.pos_max_size = balance * D128::from(0.8) / D128::from(2);
    }

    pub fn balance_refresh(&mut self, balance: D128) {
        self.pos_max_size = balance * D128::from(0.8) / D128::from(2);
    }

    pub fn new_limit(
        &mut self,
        id: Option<Uuid>, price: D128,
        size: D128,
        stage: Stage,
        class: OrderClassification,
        rem_margin: D128,
        rem_count: D128,
    ) -> bool {
        if stage == Stage::Entry && class == OrderClassification::Rebase && (size > rem_margin || D128::ONE > rem_count) {
            // debug!("posrej {} rem: {}, count: {}", self.side, rem_margin, rem_count);
            return false;
        }
        let ord = Order::new_rebate(id, price, size, class);
        match stage {
            Stage::Entry => {
                match self.opens.add_order(ord) {
                    Ok(order) => {
                        Position::send_order(self.pool.clone(), order, self.side, Stage::Entry, self.symbol.clone(), self.strat_tx.clone());
                        true
                    },
                    Err(_) => panic!("Dupe order created"),
                }
            }
            Stage::Exit => {
                match self.closes.add_order(ord) {
                    Ok(order) => {
                        Position::send_order(self.pool.clone(), order, self.side, Stage::Exit, self.symbol.clone(), self.strat_tx.clone());
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
    ) -> bool {
        if stage == Stage::Entry && class == OrderClassification::Rebase && (size > rem_margin || D128::ONE >= rem_count) { return false; }
        let ord = Order::new_taker(id, expected_price, size, class);
        match stage {
            Stage::Entry => {
                match self.opens.add_order(ord) {
                    Ok(order) => {
                        Position::send_order(self.pool.clone(), order, self.side, Stage::Entry, self.symbol.clone(), self.strat_tx.clone());
                        true
                    },
                    Err(_) => panic!("Dupe order created"),
                }
            }
            Stage::Exit => {
                match self.closes.add_order(ord) {
                    Ok(order) => {
                        Position::send_order(self.pool.clone(), order, self.side, Stage::Exit, self.symbol.clone(), self.strat_tx.clone());
                        true
                    }
                    Err(_) => panic!("Dupe order created"),
                }
            }
        }
    }

    pub fn send_cancel(pool: Handle, order: &mut Order, side: Side, stage: Stage, symbol: String,  sender: Sender<StrategyMessage>) {
        // I don't know why i do this
        order.pre_cancel();
        let sender = sender.clone();
        // TODO Reduce this to not only the fields needed
        let order = order.clone();
        pool.spawn(async move {
            let cancel_result = BROKER.cancel_order(order.id, symbol).await;
            sender.send(StrategyMessage::AccountMessage(
                AccountMessage::CancelResponse(CancelResponseContext::new(order.id, side, stage, order.order_class, cancel_result)),
            )).unwrap();
        });
    }

    pub fn send_order(pool: Handle, order: &mut Order, side: Side, stage: Stage, symbol: String, sender: Sender<StrategyMessage>) {
        let size = (order.orig_size.to_float() * 1000.).round() / 1000.;
        let price = (order.orig_price.to_float() * 100.).round() / 100.;
        // let size = order.orig_size;
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
                        id, symbol, price,
                        size, broker_side, stage,
                    ).await;
                    // println!("[DEBUG] Limit order sent");
                    let order_result = order_result;
                    //if let Ok(order) = order_result {
                    sender.send(StrategyMessage::AccountMessage(
                        AccountMessage::OrderResponse(
                            OrderResponseContext::new(id, side, stage, order_class, order_result),
                        ),
                    )).unwrap();
                    //}
                });
            }
            OrderType::Market => {
                info!("position market order sender");
                pool.spawn(async move {

                    let order_result = BROKER.create_market(
                        id,
                        symbol, size,
                        broker_side, stage
                    ).await;
                    //if let Ok(order) = order_result {
                        sender.send(StrategyMessage::AccountMessage(
                            AccountMessage::OrderResponse(
                                OrderResponseContext::new(id, side, stage, order_class, order_result),
                            ),
                        )).unwrap();
                    //}
                });
            }
            _ => todo!(),
        }
    }
}