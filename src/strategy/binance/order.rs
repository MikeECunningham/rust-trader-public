/// An order status represents the current state of an order
/// including its type and wether it is "in flight" or not.
/// An order may also be completed

use dec::D128;
use std::fmt::Display;
use std::str::FromStr;
// use std::time::Instant;
use uuid::Uuid;

use crate::backend::binance::types::{OrderResponse, OrderType, OrderStatus, CreateOrderStatus, OrderResponseWrapper, OrderUpdateData, CancelResponseWrapper, CancelResponse, BinanceError};
use crate::backend::types::{TimeInForce, Side};
use crate::strategy::types::{Stage, OrderClassification};

use super::REBATE;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OrderProgress {
    Init,
    Resting,
    PartiallyFilled,
    Filled,
    Cancelled,
    Failed,
    Untracked,
}

#[derive(Debug, Clone)]
pub enum OrderId {
    String(String),
    Uuid(Uuid),
}

impl OrderProgress {
    pub fn can_cancel(&self) -> bool {
        self == &OrderProgress::Resting || self == &OrderProgress::PartiallyFilled || self == &OrderProgress::Init
    }

    pub fn incomplete_unfailed(&self) -> bool {
        self == &OrderProgress::Init || self == &OrderProgress::Resting || self == &OrderProgress::PartiallyFilled
    }
}

#[derive(Debug, Clone)]
pub struct OrderResponseContext {
    pub id: Uuid,
    pub side: Side,
    pub stage: Stage,
    pub class: OrderClassification,
    pub rest_response: OrderResponseWrapper,
}

impl OrderResponseContext {
    pub fn new(id: Uuid, side: Side, stage: Stage, class: OrderClassification, rest_response: OrderResponseWrapper) -> OrderResponseContext {
        OrderResponseContext {
            id,
            side,
            stage,
            class,
            rest_response
        }
    }
}

#[derive(Debug, Clone)]
pub struct CancelResponseContext {
    pub id: Uuid,
    pub side: Side,
    pub stage: Stage,
    pub class: OrderClassification,
    pub rest_response: CancelResponseWrapper,
}

impl CancelResponseContext {
    pub fn new(id: Uuid, side: Side, stage: Stage, class: OrderClassification, rest_response: CancelResponseWrapper) -> CancelResponseContext {
        CancelResponseContext {
            id,
            side,
            stage,
            class,
            rest_response,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub id: Uuid,
    pub auto_id: u64,
    pub auto_gen: bool,
    pub filled_price: D128,
    pub orig_price: D128,
    pub orig_size: D128,
    pub in_flight: bool,
    pub cancel_in_flight: bool,
    pub unfilled_size: D128,
    pub unfilled_liq: D128,
    pub filled_size: D128,
    pub filled_liq: D128,
    pub cum_fee: D128,
    pub expected_fee: D128,
    pub time_in_force: TimeInForce,
    pub order_type: OrderType,
    pub order_class: OrderClassification,
    pub progress: OrderProgress,
    pub unknown_cancel_counter: usize,
}

impl From<OrderUpdateData> for Order {
    fn from(incoming: OrderUpdateData) -> Self {
        let mut order = match incoming.order_type {
            OrderType::Limit => Order::new_orphan(
                Some(Uuid::from_u128(incoming.auto_id as u128)),
                Some(incoming.original_price),
                incoming.original_qty,
            ),
            OrderType::Market => Order::new_orphan(
                Some(Uuid::from_u128(incoming.auto_id as u128)),
                Some(incoming.original_price),
                incoming.original_qty,
            ),
            _ => todo!(),
        };
        order.id = Uuid::from_u128(incoming.auto_id as u128);
        order.auto_gen = true;
        // order.auto_id = incoming.auto_id; DON'T FORGET TO RE-ADD THIS
        order.patch_ws(&incoming);
        order.progress = OrderProgress::Untracked;
        debug!("\nA NEW ORDER WAS CREATED FROM INCOMING: {:?}\n", order);
        order
    }
}

/// Creates an orphan, make sure to check if the id exists first
impl From<OrderResponse> for Order {
    fn from(incoming: OrderResponse) -> Self {
        let mut order = match incoming.order_type {
            OrderType::Limit => Order::new_orphan(
                Some(incoming.id),
                Some(incoming.price),
                incoming.orig_qty,
            ),
            OrderType::Market => Order::new_orphan(
                Some(incoming.id),
                None,
                incoming.orig_qty,
            ),
            _ => todo!(),
        };
        order.id = incoming.id;
        order.auto_id = incoming.auto_id;
        order.auto_gen = true;
        order.patch_rest(&incoming);
        order.progress = OrderProgress::Untracked;
        debug!("\nA NEW ORDER WAS CREATED FROM INCOMING: {:?}\nfrom {:?}\n", order, incoming);
        order
    }
}

impl From<CancelResponse> for Order {
    fn from(incoming: CancelResponse) -> Self {
        let mut order = match incoming.order_type {
            OrderType::Market => Order::new_orphan(Some(incoming.id), None, incoming.orig_qty),
            OrderType::Limit => Order::new_orphan(Some(incoming.id), Some(incoming.price), incoming.orig_qty),
            _ => todo!(),
        };
        order.id = incoming.id;
        order.auto_id = incoming.auto_id;
        order.auto_gen = true;
        order.filled_size = incoming.cum_qty;
        order.filled_liq = incoming.cum_qty * incoming.cum_quote;
        order.unfilled_size = incoming.orig_qty - order.filled_size;
        order.unfilled_liq = (incoming.orig_qty * incoming.price) - order.filled_liq;
        order.progress = OrderProgress::Untracked;
        debug!("\nA NEW ORDER WAS CREATED FROM INCOMING CANCEL: {:?}\nfrom {:?}\n", order, incoming);
        order
    }
}

impl Order {
    pub fn can_cancel(&self) -> bool {
        self.progress.can_cancel() && !self.cancel_in_flight && !self.in_flight
    }

    /// Marks the order as "pre flight", meaning it hasn't been sent yet
    /// More specifically, runs any preflight functionality
    pub fn pre_flight(&mut self) {
        self.in_flight = true;
    }

    pub fn pre_cancel(&mut self) {
        self.cancel_in_flight = true;
    }

    fn patch_rest(&mut self, order: &OrderResponse) {
        self.filled_size = order.cum_qty;
        self.filled_liq = order.cum_qty * order.cum_quote;
        self.unfilled_size = order.orig_qty - self.filled_size;
        self.unfilled_liq = (order.orig_qty * order.price) - self.filled_liq;
    }

    fn patch_ws(&mut self, order: &OrderUpdateData) {
        self.filled_size = order.accumulated_filled_qty;
        self.filled_liq = order.accumulated_filled_qty * order.filled_price;
        self.unfilled_size = order.original_qty - order.accumulated_filled_qty;
        self.unfilled_liq = self.unfilled_size * order.original_price;
        self.cum_fee = match order.commission { Some(c) => c, None => D128::ZERO, };
    }

    pub fn order_update(&mut self, order: OrderUpdateData) {
        self.in_flight = false;
        match order.order_status {
            OrderStatus::New => {
                match self.progress {
                    OrderProgress::Init | OrderProgress::Resting => {
                        self.progress = OrderProgress::Resting;
                    },
                    _ => {},
                }
                self.patch_ws(&order);

            }
            OrderStatus::PartiallyFilled => {
                match self.progress {
                    OrderProgress::Init | OrderProgress::Resting | OrderProgress::PartiallyFilled => {
                        self.progress = OrderProgress::PartiallyFilled;
                    },
                    _ => {},
                }
                self.patch_ws(&order);
            }
            OrderStatus::Filled => {
                match self.progress {
                    OrderProgress::Init | OrderProgress::Resting | OrderProgress::PartiallyFilled | OrderProgress::Filled => {
                        self.progress = OrderProgress::Filled;
                    },
                    _ => {},
                }
                self.patch_ws(&order);
            }
            OrderStatus::Cancelled => {
                self.progress = OrderProgress::Cancelled;
                // debug!("{:?}", order);
                // todo!();
            }
            OrderStatus::Expired => {
                // This generally means a post only limit order bounced off an invalid level
                self.progress = OrderProgress::Cancelled;
            },
            OrderStatus::NewInsurance => todo!(),
            OrderStatus::NewADL => todo!(),
        }
    }

    pub fn order_response(&mut self, order: OrderResponse) {
        self.in_flight = false;
        self.auto_id = order.auto_id;
        self.auto_gen = true;
        match self.progress {
            OrderProgress::Init => {
                match order.status {
                    OrderStatus::New => {
                        self.progress = OrderProgress::Resting;
                        self.patch_rest(&order);
                    },
                    OrderStatus::Cancelled | OrderStatus::Expired => {
                        self.progress = OrderProgress::Cancelled;
                        debug!("order res came back cancelled");
                    },
                    _ => panic!("order res bad status {:?}", order),
                }
            },
            OrderProgress::Resting
            | OrderProgress::PartiallyFilled
            | OrderProgress::Filled => {
                match order.status {
                    OrderStatus::Cancelled | OrderStatus::Expired => {
                        panic!("rest cancelled a progressed ws {:?}", order);
                    },
                    _ => {},
                }
             },
            OrderProgress::Cancelled
            | OrderProgress::Failed => {
                match order.status {
                    OrderStatus::New | OrderStatus::Cancelled | OrderStatus::Expired => {
                        // it's okay for REST success/fail to come in after ws already blew the order
                        // still want to keep this branch around for the future
                    },
                    _ => todo!(), // Anything else is just inexplicable
                }
            },
            OrderProgress::Untracked => { /* Doesn't matter what untrackeds are up to */ },
        }
    }

    // Just broken out to remind me to handle later
    // A lot of these should be panics
    pub fn fail_response(&mut self) {
        self.in_flight = false;
        match self.progress {
            OrderProgress::Init => {
                self.progress = OrderProgress::Failed;
            },
            OrderProgress::Resting
            | OrderProgress::PartiallyFilled
            | OrderProgress::Filled => {
                self.progress = OrderProgress::Failed;
            },
            OrderProgress::Cancelled
            | OrderProgress::Failed => {
                self.progress = OrderProgress::Failed;
            },
            OrderProgress::Untracked => {
                self.progress = OrderProgress::Failed;
            },
        }
    }

    pub fn cancel_response(&mut self) {
        self.cancel_in_flight = false;
        self.progress = OrderProgress::Cancelled;
    }

    pub fn fail_cancel_response(&mut self, error: BinanceError) {
        self.cancel_in_flight = false;
        if error.msg.contains("Unknown order sent") {
            self.unknown_cancel_counter += 1;
            debug!("unknown cancel");
            if self.unknown_cancel_counter > 3 {
                panic!("possible desync: too many failed cancels, crashing.");
                // self.progress = OrderProgress::Failed
            }
        }
    }

    pub fn new_taker(
        id: Option<Uuid>,
        expected_price: D128,
        size: D128,
        class: OrderClassification,
    ) -> Order {
        Order {
            id: match id {
                Some(id) => id,
                None => Uuid::new_v4(),
            },
            auto_id: u64::default(),
            auto_gen: false,
            filled_price: D128::ZERO,
            orig_price: expected_price,
            orig_size: size,
            expected_fee: expected_price * size * 0.00075,
            in_flight: false,
            cancel_in_flight: false,
            unfilled_size: size,
            filled_size: D128::ZERO,
            time_in_force: TimeInForce::GoodTillCancel,
            order_type: OrderType::Market,
            unfilled_liq: D128::ZERO,
            filled_liq: D128::ZERO,
            cum_fee: D128::ZERO,
            progress: OrderProgress::Init,
            order_class: class,
            unknown_cancel_counter: 0,
        }
    }

    pub fn new_rebate(
        id: Option<Uuid>,
        price: D128,
        size: D128,
        class: OrderClassification,
    ) -> Order {
        let mut ord = Order::new_taker(id, price, size, class);
        ord.expected_fee = price * size * *REBATE * -1;
        ord.time_in_force = TimeInForce::PostOnly;
        ord.order_type = OrderType::Limit;
        ord.unfilled_liq = ord.orig_price * ord.orig_size;
        ord
    }

    /// Holds unassociated orders
    pub fn new_orphan(
        id: Option<Uuid>,
        price: Option<D128>,
        size: D128,
    ) -> Order {
        let mut ord = match price {
            Some(price) => Order::new_rebate(id, price, size, OrderClassification::None),
            None => Order::new_taker(id, D128::NAN, size, OrderClassification::None),
        };
        ord.progress = OrderProgress::Untracked;
        ord
    }
}