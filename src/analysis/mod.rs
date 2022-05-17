/*
* THIS FILE HAS HAD CONTENT REDACTED
*/

/**
 * With models generally being big, heavy list/map structures that can't implement Copy,
 * or otherwise be quickly sent across threads, it's best to process their data into
 * lightweight payloads here.
 */

use std::time::Instant;
use crate::orderbook::{OrderBook, OrderBookValue, Tops};
use crate::tradeflow::TradeFlow;
pub mod stats;

use dec::D128;

#[derive(Clone, Copy, Debug)]
pub struct BookResult {
    pub total_bid_liq: D128,
    pub total_ask_liq: D128,
    pub best_bid_volatility: D128,
    pub best_ask_volatility: D128,
    pub best_bid: (D128, OrderBookValue),
    pub best_ask: (D128, OrderBookValue),
    pub test_timer: Instant,
}

impl BookResult {
    pub fn new() -> BookResult {
        BookResult {
            best_bid_volatility: D128::NAN,
            best_ask_volatility: D128::NAN,
            total_ask_liq: D128::NAN,
            total_bid_liq: D128::NAN,
            best_bid: (D128::NAN, OrderBookValue::new()),
            best_ask: (D128::NAN, OrderBookValue::new()),
            test_timer: Instant::now(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TradeResult {
    pub new_level: (D128, D128),
    pub test_timer: Instant,
}

pub struct Analysis {}

impl Analysis {

    pub fn new_best(&mut self, tops: Tops) {}

    /// New book came in
    pub fn new_orderbook(orderbook: &OrderBook, tradeflow: &TradeFlow) -> BookResult {
        let mut result = BookResult::new();

        let best_bid = orderbook
            .find_best_bid()
            .expect("something went wrong finding best bid in analysis");
        let best_ask = orderbook
            .find_best_ask()
            .expect("something went wrong finding best ask in analysis");

        result.best_bid = (best_bid.0.key, *best_bid.1);
        result.best_ask = (best_ask.0.key, *best_ask.1);

        result
    }

    /// New trade came in
    pub fn new_trade(orderbook: &OrderBook, tradeflow: &TradeFlow) -> TradeResult {
        let mut result = TradeResult {
            new_level: (D128::NAN, D128::NAN),
            test_timer: Instant::now(),
        };

        for (key, value) in orderbook.asks.book.iter() {
            let price = key.key;
            if tradeflow.last_buy.liquidity.sum_product_vars < value.liquidity {
                result.new_level = (price, value.volume);
                break;
            }
        }
        for (key, value) in orderbook.bids.book.iter() {
            let price = key.key;
            if tradeflow.last_sell.liquidity.sum_product_vars < value.liquidity {
                result.new_level = (price, value.volume);
                break;
            }
        }

        result
    }
}