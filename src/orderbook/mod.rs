use dec::D128;
use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::analysis::stats::RegularStats;
use crate::backend::binance::types::{Orders, BookRefresh, BestLevel, BinanceSide};
use crate::backend::bybit::stream::{OBTickData, BybitTickTypes};
use crate::backend::types::Exchange;

use serde::ser::{Serialize, SerializeMap, SerializeStruct, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::collections::btree_map::Entry;

use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub struct OrderBookValue {
    pub volume: D128,
    pub liquidity: D128,
    // pub liquidity_stats: RegularStats,
    pub timestamp: u64,
    pub sequence: u64,
}

impl OrderBookValue {
    pub fn new() -> OrderBookValue {
        OrderBookValue {
            volume: D128::ZERO,
            liquidity: D128::ZERO,
            // liquidity_stats: RegularStats::new(),
            timestamp: 0,
            sequence: 0,
        }
    }
}

impl Serialize for OrderBookValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // This code artifact needs expert restoration
        let mut state = serializer.serialize_struct("OrderBookValue", 7)?;
        state.serialize_field("volume", &self.volume.to_string())?;
        state.serialize_field("last_update_id", &self.timestamp)?;
        state.end()
    }
}

#[derive(Clone, Debug)]
pub struct OrderBookKey {
    pub key: D128,
}

impl OrderBookKey {
    pub fn new(price: &str) -> OrderBookKey {
        return OrderBookKey {
            key: D128::from_str(price).unwrap(),
        };
    }
}

impl Ord for OrderBookKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.partial_cmp(&other.key).unwrap()
    }
}

impl PartialOrd for OrderBookKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for OrderBookKey {
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(&other.key)
    }
}

impl Serialize for OrderBookKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("OrderBookKey", 1)?;
        state.serialize_field("key", &self.key.to_string())?;
        state.end()
    }
}

impl Eq for OrderBookKey {}

#[derive(Clone, Debug)]
pub struct OrderBookSide<T> {
    pub book: BTreeMap<OrderBookKey, T>,
}

impl Display for OrderBookSide<OrderBookValue> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        return match serde_json::to_string(&self) {
            Ok(v) => write!(f, "{}", v),
            Err(e) => write!(f, "{}", e),
        };
    }
}

impl Serialize for OrderBookSide<OrderBookValue> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.book.len()))?;
        for (k, v) in &self.book {
            map.serialize_entry(k.key.to_string().as_str(), &v)?;
        }
        map.end()
    }
}

impl<T> OrderBookSide<T> {
    pub fn new() -> OrderBookSide<T> {
        OrderBookSide {
            book: BTreeMap::new(),
        }
    }

    pub fn get(&self, key: &OrderBookKey) -> Option<&T> {
        self.book.get(key)
    }

    pub fn get_mut(&mut self, key: &OrderBookKey) -> Option<&mut T> {
        return self.book.get_mut(key);
    }

    pub fn set(&mut self, key: OrderBookKey, value: T) -> &mut OrderBookSide<T> {
        self.book.insert(key, value);
        return self;
    }

    pub fn remove(&mut self, key: &OrderBookKey) -> &mut OrderBookSide<T> {
        self.book.remove(key);
        return self;
    }

    pub fn next_front(&mut self, key: &OrderBookKey) -> Option<(&OrderBookKey, &mut T)> {
        let exclude = OrderBookKey { key: key.key };
        return match self.book.range_mut(..exclude).next() {
            None => None,
            Some(v) => Option::Some(v),
        };
    }

    pub fn next_back(&mut self, key: &OrderBookKey) -> Option<(&OrderBookKey, &mut T)> {
        let exclude = OrderBookKey { key: key.key };
        return match self.book.range_mut(exclude..).next() {
            None => None,
            Some(v) => Option::Some(v),
        };
    }

    pub fn prev_front(&mut self, key: &OrderBookKey) -> Option<(&OrderBookKey, &mut T)> {
        let exclude = OrderBookKey { key: key.key };
        return match self.book.range_mut(..exclude).next_back() {
            None => None,
            Some(v) => Option::Some(v),
        };
    }

    pub fn prev_back(&mut self, key: &OrderBookKey) -> Option<(&OrderBookKey, &mut T)> {
        let exclude = OrderBookKey { key: key.key };
        return match self.book.range_mut(exclude..).next_back() {
            None => None,
            Some(v) => Option::Some(v),
        };
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tops {
    pub ask_total_liq: D128,
    pub spread: D128,
    // pub spread_stats: RegularStats,
    pub best_bid: (D128, D128),
    pub best_ask: (D128, D128),
    // pub bid_price_stats: RegularStats,
    // pub ask_price_stats: RegularStats,
    // pub bid_qty_stats: RegularStats,
    // pub ask_qty_stats: RegularStats,
    pub test_timer: Instant,
    pub updated_last_tick: BinanceSide,
}

impl Tops {
    pub fn new() -> Self {
        Tops {
            ask_total_liq: D128::ZERO,
            best_bid: (D128::ZERO, D128::ZERO),
            best_ask: (D128::ZERO, D128::ZERO),
            spread: D128::ZERO,
            // spread_stats: RegularStats::new(),
            // bid_price_stats: RegularStats::new(),
            // bid_qty_stats: RegularStats::new(),
            // ask_price_stats: RegularStats::new(),
            // ask_qty_stats: RegularStats::new(),
            test_timer: Instant::now(),
            updated_last_tick: BinanceSide::Both,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrderBook {
    pub bids: OrderBookSide<OrderBookValue>,
    pub bid_total_liq: D128,
    pub asks: OrderBookSide<OrderBookValue>,
    pub culling_threshold: u64,
    pub internal_time: Instant,
    pub exchange: Exchange,
    pub last_sequence: u64,
    pub maker_commission: f32,
    pub taker_commission: f32,
    pub highest_jump: D128,
    pub initialized: bool,
    pub tops: Tops,
    pub last: D128,
}

impl OrderBook {
    pub fn new() -> OrderBook {
        let culling_threshold = 2000;
        OrderBook {
            bids: OrderBookSide::new(),
            bid_total_liq: D128::ZERO,
            asks: OrderBookSide::new(),
            culling_threshold,
            internal_time: Instant::now(),
            exchange: Exchange::None,
            last_sequence: 0,
            maker_commission: 0.0,
            taker_commission: 0.0,
            tops: Tops::new(),
            highest_jump: D128::ZERO,
            initialized: false,
            last: D128::ZERO,
        }
    }

    pub fn find_best_ask(&self) -> Option<(&OrderBookKey, &OrderBookValue)> {
        return match self.asks.book.iter().next() {
            None => None,
            Some(v) => Some(v.clone()),
        };
    }

    pub fn find_best_bid(&self) -> Option<(&OrderBookKey, &OrderBookValue)> {
        return match self.bids.book.iter().next_back() {
            None => None,
            Some(v) => Some(v.clone()),
        };
    }

    pub fn find_last_ask(&self) -> Option<(&OrderBookKey, &OrderBookValue)> {
        return match self.asks.book.iter().next_back() {
            None => None,
            Some(v) => Some(v.clone()),
        };
    }

    pub fn find_last_bid(&self) -> Option<(&OrderBookKey, &OrderBookValue)> {
        return match self.bids.book.iter().next() {
            None => None,
            Some(v) => Some(v.clone()),
        };
    }

    fn exchange_check(&mut self, exchange: Exchange) {
        if self.exchange == Exchange::None {
            self.exchange = exchange
        } else if self.exchange != exchange {
            panic!("Updated OB locked to a different exchange");
        }
    }

    pub fn binance_update_best_ticker(&mut self, update: BestLevel) {
        self.exchange_check(Exchange::Binance);

        let cull_time = update.transaction_time - self.culling_threshold;
        let bid_price_check = self.tops.best_bid.0 != update.bid_price;
        let ask_price_check = self.tops.best_ask.0 != update.ask_price;
        let mut bid_up = false;
        let mut ask_up = false;

        if bid_price_check || self.tops.best_bid.1 != update.bid_qty {
            bid_up = true;
            self.tops.best_bid = (update.bid_price, update.bid_qty);
            if bid_price_check {
                // self.tops.bid_price_stats.add(update.transaction_time, update.bid_price);
                // self.tops.bid_price_stats.prune(cull_time);
                // self.tops.bid_qty_stats = RegularStats::new();
            }
            // self.tops.bid_qty_stats.add(update.transaction_time, update.bid_qty);
            // self.tops.bid_qty_stats.prune(cull_time);
        }
        if ask_price_check || self.tops.best_ask.1 != update.ask_qty {
            ask_up = true;
            self.tops.best_ask = (update.ask_price, update.ask_qty);
            if ask_price_check {
                // self.tops.ask_price_stats.add(update.transaction_time, update.ask_price);
                // self.tops.ask_price_stats.prune(cull_time);
                // self.tops.ask_qty_stats = RegularStats::new();
            }
            // self.tops.ask_qty_stats.add(update.transaction_time, update.ask_qty);
            // self.tops.ask_qty_stats.prune(cull_time);
        }
        match (bid_up, ask_up) {
            (true, true) => self.tops.updated_last_tick = BinanceSide::Both,
            (true, false) => self.tops.updated_last_tick = BinanceSide::Buy,
            (false, true) => self.tops.updated_last_tick = BinanceSide::Sell,
            (false, false) => info!("possible desync gettings tops"),
        }

        self.tops.spread = self.tops.best_ask.0 - self.tops.best_bid.0;
        // self.tops.spread_stats.add(update.transaction_time, self.tops.spread);

        // info!("bid: in {}, {} | mod {}, {}\nask: in {}, {} | mod {},{}",
        //     update.bid_price, update.bid_qty, self.best_bid.0, self.best_bid.1,
        //     update.ask_price, update.ask_qty, self.best_ask.0, self.best_ask.1);
        
    }

    pub fn binance_refresh(&mut self, refresh: BookRefresh) {
        self.exchange_check(Exchange::Binance);

        for (price, quantity) in refresh.bids.iter() {
            match self.bids.book.entry(OrderBookKey { key: *price }) {
                Entry::Vacant(v) => {
                    // The problem with editing empties is we lose the sequence record
                    // So our next best guess is the master sequence
                    if refresh.last_update_id > self.last_sequence {
                        if *quantity != D128::ZERO {
                            v.insert(OrderBookValue {
                                volume: *quantity,
                                liquidity: *price * *quantity,
                                timestamp: refresh.transaction_time,
                                sequence: refresh.last_update_id,
                            });
                        }
                    } else if refresh.last_update_id < self.last_sequence {
                        debug!("old order");
                    }
                },
                Entry::Occupied(mut o) => {
                    let entry = o.get_mut();
                    if refresh.last_update_id > entry.sequence {
                        if *quantity != D128::ZERO {
                            entry.volume = *quantity;
                            entry.liquidity = *price * *quantity;
                            entry.timestamp = refresh.transaction_time;
                            entry.sequence = refresh.last_update_id;
                        } else {
                            o.remove();
                        }
                    } else if refresh.last_update_id < entry.sequence {
                        debug!("old order");
                    }
                },
            }
        }

        for (price, quantity) in refresh.asks.iter() {
            match self.asks.book.entry(OrderBookKey { key: *price }) {
                Entry::Vacant(v) => {
                    // The problem with editing empties is we lose the sequence record
                    // So our next best guess is the master sequence
                    if refresh.last_update_id > self.last_sequence {
                        if *quantity != D128::ZERO {
                            v.insert(OrderBookValue {
                                volume: *quantity,
                                liquidity: *price * *quantity,
                                timestamp: refresh.transaction_time,
                                sequence: refresh.last_update_id,
                            });
                        }
                    } else if refresh.last_update_id < self.last_sequence {
                        debug!("old order");
                    }
                },
                Entry::Occupied(mut o) => {
                    let entry = o.get_mut();
                    if refresh.last_update_id > entry.sequence {
                        if *quantity != D128::ZERO {
                            entry.volume = *quantity;
                            entry.liquidity = *price * *quantity;
                            entry.timestamp = refresh.transaction_time;
                            entry.sequence = refresh.last_update_id;
                        } else {
                            o.remove();
                        }
                    } else if refresh.last_update_id < entry.sequence {
                        debug!("old order");
                    }
                },
            }
        }

        self.initialized = true;

        if self.last_sequence < refresh.last_update_id { self.last_sequence = refresh.last_update_id; }
    }

    pub fn binance_update(&mut self, update: Orders) {
        self.exchange_check(Exchange::Binance);

        if !(update.first_update_id <= self.last_sequence && update.last_update_id >= self.last_sequence) {
            // We need to refresh the Orderbook to catch missing pieces
        }

        for (price, quantity) in update.bids.iter() {
            match self.bids.book.entry(OrderBookKey { key: *price }) {
                Entry::Vacant(v) => {
                    // The problem with editing empties is we lose the sequence record
                    // So our next best guess is the master sequence
                    if update.last_update_id > self.last_sequence {
                        if *quantity != D128::ZERO {
                            v.insert(OrderBookValue {
                                volume: *quantity,
                                liquidity: *price * *quantity,
                                timestamp: update.transaction_time,
                                sequence: update.last_update_id,
                            });
                        }
                    } else if update.last_update_id < self.last_sequence {
                        debug!("old order");
                    }
                },
                Entry::Occupied(mut o) => {
                    let entry = o.get_mut();
                    if update.last_update_id > entry.sequence {
                        if *quantity != D128::ZERO {
                            entry.volume = *quantity;
                            entry.liquidity = *price * *quantity;
                            entry.timestamp = update.transaction_time;
                            entry.sequence = update.last_update_id;
                        } else {
                            o.remove();
                        }
                    } else if update.last_update_id < entry.sequence {
                        debug!("old order");
                    }
                },
            }
        }

        for (price, quantity) in update.asks.iter() {
            match self.asks.book.entry(OrderBookKey { key: *price }) {
                Entry::Vacant(v) => {
                    // The problem with editing empties is we lose the sequence record
                    // So our next best guess is the master sequence
                    if update.last_update_id > self.last_sequence {
                        if *quantity != D128::ZERO {
                            v.insert(OrderBookValue {
                                volume: *quantity,
                                liquidity: *price * *quantity,
                                timestamp: update.transaction_time,
                                sequence: update.last_update_id,
                            });
                        }
                    } else if update.last_update_id < self.last_sequence {
                        debug!("old order");
                    }
                },
                Entry::Occupied(mut o) => {
                    let entry = o.get_mut();
                    if update.last_update_id > entry.sequence {
                        if *quantity != D128::ZERO {
                            entry.volume = *quantity;
                            entry.liquidity = *price * *quantity;
                            entry.timestamp = update.transaction_time;
                            entry.sequence = update.last_update_id;
                        } else {
                            o.remove();
                        }
                    } else if update.last_update_id < entry.sequence {
                        debug!("old order");
                    }
                },
            }
        }

        self.last_sequence = update.last_update_id;
    }

    pub fn bybit_update(&mut self, update: OBTickData, sequence: u64, timestamp: u64) {
        self.exchange_check(Exchange::Bybit);

        if sequence <= self.last_sequence {
            eprintln!("old sequence arrived");
            return;
        }
        // println!("timestamp: {}", timestamp);

        for d in update.delete.iter() {
            let price = D128::from_str(&d.price).expect(&format!(
                "problem parsing the string for price {} into a D128 for deletion",
                d.price
            ));
            if d.side == "Buy" {
                self.bids.remove(&OrderBookKey { key: price });
            } else if d.side == "Sell" {
                self.asks.remove(&OrderBookKey { key: price });
            } else {
                eprintln!(
                    "the side value of a delete level was not Buy or Sell, but {}",
                    d.side
                );
            }
        }

        for u in update.update.iter() {
            let price = D128::from_str(&u.price).expect(&format!(
                "problem parsing the string for price {} into a D128 for update",
                u.price
            ));
            let vol = D128::from_str(&u.size.to_string()).expect(&format!(
                "problem parsing the string for volume {} into a D128 for update",
                u.size
            ));
            if u.side == "Buy" {
                match self.bids.get_mut(&OrderBookKey { key: price }) {
                    Some(b) => {
                        b.volume = vol;
                        b.liquidity = price * vol;
                        // b.liquidity_stats.prune(timestamp - self.culling_threshold);
                        // b.liquidity_stats.add(timestamp, b.liquidity);
                        b.timestamp = timestamp;
                    }
                    None => {
                        eprintln!("An orderbook update went into a missing level: {}", price);
                        self.bids.set(
                            OrderBookKey { key: price },
                            OrderBookValue {
                                volume: vol,
                                liquidity: vol * price,
                                // liquidity_stats: RegularStats::init(timestamp, vol * price),
                                timestamp,
                                sequence,
                            },
                        );
                    }
                }
            } else if u.side == "Sell" {
                match self.asks.get_mut(&OrderBookKey { key: price }) {
                    Some(a) => {
                        a.volume = vol;
                        a.liquidity = price * vol;
                        // a.liquidity_stats.prune(timestamp - self.culling_threshold);
                        // a.liquidity_stats.add(timestamp, a.liquidity);
                        a.timestamp = timestamp;
                    }
                    None => {
                        eprintln!("An orderbook update went into a missing level: {}", price);
                        self.asks.set(
                            OrderBookKey { key: price },
                            OrderBookValue {
                                volume: vol,
                                liquidity: vol * price,
                                // liquidity_stats: RegularStats::init(timestamp, vol * price),
                                timestamp,
                                sequence,
                            },
                        );
                    }
                }
            } else {
                eprintln!(
                    "the side value of an update level was not Buy or Sell, but {}",
                    u.side
                );
            }
        }

        for i in update.insert.iter() {
            let price = D128::from_str(&i.price).expect(&format!(
                "problem parsing the string for price {} into a D128 for deletion",
                i.price
            ));
            let vol = D128::from_str(&i.size.to_string()).expect(&format!(
                "problem parsing the string for volume {} into a D128 for update",
                i.size
            ));
            if i.side == "Buy" {
                self.bids.set(
                    OrderBookKey { key: price },
                    OrderBookValue {
                        volume: vol,
                        liquidity: vol * price,
                        // liquidity_stats: RegularStats::init(timestamp, vol * price),
                        // total_liquidity: D128::ZERO,
                        timestamp,
                        sequence,
                    },
                );
            } else if i.side == "Sell" {
                self.asks.set(
                    OrderBookKey { key: price },
                    OrderBookValue {
                        volume: vol,
                        liquidity: vol * price,
                        // liquidity_stats: RegularStats::init(timestamp, vol * price),
                        timestamp,
                        sequence,
                    },
                );
            } else {
                eprintln!(
                    "the side value of an insert level was not Buy or Sell, but {}",
                    i.side
                );
            }
        }

        self.last_sequence = sequence;

        // println!(
        //     "worst bid: {}, worst ask: {}",
        //     self.find_last_bid().expect("msg").0.key,
        //     self.find_last_ask().expect("msg").0.key
        // );

        // println!(
        //     "best ask: {}\nbest bid: {}",
        //     self.find_best_ask().expect("couldn't get a best ask ").key,
        //     self.find_best_bid().expect("couldn't get a best bid ").key
        // );
    }
}
