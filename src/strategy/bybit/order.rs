/// An order status represents the current state of an order
/// including it's type and wether it is "in flight" or not.
/// An order may also be completed

use dec::D128;
use std::fmt::Display;
use std::str::FromStr;
// use std::time::Instant;
use uuid::Uuid;

use crate::backend::bybit::stream::BybitOrderData;
use crate::backend::bybit::broker::{ RestResponse, CancelResult, CreateOrderStatus, OrderType, Side};
use crate::backend::bybit::broker::{OrderResult, OrderStatus};
use crate::backend::types::TimeInForce;
use crate::strategy::types::{Stage, OrderClassification};

use super::{REBATE};

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

impl OrderProgress {
    pub fn can_cancel(&self) -> bool {
        self == &OrderProgress::Resting || self == &OrderProgress::PartiallyFilled || self == &OrderProgress::Init
    }

    pub fn incomplete_unfailed(&self) -> bool {
        self == &OrderProgress::Init || self == &OrderProgress::Resting || self == &OrderProgress::PartiallyFilled
    }
}

#[derive(Debug, Clone)]
pub struct OrderResponse {
    pub id: Uuid,
    pub side: Side,
    pub stage: Stage,
    pub sent: String,
    pub class: OrderClassification,
    pub rest_response: RestResponse<OrderResult>,
}

#[derive(Debug, Clone)]
pub struct CancelResponse {
    pub id: Uuid,
    pub auto_id: Uuid,
    pub side: Side,
    pub stage: Stage,
    pub rest_response: RestResponse<CancelResult>,
}

impl OrderResponse {
    pub fn new(id: Uuid, side: Side, stage: Stage, sent: String, class: OrderClassification, rest_response: RestResponse<OrderResult>) -> OrderResponse {
        OrderResponse {
            id,
            side,
            stage,
            sent,
            class,
            rest_response
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IncomingOrderWS {
    pub id: Uuid,
    pub auto_id: Uuid,
    pub price: D128,
    pub size: D128,
    pub side: Side,
    pub order_type: OrderType,
    pub order_status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub cum_remaining_size: D128,
    pub last_fill_price: D128,
    pub cum_fill_size: D128,
    pub cum_fill_liq: D128,
    pub cum_fill_fee: D128,
    pub unfilled: bool,
    pub stage: Stage,
}

impl From<&BybitOrderData> for IncomingOrderWS {
    fn from(order: &BybitOrderData) -> Self {
        IncomingOrderWS {
            id: match Uuid::from_str(&order.order_link_id) {
                Ok(uuid) => uuid,
                Err(e) => {
                    // eprintln!("{:?}, FRESH UUID GENERATED", e);
                    Uuid::new_v4()
                }
            },
            auto_id: Uuid::from_str(&order.order_id).expect("order uuid"),
            price: D128::from(order.price),
            size: D128::from(order.qty),
            side: Side::try_from(order.side.clone()).expect("incoming order"),
            order_type: OrderType::try_from(order.order_type.clone()).expect("incoming order"),
            time_in_force: TimeInForce::try_from(order.time_in_force.clone())
                .expect("incoming order"),
            cum_remaining_size: D128::from(order.leaves_qty),
            last_fill_price: D128::from(order.last_exec_price),
            cum_fill_size: D128::from(order.cum_exec_qty),
            cum_fill_liq: D128::from(order.cum_exec_value),
            cum_fill_fee: D128::from(order.cum_exec_fee),
            stage: Stage::from(order.reduce_only),
            unfilled: !D128::from(order.leaves_qty).is_zero(),
            order_status: order.order_status,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IncomingOrderREST {
    pub id: Uuid,
    pub auto_id: Uuid,
    pub price: D128,
    pub size: D128,
    pub side: Side,
    pub order_type: OrderType,
    pub order_status: CreateOrderStatus,
    pub time_in_force: TimeInForce,
    pub cum_remaining_size: D128,
    pub last_fill_price: D128,
    pub cum_fill_size: D128,
    pub cum_fill_liq: D128,
    pub cum_fill_fee: D128,
    pub unfilled: bool,
    pub stage: Stage,
}


impl From<OrderResult> for IncomingOrderREST {
    fn from(order: OrderResult) -> Self {
        IncomingOrderREST {
            id: order.order_link_id,
            auto_id: order.order_id,
            price: match order.price {
                Some(p) => p,
                None => D128::NAN,
            },
            size: order.qty,
            side: order.side,
            order_type: order.order_type,
            order_status: order.order_status,
            time_in_force: order.time_in_force,
            cum_remaining_size: order.leaves_qty,
            last_fill_price: order.last_exec_price,
            cum_fill_size: order.cum_exec_qty,
            cum_fill_liq: order.cum_exec_value,
            cum_fill_fee: order.cum_exec_fee,
            unfilled: !order.leaves_qty.is_zero(),
            stage: Stage::from(order.reduce_only),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub id: Uuid,
    pub auto_id: Uuid,
    pub auto_gen: bool,
    pub price: D128,
    pub size: D128,
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
}

impl From<IncomingOrderWS> for Order {
    fn from(incoming: IncomingOrderWS) -> Self {
        let mut order = match incoming.order_type {
            OrderType::Limit => Order::new_orphan(
                Some(incoming.id),
                Some(incoming.price),
                incoming.size,
            ),
            OrderType::Market => Order::new_orphan(
                Some(incoming.id),
                None,
                incoming.size,
            ),
        };
        order.id = incoming.id;
        order.auto_gen = true;
        order.auto_id = incoming.auto_id;
        order.patch_ws(&incoming);
        order.progress = OrderProgress::Untracked;
        debug!("\nA NEW ORDER WAS CREATED FROM INCOMING: {:?}\n", order);
        order
    }
}

/// Creates an orphan, make sure to check if the id exists first
impl From<IncomingOrderREST> for Order {
    fn from(incoming: IncomingOrderREST) -> Self {
        let mut order = match incoming.order_type {
            OrderType::Limit => Order::new_orphan(
                Some(incoming.id),
                Some(incoming.price),
                incoming.size,
            ),
            OrderType::Market => Order::new_orphan(
                Some(incoming.id),
                None,
                incoming.size,
            ),
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

impl Order {
    pub fn can_cancel(&self) -> bool {
        self.progress.can_cancel() && !self.cancel_in_flight
    }

    /// Marks the order as "pre flight", meaning it hasn't been sent yet
    /// More specifically, runs any preflight functionality
    pub fn pre_flight(&mut self) {
        self.in_flight = true;
    }

    pub fn pre_cancel(&mut self) {
        self.cancel_in_flight = true;
    }

    fn patch_rest(&mut self, order: &IncomingOrderREST) {
        self.filled_size = order.cum_fill_size;
        self.filled_liq = order.cum_fill_liq;
        self.unfilled_size = order.cum_remaining_size;
        self.unfilled_liq = order.cum_remaining_size * order.price;
        self.cum_fee = order.cum_fill_fee;
    }

    fn patch_ws(&mut self, order: &IncomingOrderWS) {
        self.filled_size = order.cum_fill_size;
        self.filled_liq = order.cum_fill_liq;
        self.unfilled_size = order.cum_remaining_size;
        self.unfilled_liq = order.cum_remaining_size * order.price;
        self.cum_fee = order.cum_fill_fee;
    }

    pub fn order_update(&mut self, order: IncomingOrderWS) {
        self.in_flight = false;
        match order.order_status {
            OrderStatus::Created => {
                self.progress = OrderProgress::Resting;
                debug!("{:?}", order);
                todo!();
            }
            OrderStatus::Rejected => {
                self.progress = OrderProgress::Failed;
                debug!("{:?}", order);
                // todo!();
            }
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
            OrderStatus::PendingCancel => {
                self.progress = OrderProgress::Cancelled;
                // debug!("{:?}", order);
                // todo!();
            }
        }
    }

    pub fn order_response(&mut self, order: IncomingOrderREST) {
        self.in_flight = false;
        self.auto_id = order.auto_id;
        self.auto_gen = true;
        match self.progress {
            OrderProgress::Init => {
                match order.order_status {
                    CreateOrderStatus::Created => {
                        self.progress = OrderProgress::Resting;
                        self.patch_rest(&order);
                    },
                    CreateOrderStatus::Rejected => {
                        self.progress = OrderProgress::Failed;
                        debug!("order res came back rejected");
                    },
                    CreateOrderStatus::Cancelled => {
                        self.progress = OrderProgress::Cancelled;
                        debug!("order res came back cancelled");
                    },
                    _ => panic!("order res bad status {:?}", order),
                }
            },
            OrderProgress::Resting
            | OrderProgress::PartiallyFilled
            | OrderProgress::Filled => {
                match order.order_status {
                    CreateOrderStatus::Rejected => {
                        panic!("rest failed a progressed ws {:?}", order);
                    },
                    CreateOrderStatus::Cancelled => {
                        panic!("rest cancelled a progressed ws {:?}", order);
                    },
                    _ => {},
                }
             },
            OrderProgress::Cancelled
            | OrderProgress::Failed => {
                match order.order_status {
                    CreateOrderStatus::Created => {
                        // it's okay for REST success to come in after ws already blew the order
                        // still want to keep this branch around for the future
                    },
                    _ => todo!(),
                }
            },
            OrderProgress::Untracked => {  },
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

    pub fn fail_cancel_response(&mut self) {
        self.cancel_in_flight = false;
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
            auto_id: Uuid::nil(),
            auto_gen: false,
            price: expected_price,
            size,
            expected_fee: expected_price * size * D128::from(0.00075),
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
        }
    }

    pub fn new_rebate(
        id: Option<Uuid>,
        price: D128,
        size: D128,
        class: OrderClassification,
    ) -> Order {
        let mut ord = Order::new_taker(id, price, size, class);
        ord.expected_fee = price * size * *REBATE * D128::from(-1);
        ord.time_in_force = TimeInForce::PostOnly;
        ord.order_type = OrderType::Limit;
        ord.unfilled_liq = ord.price * ord.size;
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