use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Sub;

use dec::D128;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use thiserror::Error;
use uuid::Uuid;

use crate::strategy::types::OrderClassification;

use super::IncomingOrderREST;
use super::IncomingOrderWS;
use super::Order;
use super::OrderProgress;

#[derive(Clone, Copy)]
pub struct OrderData {
    pub inv: D128,
    pub liq: D128,
    pub cb: D128,
    pub prebate_cb: D128,
    pub rebate: D128,
    pub count: D128,
}

impl Display for OrderData {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "OrderData: {{ inv: {}, liq: {}, cb: {}, prebate cb: {}, rebate: {}, count: {} }}",
        self.inv, self.liq, self.cb, self.prebate_cb, self.rebate, self.count)
    }
}

impl Sub for OrderData {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        let mut od = Self::new();
        od.update(self.inv - other.inv, self.liq - other.liq,
            (self.cb - self.prebate_cb) - (other.cb - other.prebate_cb));
        od
    }
}

impl OrderData {
    pub fn new() -> OrderData {
        OrderData {
            inv: D128::ZERO,
            liq: D128::ZERO,
            cb: D128::ZERO,
            prebate_cb: D128::ZERO,
            rebate: D128::ZERO,
            count: D128::ZERO,
        }
    }

    pub fn patch(&mut self, inv: D128, liq: D128, rebate: D128) {
        self.inv += inv;
        self.liq += liq;
        self.rebate += rebate;
        self.count += D128::ONE;
    }

    pub fn process(&mut self) {
        self.prebate_cb = self.liq / self.inv;

        // sweet jesus kill me
/*todo*/ self.cb = (self.liq + self.rebate) / self.inv; // the + here is side-specific and we have no clean way to inject side
        // you will live to see manmade horrors beyond your comprehension
    } // it's over, it's completely over

    pub fn update(&mut self, inv: D128, liq: D128, rebate: D128) {
        self.patch(inv, liq, rebate);
        self.process();
    }

    pub fn neutral_cb(&self, rebate: D128) -> D128 {
        ( self.inv * self.prebate_cb * rebate )
        / ( ( D128::from(2) * self.inv ) - ( self.inv * rebate ) )
    }
}

#[derive(Clone, Copy)]
pub struct AllLiqs {
    pub flight: OrderData,
    pub active: OrderData,
    pub filled: OrderData,
    pub total_reserved: OrderData,
    pub total_outstanding: OrderData,
    pub uncancelled_outstanding: OrderData,
    pub total_count: D128,
}

impl Display for AllLiqs {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "All Liquidities:\n      {{\n        flight: {},\n        active: {},\n        filled: {},\n        total reserved: {},\n        total outstanding: {},\n        uncancelled outstanding: {},\n        total count: {},\n      }}",
        self.flight, self.active, self.filled, self.total_reserved, self.total_outstanding, self.uncancelled_outstanding, self.total_count)
    }
}

impl AllLiqs {
    pub fn new() -> AllLiqs {
        AllLiqs {
            flight: OrderData::new(),
            active: OrderData::new(),
            filled: OrderData::new(),
            total_reserved: OrderData::new(),
            total_outstanding: OrderData::new(),
            uncancelled_outstanding: OrderData::new(),
            total_count: D128::ZERO,
        }
    }

    pub fn neutral_cb(&self, rebate: D128) -> D128 {
        self.total_outstanding.neutral_cb(rebate)
    }

    pub fn uncancelled_neutral_cb(&self, rebate: D128) -> D128 {
        self.uncancelled_outstanding.neutral_cb(rebate)
    }
}

#[derive(Error, Debug, Copy, Clone)]
pub enum OrderListError {
    #[error("Tried to put an order into a fail-state that was already in a fail-state")]
    AlreadyCancelledFilledOrFailedError,
    #[error("Could not find order, generating a placeholder for the attempt")]
    NotFoundMakingPlaceholderError,
    #[error("Tried to make an order out of a pre-existing ID")]
    OrderAlreadyExistsError,
}

#[derive(Debug, Clone)]
pub struct OrderList {
    pub order_map: HashMap<Uuid, Order>,
    pub active_count: usize,
}

impl OrderList {
    pub fn new() -> OrderList {
        OrderList {
            order_map: HashMap::new(),
            active_count: 0,
        }
    }

    pub fn all_liqs(&self, close: bool) -> AllLiqs {
        let mut flight = OrderData::new();
        let mut active = OrderData::new();
        let mut filled = OrderData::new();
        let mut total_reserved = OrderData::new();
        let mut total_outstanding = OrderData::new();
        let mut uncancelled_outstanding = OrderData::new();
        let mut total_count = D128::ZERO;
        for (_id, order) in self.order_map.iter() {
            // if close {info!("all liqs close order: {:?}", order);}
            match order.progress {
                OrderProgress::Init => {
                    total_count += D128::ONE;
                    total_reserved.patch(order.unfilled_size, order.unfilled_liq, order.expected_fee);
                    flight.patch(order.unfilled_size, order.unfilled_liq, order.expected_fee);
                    total_outstanding.patch(order.size, order.unfilled_liq + order.filled_liq, order.expected_fee);
                    if !order.cancel_in_flight {
                        uncancelled_outstanding.patch(order.size, order.unfilled_liq + order.filled_liq, order.expected_fee);
                    }
                },
                OrderProgress::Resting => {
                    total_count += D128::ONE;
                    total_reserved.patch(order.unfilled_size, order.unfilled_liq, order.expected_fee);
                    active.patch(order.unfilled_size, order.unfilled_liq, order.expected_fee);
                    total_outstanding.patch(order.size, order.unfilled_liq + order.filled_liq, order.expected_fee);
                    if !order.cancel_in_flight {
                        uncancelled_outstanding.patch(order.size, order.unfilled_liq + order.filled_liq, order.expected_fee);
                    }
                },
                OrderProgress::PartiallyFilled => {
                    total_count += D128::ONE;
                    active.patch(order.unfilled_size, order.unfilled_liq, order.expected_fee - order.cum_fee);
                    total_reserved.patch(order.unfilled_size, order.unfilled_liq, order.expected_fee - order.cum_fee);
                    filled.patch(order.filled_size, order.filled_liq, order.cum_fee);
                    total_outstanding.patch(order.size, order.unfilled_liq + order.filled_liq, order.cum_fee);
                    if !order.cancel_in_flight {
                        uncancelled_outstanding.patch(order.size, order.unfilled_liq + order.filled_liq, order.cum_fee);
                    }
                },
                OrderProgress::Filled => {
                    total_count += D128::ONE;
                    filled.patch(order.filled_size, order.filled_liq, order.cum_fee);
                    total_outstanding.patch(order.filled_size, order.filled_liq, order.cum_fee);
                    if !order.cancel_in_flight {
                        uncancelled_outstanding.patch(order.filled_size, order.filled_liq, order.cum_fee);
                    }
                },
                OrderProgress::Untracked => {
                }
                _ => {},
            }
        }
        flight.process();
        active.process();
        filled.process();
        total_reserved.process();
        total_outstanding.process();
        uncancelled_outstanding.process();
        // info!("");
        AllLiqs { flight, active, filled, total_reserved, total_outstanding, uncancelled_outstanding, total_count }
    }

    pub fn rest_order(&mut self, id: Uuid, order: Option<IncomingOrderREST>) {
        match order {
            Some(ord) => match self.order_map.entry(id) {
                Occupied(mut occ) => {
                    let occ = occ.get_mut();
                    occ.order_response(ord);
                },
                Vacant(vac) => {
                    let _occ =  vac.insert(Order::from(ord));
                    debug!("REST response's context didn't match to a known order, making orphan");
                },
            },
            None => match self.order_map.entry(id) {
                Occupied(mut occ) => {
                    let occ = occ.get_mut();
                    occ.fail_response();
                },
                Vacant(vac) => {
                    vac.insert(Order::new_orphan(Some(id), None, D128::ZERO));
                    debug!("REST response's context didn't match to a known order, making orphan");
                },
            },
        }
    }

    pub fn rest_cancel(&mut self, id: Uuid, success: bool) {
        match self.order_map.entry(id) {
            Occupied(mut occ) => {
                if success {
                    occ.get_mut().cancel_response();
                } else {
                    occ.get_mut().fail_cancel_response();
                }
            },
            Vacant(vac) => {
                vac.insert(Order::new_orphan(Some(id), None, D128::ZERO));
                info!("REST cancel response's context didn't match to a known order, making orphan");
            },
        }
    }

    pub fn ws_order(&mut self, id: Uuid, order: IncomingOrderWS) {
        match self.order_map.entry(id) {
            Occupied(mut occ) => {
                occ.get_mut().order_update(order);
            },
            Vacant(vac) => {
                vac.insert(Order::from(order));
                panic!("WS update didn't match a known order, making orphan");
            },
        }
    }

    pub fn add_order(&mut self, order: Order) -> Result<&mut Order, OrderListError> {
        match self.order_map.entry(order.id) {
            Occupied(_) => Err(OrderListError::OrderAlreadyExistsError),
            Vacant(vac) => {
                Ok(vac.insert(order))
            },
        }
    }

    pub fn clean(&mut self) {
        self.order_map.retain(|_, ord| ord.progress.incomplete_unfailed());
    }

    pub fn get_top(&self) -> Option<&Order> {
        match self.order_map.iter().find(|(_, order)| order.order_class == OrderClassification::Top && order.can_cancel()) {
            Some((_, ord)) => Some(ord),
            None => None,
        }
    }

    pub fn get_top_data(&self) -> Option<OrderData> {
        match self.get_top() {
            Some(order) => {
                let mut od = OrderData::new();
                od.update(order.size, order.size * order.price, order.expected_fee);
                Some(od)
            },
            None => None,
        }
    }
}
