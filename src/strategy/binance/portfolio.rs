use std::fmt::{Display, Formatter};

use crossbeam_channel::Sender;
/// Bybit Account --> account interface --> position interface --> position --> orders

use dec::D128;
use tokio::runtime::{Runtime, Builder};
use uuid::Uuid;

use crate::backend::binance::types::{PositionUpdateData, BinanceSide, OrderUpdateData, OrderResponseWrapper, CancelResponseWrapper, AccountBalance, PositionUpdatePosition, PositionUpdateBalance};
use crate::backend::types::Side;
use crate::strategy::types::{Stage, OrderClassification};

use super::order_list::OrderData;
use super::{StrategyMessage, Position, PositionData, FinData, FindCancelRes, Order, OrderResponseContext};

#[derive(Clone, Copy)]
pub struct Limits {
    pub tops: D128,
    pub rebases: D128,
    pub liquidity: D128,
}

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

/// An asset position portfolio manages the buy and sell positions for a given traded pair
/// Each portfolio has a balance of the asset, and a pair of positions
/// representing both sides of the portfolio.
/// We also store the balance and historical positions on the portfolio as well.
/// For each pair, one may maintain separate bearish and bullish positions.
/// Orders against a pair simply change the delta of the corresponding position side.
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
    pub balance: D128,
    pub available_balance: D128,
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
            buy: Position::new(pool.handle().clone(), strat_tx.clone(), symbol.clone(), Side::Buy, D128::ZERO, max / 2),
            sell: Position::new(pool.handle().clone(), strat_tx.clone(), symbol.clone(), Side::Sell, D128::ZERO, max / 2),
            historical: vec![],
            init_size: D128::from(0.001),
            max_open_orders: max,
            rebase_distance_limit: D128::from(10),
            max_size: D128::ZERO,
            balance: D128::ZERO,
            available_balance: D128::ZERO,
            symbol: symbol,
            strat_tx,
            pool,
            data: PortfolioData::new(),
        };
        app.data_refresh();
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
        id: Option<Uuid>,
        price: D128,
        size: D128,
        side: Side,
        stage: Stage,
        class: OrderClassification,
    ) -> bool {
        // self.data_refresh();
        if stage == Stage::Entry && class == OrderClassification::Rebase && (size > (self.data.remaining_margin / price) || D128::ONE > self.data.remaining_count) {
            // debug!("portrej {} rem: {}, count: {}", side, self.data.remaining_margin, self.data.remaining_count);
            return false;
        }
        if size.is_nan() { panic!("size is nan"); }
        else if size.is_zero() { panic!("size is zero, dump: {}\n{:?}\n{:?}", self.data, self.buy, self.sell); }
        // info!("{:?} {:?} order up for {}", side, stage, size);
        match side {
            Side::Buy => {
                let r = self.buy.new_limit(id, price, size, stage, class, self.data.buy.remaining_margin / price, self.data.buy.remaining_count);
                self.data_refresh();
                r
            },
            Side::Sell => {
                let r = self.sell.new_limit(id, price, size, stage, class, self.data.sell.remaining_margin / price, self.data.sell.remaining_count);
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
    ) -> bool {
        // self.data_refresh();
        if stage == Stage::Entry && class == OrderClassification::Rebase && (size > (self.data.remaining_margin / expected_price) || D128::ONE > self.data.remaining_count) { info!("failed portfolio\n{}", self.data); return false; }
        if size.is_nan() { panic!("size is nan, dump: {}\n{:?}\n{:?}", self.data, self.buy, self.sell); }
        else if size.is_zero() { panic!("size is zero, dump: {}\n{:?}\n{:?}", self.data, self.buy, self.sell); }
        match side {
            Side::Buy => {
                let r = self.buy.new_market(id, expected_price, size, stage, class, self.data.buy.remaining_margin / expected_price, self.data.buy.remaining_count);
                self.data_refresh();
                r
            },
            Side::Sell => {
                let r = self.sell.new_market(id, expected_price, size, stage, class, self.data.sell.remaining_margin / expected_price, self.data.sell.remaining_count);
                self.data_refresh();
                r
            },
        }
    }

    pub fn cancel_order(&mut self, id: Uuid, side: Side, stage: Stage) -> bool {
        match side {
            Side::Buy => {
                let r = self.buy.cancel_order(id, stage);
                self.data_refresh();
                r
            },
            Side::Sell => {
                let r = self.sell.cancel_order(id, stage);
                self.data_refresh();
                r
            },
        }
    }

    pub fn order_rest_response(&mut self, id: Uuid, side: Side, stage: Stage, order: OrderResponseWrapper) {
        match side {
            Side::Buy => self.buy.order_rest_response(id, stage, order),
            Side::Sell => self.sell.order_rest_response(id, stage, order),
        };
        self.data_refresh();
    }

    pub fn cancel_response(&mut self, id: Uuid, side: Side, stage: Stage, cancel: CancelResponseWrapper) {
        match cancel {
            CancelResponseWrapper::Cancel(_) => {},
            CancelResponseWrapper::Error(_) => {/*debug!("cancel err\n{}", self.data);*/},
        };
        match side {
            Side::Buy => self.buy.rest_cancel(stage, id, cancel),
            Side::Sell => self.sell.rest_cancel(stage, id, cancel),
        }
        self.data_refresh();
    }

    pub fn order_update(&mut self, order: OrderUpdateData) {
        match order.position_side {
                BinanceSide::Buy => self.buy.order_update(order),
                BinanceSide::Sell => self.sell.order_update(order),
                BinanceSide::Both => todo!(),
            };
        self.data_refresh();
    }

    pub fn position_update(&mut self, position: PositionUpdatePosition) {
        match position.side {
            BinanceSide::Buy => self.buy.position_update(position),
            BinanceSide::Sell => self.sell.position_update(position),
            BinanceSide::Both => todo!(),
        }
        self.data_refresh();
    }

    pub fn balance_update(&mut self, balance: &PositionUpdateBalance) {
        self.balance += balance.balance_change;
        self.available_balance = balance.wallet_balance;
        self.max_size = self.available_balance * 0.8;
        self.buy.balance_update(balance.wallet_balance);
        self.sell.balance_update(balance.wallet_balance);
        self.data_refresh();
    }

    pub fn balance_refresh(&mut self, balance: AccountBalance) {
        self.balance = balance.balance;
        self.available_balance = balance.available_balance;
        self.max_size = balance.balance * 0.8;
        self.buy.balance_refresh(balance.balance);
        self.sell.balance_refresh(balance.balance);
        self.data_refresh();
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

    pub fn cancel_distant_rebases(&mut self, top: D128, side: Side, stage: Stage) -> FindCancelRes {
        let r = match side {
            Side::Buy => self.buy.cancel_distant_rebases(top, self.rebase_distance_limit, stage),
            Side::Sell => self.sell.cancel_distant_rebases(top, self.rebase_distance_limit, stage),
        };
        self.data_refresh();
        r
    }

    /// Returns true if a close was found on the given level
    pub fn cancel_non_tops(&mut self, best: D128, side: Side, stage: Stage) -> FindCancelRes {
        let r = match side {
            Side::Buy => self.buy.cancel_non_tops(best, stage),
            Side::Sell => self.sell.cancel_non_tops(best, stage),
        };
        self.data_refresh();
        r
    }

    pub fn get_smallest_rebase_size(&self, side: Side, stage: Stage) -> Option<D128> {
        match side {
            Side::Buy => self.buy.get_smallest_rebase_size(stage),
            Side::Sell => self.sell.get_smallest_rebase_size(stage),
        }
    }
}