/// This is an event loop that listens to signals from multiple websocket streams
/// that are constantly processed and output events that this file reacts to

use crate::backend::bybit::stream::{Signal, PrivateTicks, OBTick, TradeTick};
use crate::analysis::Analysis;
use crate::tradeflow::TradeFlow;
use crate::strategy::bybit::{StrategyMessage, ModelMessage, OrderBookMessage, AccountMessage, PositionMessage, OrderMessage, BybitOrderTickSignal};
use crate::orderbook::OrderBook;
use crossbeam_channel::Sender;
use thiserror::Error;

use std::num::ParseIntError;

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


    /// Waits for a snapshot to come in through the socket and then fills the local models with the
    /// relevent information
    /// When the snapshot tick comes in, this processes the data to create an initial orderbook structure.
    /// This blocks the rest of the orderbook model loop until the snapshot arrives.
    /// While we are waiting for the snapshot, we may possibly get other types of ticks, so this method handles that
    pub fn wait_for_snapshot(&mut self) -> Result<(), SnapshotError>{
        loop {
            match self.signal_rx.blocking_recv() {
                Some(t) => match t {
                    // This handles the initial tick
                    Signal::Orderbook(snapshot) => {
                        let timestamp = snapshot.timestamp.split_at(snapshot.timestamp.len() - 3);
                        self.ob_model.bybit_update(
                            snapshot.data,
                            snapshot
                                .cross_seq
                                .parse()?,
                            snapshot
                                .timestamp
                                .parse()
                                .expect("problem parsing timestamp"),
                        );
                        // Since the initial tick is now handled, we break out of the infinite loop
                        break;
                    }
                    Signal::Tradeflow(tr) => {
                        println!("tr tick in snapshot loop");
                        self.tr_model.bybit_update(tr.data);
                    }
                    Signal::PrivateTicks(pt) => match pt {
                        PrivateTicks::PositionTick(pos) => {
                            for x in pos.data.iter() {
                                println!("position\nprice: {}\nsize: {}", x.entry_price, x.size);
                            }
                        }
                        PrivateTicks::ExecutionTick(exe) => {
                            for x in exe.data.iter() {
                                println!("execution\nprice: {}\nsize: {}", x.price, x.order_qty);
                            }
                        }
                        PrivateTicks::OrderTick(o) => {
                            for ord in o.data.iter() {
                                println!("recvd: order\nprice: {}\nsize: {}", ord.price, ord.qty);
                            }
                        }
                        PrivateTicks::StopOrderTick(so) => {
                            for stord in so.data.iter() {
                                println!(
                                    "recvd: stop order\nprice: {}\nsize: {}",
                                    stord.price, stord.qty
                                );
                            }
                        }
                        PrivateTicks::WalletTick(_) => {},
                    }
                },
                None => {}
            }
            println!("in snapshot loop, no snapshots yet");
        }
        return Ok(());
    }

    /// Handles signals that are meant for orderbook updates
    fn handle_orderbook_signal(&mut self, ob: OBTick) {
        
        let timestamp = ob.timestamp.split_at(ob.timestamp.len() - 3);
        
        self.ob_model.bybit_update(
            ob.data,
            ob.cross_seq.parse().expect("problem parsing cross_seq"),
            timestamp.0.parse().expect("problem parsing timestamp"),
        );

        let analysis_result = Analysis::new_orderbook(&self.ob_model, &self.tr_model);
        self.strat_tx.send(StrategyMessage::ModelMessage(
            ModelMessage::OrderBookMessage(OrderBookMessage {
                orderbook_analysis: analysis_result})
        )).expect("something went wrong sending ob to strat");
    }

    /// Handles tradeflow related signals
    fn handle_tradeflow_signal(&mut self, tr: TradeTick) {
        self.tr_model.bybit_update(tr.data);
        // Comming to a future update near u
        //let _analysis_result = Analysis::new_trade(&self.ob_model, &self.tr_model);
    }

    /// Handles private tick signals 
    fn handle_private_tick_signal(&self, pt: PrivateTicks) {
        match pt {
            PrivateTicks::PositionTick(pt) => {
                self.strat_tx.send(StrategyMessage::AccountMessage(
                    AccountMessage::PositionMessage(
                        PositionMessage::PositionUpdate(pt),
                    ),
                ))
                .expect("msg");
            }
            PrivateTicks::ExecutionTick(et) => {
                self.strat_tx.send(StrategyMessage::AccountMessage(
                    AccountMessage::OrderMessage(OrderMessage::ExecutionUpdate(et)),
                ))
                .expect("msg");
            }
            PrivateTicks::OrderTick(ot) => {
                self.strat_tx.send(StrategyMessage::AccountMessage(
                    AccountMessage::OrderMessage(OrderMessage::OrderUpdate(BybitOrderTickSignal{
                        order_tick: ot
                    })),
                ))
                .expect("msg");
            }
            PrivateTicks::StopOrderTick(sot) => {
                self.strat_tx.send(StrategyMessage::AccountMessage(
                    AccountMessage::OrderMessage(OrderMessage::StopOrderUpdate(
                        sot,
                    )),
                ))
                .expect("msg");
            }
            PrivateTicks::WalletTick(_) => {},
        };
    }

    /// Begins the main modeling event loop.
    /// NOTE: Should be called in a separate thread to prevent blocking the main thread.
    pub fn event_loop(&mut self) {

        // This loop will respond to signals emitted by multiple websocket listeners
        loop {
            match self.signal_rx.blocking_recv() {
                Some(t) => {
                    match t {
                        Signal::Orderbook(ob) => self.handle_orderbook_signal(ob),
                        Signal::Tradeflow(tr) => self.handle_tradeflow_signal(tr),
                        Signal::PrivateTicks(pt) => self.handle_private_tick_signal(pt)
                    }
                }
                None => {  panic!("main receiver loop error"); }
            }
        }
    }
}