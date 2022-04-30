use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use dec::D128;
use std::time::Instant;


use crate::analysis::BookResult;
use crate::backend::bybit::broker::BROKER;
use crate::backend::bybit::broker::SetServerOffsetError;
use crate::backend::bybit::broker::Side;
use crate::backend::bybit::errors::PerpetualStatus;
use crate::backend::bybit::rate_limits::EndpointLimits;
use crate::backend::bybit::stream::BybitExecutionData;
use crate::backend::bybit::stream::BybitOrderData;
use crate::backend::bybit::stream::BybitPositionData;
use crate::backend::bybit::rate_limits::IPLimits;
use crate::strategy::types::OrderClassification;
use crate::strategy::types::Stage;

use super::AccountMessage;
use super::ApplyBookResultError;
use super::Portfolio;
use super::CancelOrderResponseError;
use super::CancelResponse;
use super::FindCancelRes;
use super::IncomingOrderREST;
use super::IncomingOrderWS;
use super::IncomingPosition;
use super::ModelMessage;
use super::OrderMessage;
use super::OrderResponse;
use super::OrderResponseError;
use super::PositionMessage;
use super::StratBranch;
use super::StrategyMessage;
use super::StrategyRuntimeError;
use super::UnauthorizedRequestError;


pub const RISK: usize = 10;
pub const SCALE_RISK: usize = 0;
pub const SCALE: i32 = 2;
pub const RATE_CAP: i32 = 10;

pub const CHIRP: bool = false;
pub const CHIRP_ON_FLIP: bool = true;

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
    ip_limits: IPLimits,
    endpoint_limits: EndpointLimits,
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
            ip_limits: IPLimits::new(),
            endpoint_limits: EndpointLimits::new(),
        })
    }

    /// Starts the event loop that receivs strategy signals and updates the
    /// current state of the strategy accordingly
    pub fn listen(&mut self) -> Result<(), StrategyRuntimeError> {
        loop {
            if self.strat_rx.len() > 1 {
                debug!("Strategy fell behind. {} messages were waiting to be processed", self.strat_rx.len());
            }
            match self.strat_rx.recv() {
                Ok(sm) => {
                    //debug!("Received message in strategy listener");
                    match sm {
                        StrategyMessage::ModelMessage(mm) => match mm {
                            ModelMessage::OrderBookMessage(obm) => {
                                self.book_update(&obm.orderbook_analysis);
                            }
                            ModelMessage::TradeFlowMessage(tfm) => {
                                self.trade_update();
                            }
                        },
                        StrategyMessage::AccountMessage(acc) => {
                            match acc {
                                AccountMessage::OrderMessage(om) => match om {
                                    OrderMessage::OrderResult(or) => {
                                        if let Err(err) = self.order_response(or) {
                                            if let OrderResponseError::ContactSupportError(fatal_err) = err {
                                                return Err(StrategyRuntimeError::ContactSupportError(fatal_err))
                                            }
                                        }
                                    }
                                    OrderMessage::OrderUpdate(ot) => {
                                        // info!("order update {:?}", ot);
                                        for data in ot.order_tick.data.iter() {
                                            self.asset_portfolio.order_update(
                                                IncomingOrderWS::from(data),
                                                self.strat_tx.clone(),
                                            );
                                        }
                                    }
                                    OrderMessage::CancelResult(cr) => {
                                        self.cancel_order_response(cr);
                                    }
                                    OrderMessage::StopOrderUpdate(sot) => {
                                        debug!("stop order tick lmao");
                                    }
                                    OrderMessage::ExecutionUpdate(et) => {
                                        for exec in et.data {
                                            self.execution_update(exec);
                                        }
                                    }
                                },
                                AccountMessage::PositionMessage(pm) => match pm {
                                    PositionMessage::PositionUpdate(pt) => {
                                        for pos in pt.data {
                                            self.asset_portfolio.position_update(IncomingPosition::from(&pos));
                                        }
                                    }
                                },
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        }
    }

    fn chirp(&mut self, branch: StratBranch, side: Side) -> bool {
        match CHIRP {
            true => match CHIRP_ON_FLIP {
                true => match (match side { Side::Buy => self.last_buy_branch, Side::Sell => self.last_sell_branch }) != branch {
                        true => {
                            match side { Side::Buy => self.last_buy_branch = branch, Side::Sell => self.last_sell_branch = branch };
                            info!("{} {:?}", side, branch);
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
                    true
                },
            },
            false => {
                match side { Side::Buy => self.last_buy_branch = branch, Side::Sell => self.last_sell_branch = branch };
                false
            },
        }
    }

    fn strat_branch(opens: bool, closes: bool, inventory: bool) -> StratBranch {
        if opens {
            if closes {
                if inventory {
                    StratBranch::NNN
                } else {
                    StratBranch::NNS
                }
            } else {
                if inventory {
                    StratBranch::NSN
                } else {
                    StratBranch::NSS
                }
            }
        } else {
            if closes {
                if inventory {
                    StratBranch::SNN
                } else {
                    StratBranch::SNS
                }
            } else {
                if inventory {
                    StratBranch::SSN
                } else {
                    StratBranch::SSS
                }
            }
        }
    }
    
    fn apply_book_result_side(&mut self, side: Side, cb_rebate: D128, book: &BookResult) -> Result<(), ApplyBookResultError> {
        // let timer = std::time::Instant::now();
        let exit_side = !side;
        let init_size = self.asset_portfolio.init_size;
        // info!("start pdata: {}", self.asset_portfolio.data);
        // [OPENS, CLOSES, INVENTORY]
        // debug!("strat timer {}", timer.elapsed().as_nanos());
        return Ok(());
    }

    /// Responds to an order book update.
    /// This is where the majority of the logic for the strategy will go
    fn book_update(&mut self, book: &BookResult) {
        // alright lets lose some money
        let timer = Instant::now();
        self.asset_portfolio.data_refresh();
        // info!("data refresh timer: {}", timer.elapsed().as_nanos());
        // Bullish side
        self.apply_book_result_side(Side::Buy, D128::ONE - *REBATE, book);
        // info!("data refresh + one side book update timer: {}", timer.elapsed().as_nanos());
        self.asset_portfolio.data_refresh();
        // Bearish side
        self.apply_book_result_side(Side::Sell, D128::ONE + *REBATE, book);
        // info!("full book update timer: {}", timer.elapsed().as_nanos());
    }

    fn trade_update(&mut self) {
        // Bullish
        self.trade_update_side(true);
        // Bearish
        self.trade_update_side(false);
    }

    fn trade_update_side(&mut self, bullish: bool) {
        // let position = match bullish {true => &mut self.asset_portfolio.buy, false => &mut self.asset_portfolio.sell };
        // // BULL SIDE
        // if position.active_opens.len() == 0 {
        //     if position.active_closes.len() == 0 {
        //         if position.known_inventory <= D128::ZERO {
        //             // Not in a position, no orders active
        //         } else {
        //             // In a position, no orders active
        //             debug!("buyside close (sell)");
        //         }
        //     } else {
        //         if position.known_inventory <= D128::ZERO {
        //             // No position, closing orders active
        //             debug!("\nPOSSIBLE DESYNC: CLOSING ORDERS RESTING WITH EMPTY POSITION: {:?}\n", position);
        //         } else {
        //             // In a position, closing orders active
        //         }
        //     }
        // } else {
        //     if position.active_closes.len() == 0 {
        //         if position.known_inventory <= D128::ZERO {
        //             // No position, opens active
        //         } else {
        //             // In a position, opens active
        //         }
        //     } else {
        //         if position.known_inventory <= D128::ZERO {
        //             // No position, opens and closes active
        //             debug!("\nPOSSIBLE DESYNC: CLOSING (AND OPENING) ORDERS RESTING WITH EMPTY POSITION: {:?}\n", position);
        //         } else {
        //             // In a position, opens and closes active
        //         }
        //     }
        // }
    }

    fn order_response(&mut self, or: OrderResponse) -> Result<(), OrderResponseError> {
        // debug!("Processing order result: {:?}", or);
        let id = or.id;
        let side = or.side;
        let stage = or.stage;
        let sent = or.sent;
        let class = or.class;
        let or = or.rest_response;
        // if class == OrderClassification::Top { info!("top res: {:?}", or); }

        match or.ret_code {
            PerpetualStatus::Ok => {},
            PerpetualStatus::RequestNotAuthorized => BROKER.set_server_offset(self.handle_unauthorized_request(or.ret_msg)?)?,
            PerpetualStatus::CloseOrderSideLargerThanPosLeavingQty => {
                debug!("Tried to close beyond position size: {:?}", or);
                // self.asset_portfolio.drop_order(id, side, stage);
            }
            PerpetualStatus::NoChangeMadeForTpSlPrice => {
                panic!("No change made for tpsl price: {:?}\noriginal ord: {}", or, sent);
            }
            PerpetualStatus::ParamsError => panic!("Params error. {}\nportf: {}", or.ret_msg, self.asset_portfolio.data),
            PerpetualStatus::SystemNotRespondingContactSupport => return Err(OrderResponseError::ContactSupportError(or.ret_msg)),
            _ => {
                eprintln!("{:?}", or);
                todo!() // And boy, is there ever a lot to do.
            }
        }
        let res = match or.result {
            Some(ordres) => Some(IncomingOrderREST::from(ordres)),
            None => None,
        };
        // info!("sent: {:?}\ngot: {}", res, sent);
        self.asset_portfolio.order_rest_response(id, side, stage, res);
        Ok(())
    }

    fn order_update(&mut self, order_update: &BybitOrderData) {}

    fn cancel_order_response(&mut self, cancel: CancelResponse) -> Result<(), CancelOrderResponseError>{
        // info!("cancel res: {:?}", cancel);
        let id = cancel.id;
        let auto_id = cancel.auto_id;
        let side = cancel.side;
        let stage = cancel.stage;
        let cancel = cancel.rest_response;
        let mut success = false;
        match cancel.ret_code {
            PerpetualStatus::Ok => {
                // info!("Cancel successful, dropping order {:?}", id);
                success = true;
            }
            PerpetualStatus::OrderDoesntExistOrTooLateToCancel => {
                info!("ORDER NOT EXISTS: {:?}", cancel);
            }
            PerpetualStatus::RequestNotAuthorized => {
                BROKER.set_server_offset(self.handle_unauthorized_request(cancel.ret_msg)?)?;
            },
            PerpetualStatus::SystemNotRespondingContactSupport => return Err(CancelOrderResponseError::ContactSupportError(cancel.ret_msg)),
            _ => {
                eprintln!("{:?}", cancel);
                todo!() // And boy, is there ever a lot to do.
            }
        }
        self.asset_portfolio.cancel_response(id, auto_id, side, stage, success);
        return Ok(());
    }

    fn execution_update(&self, exec_update: BybitExecutionData) {
        // debug!("exec tick: {:?}", exec_update);
    }

    fn position_update(&self, pos_update: BybitPositionData) {}

    fn handle_unauthorized_request(&mut self, ret_msg: String) -> Result<i128, UnauthorizedRequestError> {
        debug!("Unauthorized request sent");
        match ret_msg.find("req_timestamp: ") {
            Some(mut req_start) => {
                req_start += 15;

                match ret_msg.find(" server_timestamp: ") {
                    Some(mut server_start) => {
                        let req_end = server_start;
                        server_start += 19;
                        match ret_msg.find(" recv_window: ") {
                            Some(mut server_end) => {
                                let req = ret_msg[req_start..req_end].to_string();
                                let server = ret_msg[server_start..server_end].to_string();
                                let req: i128 = req.parse()?;
                                let server: i128 = server.parse()?;
                                let diff = server - req;
                                Ok(diff / 2)
                            }
                            None => Err(UnauthorizedRequestError::ParseResponseError("error parsing timestamp error message's recv_window index".to_string())),
                        }
                    }
                    None => {
                        Err(UnauthorizedRequestError::ParseResponseError("error parsing timestamp error message's server_timestamp index".to_string()))
                    }
                }
            }
            None => Err(UnauthorizedRequestError::ParseResponseError("error parsing timestamp error message's req_timestamp index".to_string())),
        }
    }
}
