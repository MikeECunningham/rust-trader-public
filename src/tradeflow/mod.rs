use std::fs::OpenOptions;
use std::str::FromStr;

use dec::D128;
use serde::{Deserialize, Serialize};
use std:: time::Instant;

use std::collections::{VecDeque};

use crate::analysis::stats::{RegularStats, NormalStats};
use crate::backend::binance::types::FuturesTrades;
use crate::backend::bybit::broker::Side;
use crate::backend::bybit::stream::TradeData;
use crate::backend::types::Exchange;

#[derive(Copy, Clone)]
pub struct OrderValue {
    pub price: D128,
    pub quantity: D128,
    pub maker_id: i64,
    pub transaction_time: u128,
}

pub struct OrderMetrics {
    pub price: RegularStats,
    pub volume: RegularStats,
    pub time: RegularStats,
    // density: Stats,
    pub liquidity: RegularStats,
    pub forever_liquidity: NormalStats,
}

impl OrderMetrics {
    fn new() -> OrderMetrics {
        return OrderMetrics {
            price: RegularStats::new(),
            volume: RegularStats::new(),
            time: RegularStats::new(),
            liquidity: RegularStats::new(),
            forever_liquidity: NormalStats::new(),
        };
    }
    fn init(init_price: D128, init_volume: D128, init_time: u64) -> OrderMetrics {
        return OrderMetrics {
            price: RegularStats::init(init_time, init_price),
            volume: RegularStats::init(init_time, init_volume),
            time: RegularStats::init(init_time, D128::from(init_time as u32)),
            liquidity: RegularStats::init(init_time, init_price * init_volume),
            forever_liquidity: NormalStats::new(),
        };
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TradeFlowValue {
    pub price: D128,
    pub quantity: D128,
    pub timestamp: u64,
}

impl TradeFlowValue {
    pub fn new() -> TradeFlowValue {
        return TradeFlowValue {
            price: D128::ZERO,
            quantity: D128::ZERO,
            timestamp: 0,
        };
    }
}

#[derive(Serialize, Debug)]
pub struct TradeFlowRecord {
    pub side: String,
    pub price: D128,
    pub quantity: D128,
    pub liquidity: D128,
    pub timestamp: i64,
}

#[derive(Deserialize, Debug)]
pub struct TradeRecord {
    pub timestamp: f64,
    pub symbol: String,
    pub side: Side,
    pub size: f64,
    pub price: f64,
    #[serde(rename = "tickDirection")]
    pub tick_direction: String,
    #[serde(rename = "trdMatchID")]
    pub trd_match_id: String,
    #[serde(rename = "grossValue")]
    pub gross_value: f64,
    #[serde(rename = "homeNotional")]
    pub home_notional: f64,
    #[serde(rename = "foreignNotional")]
    pub foreign_notional: f64,
}

pub struct TradeFlow {
    pub buys: VecDeque<(u64, TradeFlowValue)>,
    pub sells: VecDeque<(u64, TradeFlowValue)>,
    pub buy_metrics: OrderMetrics,
    pub sell_metrics: OrderMetrics,
    pub last_buy: OrderMetrics,
    pub last_sell: OrderMetrics,
    pub exchange: Exchange,
    culling_threshold: u64,
    // logger: Sender<TradeFlowRecord>,
}

impl TradeFlow {
    pub fn new() -> TradeFlow {

        let mut tradeflow = TradeFlow {
            buys: VecDeque::new(),
            sells: VecDeque::new(),
            buy_metrics: OrderMetrics::new(),
            sell_metrics: OrderMetrics::new(),
            last_buy: OrderMetrics::new(),
            last_sell: OrderMetrics::new(),
            exchange: Exchange::None,
            culling_threshold: 2000,
            // logger: tr_send.clone(),
        };

        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open("BTCUSDT2021-11-25.csv")
            .expect("error writing trade to logger");
        let mut rdr = csv::Reader::from_reader(file);
        let loadtimer = Instant::now();
        info!("[INIT] Loading trade data from csv to stats");
        let mut count = 0;
        for (i, result) in rdr.deserialize().enumerate() {
            count = i;
            match result {
                Ok(tr) => {
                    let record: TradeRecord = tr;
                    match record.side {
                        Side::Buy => {
                            tradeflow.buy_metrics.forever_liquidity.add(
                                D128::from_str(&(record.price * record.size).to_string())
                                    .expect("err serializing csv liquidity"),
                            );
                        }
                        Side::Sell => {
                            tradeflow.sell_metrics.forever_liquidity.add(
                                D128::from_str(&(record.price * record.size).to_string())
                                    .expect("err serializing csv liquidity"),
                            );
                        }
                    }
                }
                Err(e) => eprintln!("error getting csv result: {}", e),
            }
        }
        info!(
            "[INIT] Done loading trade csv, {} files in {} microseconds",
            count,
            loadtimer.elapsed().as_micros()
        );
        info!(
            "[INIT] buy liq from csv 3rd dev {}",
            tradeflow.buy_metrics.forever_liquidity.mean
                + tradeflow.buy_metrics.forever_liquidity.stan_dev * 3
        );

        tradeflow
    }

    fn exchange_check(&mut self, exchange: Exchange) {
        if self.exchange == Exchange::None {
            self.exchange = exchange
        } else if self.exchange != exchange {
            panic!("Updated OB locked to a different exchange");
        }
    }

    pub fn binance_update(&mut self, update: FuturesTrades) {
        self.exchange_check(Exchange::Binance);

        self.last_buy = OrderMetrics::new();
        self.last_sell = OrderMetrics::new();

        let cull_time = update.transaction_time - self.culling_threshold;

        if update.buyer_maker {
            self.sells.push_back((update.transaction_time, TradeFlowValue {
                price: update.price,
                quantity: update.quantity,
                timestamp: update.transaction_time,
            }));
            self.last_sell.price.add(update.transaction_time, update.price);
            self.last_sell.volume.add(update.transaction_time, update.quantity);
            self.last_sell.liquidity.add(update.transaction_time, update.price * update.quantity);
            self.last_sell.time.add(update.transaction_time, D128::from(update.transaction_time));
            self.last_sell.forever_liquidity.add(update.price * update.quantity);

            self.sell_metrics.price.add(update.transaction_time, update.price);
            self.sell_metrics.volume.add(update.transaction_time, update.quantity);
            self.sell_metrics.liquidity.add(update.transaction_time, update.price * update.quantity);
            self.sell_metrics.time.add(update.transaction_time, D128::from(update.transaction_time));
            self.sell_metrics.forever_liquidity.add(update.price * update.quantity);

            self.sell_metrics.price.prune(cull_time);
            self.sell_metrics.volume.prune(cull_time);
            self.sell_metrics.liquidity.prune(cull_time);
            self.sell_metrics.time.prune(cull_time);
        } else {
            self.buys.push_back((update.transaction_time, TradeFlowValue {
                price: update.price,
                quantity: update.quantity,
                timestamp: update.transaction_time,
            }));
            self.last_buy.price.add(update.transaction_time, update.price);
            self.last_buy.volume.add(update.transaction_time, update.quantity);
            self.last_buy.liquidity.add(update.transaction_time, update.price * update.quantity);
            self.last_buy.time.add(update.transaction_time, D128::from(update.transaction_time));
            self.last_buy.forever_liquidity.add(update.price * update.quantity);

            self.buy_metrics.price.add(update.transaction_time, update.price);
            self.buy_metrics.volume.add(update.transaction_time, update.quantity);
            self.buy_metrics.liquidity.add(update.transaction_time, update.price * update.quantity);
            self.buy_metrics.time.add(update.transaction_time, D128::from(update.transaction_time));
            self.buy_metrics.forever_liquidity.add(update.price * update.quantity);

            self.buy_metrics.price.prune(cull_time);
            self.buy_metrics.volume.prune(cull_time);
            self.buy_metrics.liquidity.prune(cull_time);
            self.buy_metrics.time.prune(cull_time);
        }
        self.buys.drain(..self.buys.partition_point(|(timestamp, _)| timestamp < &cull_time));
        self.sells.drain(..self.sells.partition_point(|(timestamp, _)| timestamp < &cull_time));
    }

    pub fn bybit_update(&mut self, update: Vec<TradeData>) {
        self.exchange_check(Exchange::Bybit);

        self.last_buy = OrderMetrics::new();
        self.last_sell = OrderMetrics::new();
        for order in update.iter() {
            let price: D128 = match order.price.parse() {
                Err(_) => {
                    return;
                }
                Ok(p) => p,
            };
            let quantity: D128 = D128::from_str(&order.size.to_string())
                .expect("something went wrong converting size to D128");
            let timestamp: u64 = order
                .trade_time_ms
                .parse()
                .expect("something went wrong parsing trade time stamp");

            let cull_time = timestamp - self.culling_threshold;

            if order.side == "Buy" {
                let lb = TradeFlowValue {
                    price,
                    quantity,
                    timestamp,
                };

                self.buys.push_back((timestamp, lb));
                self.last_buy.price.add(timestamp, price);
                self.last_buy.volume.add(timestamp, quantity);
                self.last_buy.liquidity.add(timestamp, price * quantity);
                self.last_buy
                    .time
                    .add(timestamp, D128::from(timestamp));

                self.buy_metrics.price.add(timestamp, price);
                self.buy_metrics.volume.add(timestamp, quantity);
                self.buy_metrics.liquidity.add(timestamp, price * quantity);
                self.buy_metrics
                    .time
                    .add(timestamp, D128::from(timestamp));
                self.buy_metrics.forever_liquidity.add(price * quantity);

                self.buy_metrics.price.prune(cull_time);
                self.buy_metrics.volume.prune(cull_time);
                self.buy_metrics.liquidity.prune(cull_time);
                self.buy_metrics.time.prune(cull_time);
                let drain_end = self
                    .buys
                    .partition_point(|(timestamp, _)| timestamp < &cull_time);
                self.buys.drain(..drain_end);
            } else if order.side == "Sell" {
                let ls = TradeFlowValue {
                    price,
                    quantity,
                    timestamp,
                };

                self.sells.push_back((timestamp, ls));
                self.last_sell.price.add(timestamp, price);
                self.last_sell.volume.add(timestamp, quantity);
                self.last_sell.liquidity.add(timestamp, price * quantity);
                self.last_sell
                    .time
                    .add(timestamp, D128::from(timestamp));

                self.sell_metrics.price.add(timestamp, price);
                self.sell_metrics.volume.add(timestamp, quantity);
                self.sell_metrics.liquidity.add(timestamp, price * quantity);
                self.sell_metrics
                    .time
                    .add(timestamp, D128::from(timestamp));
                self.sell_metrics.forever_liquidity.add(price * quantity);

                self.sell_metrics.price.prune(cull_time);
                self.sell_metrics.volume.prune(cull_time);
                self.sell_metrics.liquidity.prune(cull_time);
                self.sell_metrics.time.prune(cull_time);
                let drain_end = self
                    .sells
                    .partition_point(|(timestamp, _)| timestamp < &cull_time);
                self.sells.drain(..drain_end);
            }
        }
    }
}
