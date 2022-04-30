use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use dec::D128;
use tokio::runtime::Handle;
use uuid::Uuid;
use std::time::Instant;


use crate::analysis::BookResult;
use crate::analysis::TradeResult;
use crate::backend::binance::broker::BROKER;
use crate::backend::binance::types::OrderResponse;
use crate::backend::binance::types::OrderResponseWrapper;
use crate::backend::binance::types::OrderUpdateData;
use crate::backend::binance::types::PositionUpdateData;
use crate::backend::types::Side;
use crate::orderbook::Tops;
use crate::strategy::types::OrderClassification;
use crate::strategy::types::Stage;

use super::AccountMessage;
use super::CancelResponseContext;
use super::ModelMessage;
use super::OrderResponseContext;
use super::Portfolio;
use super::StratBranch;
use super::StrategyMessage;

use tokio::runtime::{Runtime, Builder};

pub const RISK: usize = 10;
pub const SCALE_RISK: usize = 0;
pub const SCALE: i32 = 2;
pub const RATE_CAP: i32 = 10;

pub const CHIRP: bool = false;
pub const CHIRP_ON_FLIP: bool = true;
pub const CHIRP_INCLUDES_DATA: bool = true;

lazy_static! {
    pub static ref REBATE: D128 = D128::from(0.00025);
    pub static ref MAX_OPEN_DIST: D128 = D128::from(30);
    pub static ref TOP_OPEN_DIST: D128 = D128::from(6);
}

pub struct Strategy {
    pub strat_tx: Sender<StrategyMessage>,
    pub strat_rx: Receiver<StrategyMessage>,
    asset_portfolio: Portfolio,
    pub total_cancels: u32,
    pub total_fills: u32,
    pub max_risked_liq: D128,
    last_buy_branch: StratBranch,
    last_sell_branch: StratBranch,
    // ip_limits: IPLimits,
    // endpoint_limits: EndpointLimits,
}

impl Strategy {
    pub fn new(
        symbol: String,
        strat_tx: Sender<StrategyMessage>,
        strat_rx: Receiver<StrategyMessage>
    ) -> tokio::io::Result<Strategy> {
        let pp_strat_tx = strat_tx.clone();
        Ok(Strategy {
            strat_tx,
            strat_rx,
            total_cancels: 0,
            total_fills: 0,
            last_buy_branch: StratBranch::SSS,
            last_sell_branch: StratBranch::SSS,
            asset_portfolio: Portfolio::new(pp_strat_tx, symbol)?,
            max_risked_liq: D128::from(5000 as u32),
            // ip_limits: IPLimits::new(),
            // endpoint_limits: EndpointLimits::new(),
        })
    }

    pub fn listen(&mut self) {
        loop {
            if self.strat_rx.len() > 1 {
                // debug!("Strategy fell behind. {} messages were waiting to be processed", self.strat_rx.len());
            }
            match self.strat_rx.recv().unwrap() {
                StrategyMessage::ModelMessage(mm) => match mm {
                    ModelMessage::TradeFlowMessage(tr) => self.tradeflow_update(tr),
                    ModelMessage::OrderBookMessage(br) => self.orderbook_update(br),
                    ModelMessage::TopsMessage(t) => self.tops_update(t),
                },
                StrategyMessage::AccountMessage(am) => match am {
                    AccountMessage::PositionUpdate(pud) => self.position_update(pud),
                    AccountMessage::OrderUpdate(oud) => self.order_update(oud),
                    AccountMessage::OrderResponse(or) => self.order_response(or),
                    AccountMessage::CancelResponse(cr) => self.cancel_response(cr),
                },
            }
        }
    }

    pub fn tops(&mut self, side: Side, tops: Tops) {
        let entry_price = side.deside(&tops.best_bid, &tops.best_ask).0;
        let exit_price = side.deside(&tops.best_ask, &tops.best_bid).0;

        match self.resolve_strat_branch(side) {
            StratBranch::NNN => {
                self.asset_portfolio.new_limit(
                    None,
                    entry_price,
                    D128::from(0.001),
                    side,
                    Stage::Entry,
                    OrderClassification::Top,
                );
            },
            StratBranch::NNS => {
                let exit_size = side.deside(&self.asset_portfolio.data.buy, &self.asset_portfolio.data.sell).open_position.inv;
                self.asset_portfolio.new_limit(
                    None,
                    exit_price,
                    exit_size,
                    side,
                    Stage::Exit,
                    OrderClassification::Top,
                );
            },
            StratBranch::NSS => {
                self.asset_portfolio.cancel_non_tops(exit_price, side, Stage::Exit);
            },
            StratBranch::SNN => {
                self.asset_portfolio.cancel_non_tops(entry_price, side, Stage::Entry);
            },
            StratBranch::SNS => {
            },
            StratBranch::SSS => {
                self.asset_portfolio.cancel_non_tops(exit_price, side, Stage::Exit);
            },
            StratBranch::NSN | StratBranch::SSN => {
                // Probable error case: open positions with no inventory
                println!("{} SSN pdata: {}", match side { Side::Buy => self.asset_portfolio.data.buy, Side::Sell => self.asset_portfolio.data.sell, }, side);
                let (position, converse_position) = match side {Side::Buy => ( &mut self.asset_portfolio.buy, &mut self.asset_portfolio.sell), Side::Sell => ( &mut self.asset_portfolio.sell, &mut self.asset_portfolio.buy) };
                panic!("POSSIBLE DESYNC: CLOSING ORDERS RESTING WITH EMPTY POSITION: {:?}", position);
            }
        }
    }

    pub fn orderbook(&mut self, side: Side, ob: BookResult) {

    }

    pub fn tradeflow_update(&mut self, tr: TradeResult) {
        // info!("{}", tr.test_timer.elapsed().as_nanos());
    }

    pub fn orderbook_update(&mut self, br: BookResult) {
        // info!("{}", br.test_timer.elapsed().as_nanos());
        self.orderbook(Side::Buy, br);
        self.orderbook(Side::Sell, br);
    }

    pub fn tops_update(&mut self, tops: Tops) {
        self.tops(Side::Buy, tops);
        self.tops(Side::Sell, tops);
        // info!("Tops timer: {}", tops.test_timer.elapsed().as_micros());
    }

    pub fn position_update(&mut self, pud: PositionUpdateData) {
        // info!("{:?}", pud);
    }

    pub fn order_update(&mut self, oud: OrderUpdateData) {
        // info!("{:?}", oud);
        self.asset_portfolio.order_update(oud);
    }

    pub fn order_response(&mut self, or: OrderResponseContext) {
        // info!("{:?}", or);
        self.asset_portfolio.order_rest_response(or.id, or.side, or.stage, or.rest_response);
    }

    pub fn cancel_response(&mut self, cr: CancelResponseContext) {
        // info!("{:?}", cr);
        self.asset_portfolio.cancel_response(cr.id, cr.side, cr.stage, cr.rest_response);
    }

    fn chirp(&mut self, branch: StratBranch, side: Side) -> bool {
        match CHIRP {
            true => match CHIRP_ON_FLIP {
                true => match (match side { Side::Buy => self.last_buy_branch, Side::Sell => self.last_sell_branch }) != branch {
                        true => {
                            match side { Side::Buy => self.last_buy_branch = branch, Side::Sell => self.last_sell_branch = branch };
                            info!("{} {:?}", side, branch);
                            if CHIRP_INCLUDES_DATA { info!("data: {}", self.asset_portfolio.data); }
                            true
                        },
                        false => {
                            match side { Side::Buy => self.last_buy_branch = branch, Side::Sell => self.last_sell_branch = branch };
                            false
                        },
                    },
                false => {
                    match side { Side::Buy => self.last_buy_branch = branch, Side::Sell => self.last_sell_branch = branch };
                    info!("{} {:?}", side, branch);
                    if CHIRP_INCLUDES_DATA { info!("data: {}", self.asset_portfolio.data); }
                    true
                },
            },
            false => {
                match side { Side::Buy => self.last_buy_branch = branch, Side::Sell => self.last_sell_branch = branch };
                false
            },
        }
    }

    fn resolve_strat_branch(&mut self, side: Side) -> StratBranch {
        if side.deside(&self.asset_portfolio.data.buy, &self.asset_portfolio.data.sell).open_liqs.total_reserved.count < D128::ZERO ||
        side.deside(&self.asset_portfolio.data.buy, &self.asset_portfolio.data.sell).close_liqs.total_reserved.count < D128::ZERO {
            panic!("reserve count dropped below 0");
        }
        let branch = StratBranch::from((side.deside(&self.asset_portfolio.data.buy, &self.asset_portfolio.data.sell).open_liqs.total_reserved.count == D128::ZERO,
            side.deside(&self.asset_portfolio.data.buy, &self.asset_portfolio.data.sell).close_liqs.total_reserved.count == D128::ZERO,
            side.deside(&self.asset_portfolio.data.buy, &self.asset_portfolio.data.sell).open_liqs.filled.liq <= D128::ZERO));
        self.chirp(branch, side);
        branch
    }
}