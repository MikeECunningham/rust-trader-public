/// This is an event loop that listens to signals from multiple websocket streams
/// that are constantly processed and output events that this file reacts to

use crate::backend::binance::types::{Signal, Orders, BookRefresh, TradeFlows, OrderBookSignal, FuturesTrades, BestLevel, OrderUpdateData, PositionUpdateData};
use crate::analysis::Analysis;
use crate::tradeflow::TradeFlow;
use crate::orderbook::OrderBook;
use crate::strategy::binance::{StrategyMessage, ModelMessage};
use crossbeam_channel::Sender;
use thiserror::Error;

use std::num::ParseIntError;
use std::time::Instant;

#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("Failed to send request")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Failed to serialize response")]
    SerdeError(#[from] serde_json::Error),
    #[error("Failed to parse integer")]
    ParseIntError(#[from] ParseIntError),
}

pub struct SignalHandler {
    /// Current order book model
    ob_model: OrderBook,
    /// Model of trade flow
    tr_model: TradeFlow,
    /// Reciever for events
    signal_rx: tokio::sync::mpsc::Receiver<Signal>,
    /// Emitter to the strategy
    strat_tx: Sender<StrategyMessage>,
}

impl SignalHandler {

    pub fn new(strat_tx: Sender<StrategyMessage>, signal_rx: tokio::sync::mpsc::Receiver<Signal>) -> Self{
        SignalHandler{
            ob_model: OrderBook::new(),
            tr_model: TradeFlow::new(),
            signal_rx,
            strat_tx,
        }
    }

    /// Handles signals that are meant for orderbook updates
    fn handle_orderbook_signal(&mut self, ob: Orders) {
        let timer = ob.test_timer;
        self.ob_model.binance_update(ob);
        // info!("{}", timer.elapsed().as_nanos());

        if self.ob_model.initialized {
            let mut analysis = Analysis::new_orderbook(&self.ob_model, &self.tr_model);
            analysis.test_timer = timer;
            self.strat_tx.send(StrategyMessage::ModelMessage(ModelMessage::OrderBookMessage(analysis))).unwrap();
        }

    }

    /// Handles tradeflow related signals
    fn handle_tradeflow_signal(&mut self, ft: FuturesTrades) {
        let timer = ft.test_timer;
        self.tr_model.binance_update(ft);
        // info!("{}", timer.elapsed().as_nanos());

        if self.ob_model.initialized {
            let mut analysis = Analysis::new_trade(&self.ob_model, &self.tr_model);
            analysis.test_timer = timer;
            self.strat_tx.send(StrategyMessage::ModelMessage(ModelMessage::TradeFlowMessage(analysis))).unwrap();
        }
        //let _analysis_result = Analysis::new_trade(&self.ob_model, &self.tr_model);
    }

    fn handle_book_ticker(&mut self, bt: BestLevel) {
        self.ob_model.tops.test_timer = bt.test_timer;
        self.ob_model.binance_update_best_ticker(bt);
        // info!("{}", self.ob_model.tops.test_timer.elapsed().as_nanos());
        if self.ob_model.initialized {
            let tops = self.ob_model.tops.clone();
            self.strat_tx.send(StrategyMessage::ModelMessage(ModelMessage::TopsMessage(tops))).unwrap();
        }
    }

    fn init_orderbook(&mut self, br: BookRefresh) {
        self.ob_model.binance_refresh(br);
        info!("Finished init");
    }

    /// Begins the main modeling event loop.
    /// NOTE: Should be called in a separate thread to prevent blocking the main thread.
    pub fn event_loop(&mut self) {
        // This loop will respond to signals emitted by multiple websocket listeners
        loop {
            match self.signal_rx.blocking_recv() {
                Some(t) => {
                    match t {
                        Signal::OrderBook(os) => match os {
                            OrderBookSignal::OrderBook(ob) => self.handle_orderbook_signal(ob),
                            OrderBookSignal::BestLevels(bt) => self.handle_book_ticker(bt),
                            OrderBookSignal::OrderBookSnap(br) => self.init_orderbook(br),
                        },
                        Signal::TradeFlows(tf) => match tf {
                            TradeFlows::Trades(ft) => self.handle_tradeflow_signal(ft),
                            TradeFlows::Liquidations(_) => {},
                        },
                    }
                }
                None => {  panic!("main receiver loop error"); }
            }
        }
    }
}