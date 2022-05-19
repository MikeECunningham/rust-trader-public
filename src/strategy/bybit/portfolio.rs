use std::fmt::{Display, Formatter};

use crossbeam_channel::Sender;
/// Bybit Account --> account interface --> position interface --> position --> orders

use dec::D128;
use tokio::runtime::{Runtime, Builder};
use uuid::Uuid;

use crate::backend::bybit::broker::Side;
use crate::strategy::types::{Stage, OrderClassification};

use super::order_list::OrderData;
use super::{StrategyMessage, Position, IncomingOrderREST, IncomingOrderWS, IncomingPosition, PositionData, FinData, FindCancelRes, Order};

#[derive(Clone, Copy)]
pub struct PortfolioData {
    pub buy: PositionData,
    pub sell: PositionData,
    pub remaining_margin: D128,
    pub remaining_count: D128,
    // Someday lmao
    // pub delta: PositionData,
}

impl Display for PortfolioData {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Portfolio Data: \n{{\n    buys: {},\n    sells: {},\n    remaining margin: {}, remaining count: {}\n}}",
        self.buy, self.sell, self.remaining_margin, self.remaining_count)
    }
}

impl PortfolioData {
    pub fn new() -> PortfolioData {
        PortfolioData {
            buy: PositionData::new(),
            sell: PositionData::new(),
            remaining_margin: D128::ZERO,
            remaining_count: D128::ZERO,
        }
    }
}
// #[derive(Debug)]
pub struct Portfolio {
    pub buy: Position,
    pub sell: Position,
    pub historical: Vec<Position>,
    pub symbol: String,
    pub data: PortfolioData,
    /// Initial size for SOME orders
    pub init_size: D128,
    /// Maximum number of open orders
    pub max_open_orders: D128,
    ///
    pub max_size: D128,
    ///
    pub rebase_distance_limit: D128,
    /// Channel for sending updates back to the main strategy
    pub strat_tx: Sender<StrategyMessage>,
    pool: Runtime
}

impl Portfolio {
    pub fn new(strat_tx: Sender<StrategyMessage>, symbol: String) -> tokio::io::Result<Portfolio> {
        let pool = Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("asset_position_portfolio_pool")
            .enable_io()
            .enable_time()
            .build()?;
        let max = D128::from(8);
        let mut app = Portfolio {
            buy: Position::new(pool.handle().clone(), symbol.clone(), Side::Buy, D128::from(1.3), max / 2),
            sell: Position::new(pool.handle().clone(), symbol.clone(), Side::Sell, D128::from(1.3), max / 2),
            historical: vec![],
            init_size: D128::from(0.001),
            max_open_orders: max,
            rebase_distance_limit: D128::from(10),
            max_size: D128::from(1.3),
            symbol: symbol,
            strat_tx,
            pool,
            data: PortfolioData::new(),
        };
        app.data_generate();
        Ok(app)
    }

    fn data_generate(&self) -> PortfolioData {
        let buy = self.buy.data_refresh();
        let sell = self.sell.data_refresh();
        let pd = PortfolioData {
            buy: buy,
            sell: sell,
            remaining_margin: self.max_size - (buy.open_liqs.total_reserved.inv + sell.open_liqs.total_reserved.inv),
            remaining_count: self.max_open_orders - (buy.open_liqs.total_reserved.count + sell.open_liqs.total_reserved.count),
        };
        // info!("pd {}", pd);
        pd
    }

    pub fn data_refresh(&mut self) {
        self.data = self.data_generate();
        // info!("data: {}", self.data);
    }

    pub fn new_limit(
        &mut self,
        id: Option<Uuid>, price: D128,
        size: D128,
        side: Side,
        stage: Stage,
        class: OrderClassification,
        sender: Sender<StrategyMessage>,
    ) -> bool {
        // self.data_refresh();
        if stage == Stage::Entry && (size > self.data.remaining_margin || D128::ONE > self.data.remaining_count) {
            // debug!("portrej {} rem: {}, count: {}", side, self.data.remaining_margin, self.data.remaining_count);
            return false;
        }
        if size.is_nan() { panic!("size is nan"); }
        // info!("{:?} {:?} order up for {}", side, stage, size);
        match side {
            Side::Buy => {
                let r = self.buy.new_limit(id, price, size, stage, class, self.data.buy.remaining_margin, self.data.buy.remaining_count, sender);
                self.data_refresh();
                r
            },
            Side::Sell => {
                let r = self.sell.new_limit(id, price, size, stage, class, self.data.sell.remaining_margin, self.data.sell.remaining_count, sender);
                self.data_refresh();
                r
            },
        }
    }

    pub fn new_market(
        &mut self,
        id: Option<Uuid>,
        expected_price: D128,
        size: D128,
        side: Side,
        stage: Stage,
        class: OrderClassification,
        sender: Sender<StrategyMessage>,
    ) -> bool {
        // self.data_refresh();
        if stage == Stage::Entry && (size > self.data.remaining_margin || D128::ONE > self.data.remaining_count) { return false; }
        if size.is_nan() { panic!("size is nan, dump: {}\n{:?}\n{:?}", self.data, self.buy, self.sell); }
        else if size.is_zero() { panic!("size is zero, dump: {}\n{:?}\n{:?}", self.data, self.buy, self.sell); }
        match side {
            Side::Buy => {
                let r = self.buy.new_market(id, expected_price, size, stage, class, self.data.buy.remaining_margin, self.data.buy.remaining_count, sender);
                self.data_refresh();
                r
            },
            Side::Sell => {
                let r = self.sell.new_market(id, expected_price, size, stage, class, self.data.sell.remaining_margin, self.data.sell.remaining_count, sender);
                self.data_refresh();
                r
            },
        }
    }

    pub fn order_rest_response(&mut self, id: Uuid, side: Side, stage: Stage, order: Option<IncomingOrderREST>) {
        match side {
            Side::Buy => self.buy.order_rest_response(id, stage, order),
            Side::Sell => self.sell.order_rest_response(id, stage, order),
        };
    }

    pub fn cancel_response(&mut self, id: Uuid, auto_id: Uuid, side: Side, stage: Stage, success: bool) {
        match side {
            Side::Buy => self.buy.rest_cancel(stage, id, auto_id, success),
            Side::Sell => self.sell.rest_cancel(stage, id, auto_id, success),
        }
        // match stage {
        //     Stage::Entry => match side {
        //         Side::Buy => self.buy.rest_cancel(stage, id, auto_id, success),
        //         Side::Sell => self.sell.rest_cancel(stage, id, auto_id, success),
        //     },
        //     Stage::Exit => match side {
        //         Side::Buy => self.sell.rest_cancel(stage, id, auto_id, success),
        //         Side::Sell => self.buy.rest_cancel(stage, id, auto_id, success),
        //     },
        // }
    }

    pub fn position_update(&mut self, position: IncomingPosition) {
        match position.side {
            Side::Buy => self.buy.position_update(position),
            Side::Sell => self.sell.position_update(position),
        }
    }

    pub fn order_update(&mut self, order: IncomingOrderWS, sender: Sender<StrategyMessage>) {
        // info!("orderws {:?}", order);
        match order.stage {
            Stage::Entry => match order.side {
                Side::Buy => self.buy.order_update(order, sender),
                Side::Sell => self.sell.order_update(order, sender),
            },
            Stage::Exit => match order.side {
                Side::Buy => { self.sell.order_update(order, sender); },
                Side::Sell => { self.buy.order_update(order, sender); },
            },
        };
    }

    pub fn get_top(&self, side: Side, stage: Stage) -> Option<&Order> {
        match side {
            Side::Buy => self.buy.get_top(stage),
            Side::Sell => self.sell.get_top(stage),
        }
    }

    pub fn get_top_data(&self, side: Side, stage: Stage) -> Option<OrderData> {
        match side {
            Side::Buy => self.buy.get_top_data(stage),
            Side::Sell => self.sell.get_top_data(stage),
        }
    }

    pub fn cancel_distant_rebases(&mut self, top: D128, side: Side, stage: Stage, sender: Sender<StrategyMessage>) -> FindCancelRes {
        let r = match side {
            Side::Buy => self.buy.cancel_distant_rebases(top, self.rebase_distance_limit, stage, sender.clone()),
            Side::Sell => self.sell.cancel_distant_rebases(top, self.rebase_distance_limit, stage, sender.clone()),
        };
        self.data_refresh();
        r
    }

    /// Returns true if a close was found on the given level
    pub fn cancel_non_tops(&mut self, best: D128, side: Side, stage: Stage, sender: Sender<StrategyMessage>) -> FindCancelRes {
        // self.data_refresh();
        let r = match side {
            Side::Buy => self.buy.cancel_non_tops(best, stage, sender.clone()),
            Side::Sell => self.sell.cancel_non_tops(best, stage, sender.clone()),
        };
        self.data_refresh();
        r
    }

    pub fn get_smallest_rebase_size(&self, side: Side, stage: Stage) -> Option<D128> {
        // self.data_refresh();
        match side {
            Side::Buy => self.buy.get_smallest_rebase_size(stage),
            Side::Sell => self.sell.get_smallest_rebase_size(stage),
        }
    }
}