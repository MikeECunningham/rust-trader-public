use std::{ops::Not, fmt::{Display, Formatter}};

use serde::{Deserialize, Serialize};

use super::binance::types::BinanceSide;

/// For more global types




#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum TimeInForce {
    #[serde(alias="GTC")]
    GoodTillCancel,
    #[serde(alias="IOC")]
    ImmediateOrCancel,
    #[serde(alias="FOK")]
    FillOrKill,
    #[serde(alias="GTX")]
    GoodTillCrossing,
    #[serde(alias="PO")]
    PostOnly,
}

impl TryFrom<String> for TimeInForce {
    type Error = &'static str;
    fn try_from(time_in_force: String) -> Result<Self, Self::Error> {
        let time_in_force = time_in_force.to_lowercase();
        if time_in_force == "goodtillcancel" {
            Ok(TimeInForce::GoodTillCancel)
        } else if time_in_force == "immediateorcancel" {
            Ok(TimeInForce::ImmediateOrCancel)
        } else if time_in_force == "fillorkill" {
            Ok(TimeInForce::FillOrKill)
        } else if time_in_force == "postonly" {
            Ok(TimeInForce::PostOnly)
        } else {
            Err("Invalid Data: time_in_force must match a permutation of the TimeInForce enum")
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Exchange {
    Alpaca,
    Binance,
    Bybit,
    None,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
pub enum Side {
    #[serde(alias="BUY")]
    #[serde(alias="buy")]
    #[serde(alias="LONG")]
    #[serde(alias="Long")]
    #[serde(alias="long")]
    Buy,
    #[serde(alias="SELL")]
    #[serde(alias="sell")]
    #[serde(alias="SHORT")]
    #[serde(alias="Short")]
    #[serde(alias="short")]
    Sell,
}

impl Not for Side {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", match self { Side::Buy => "Buy", Side::Sell => "Sell", })
    }
}

impl TryFrom<BinanceSide> for Side {
    type Error = &'static str;
    
    fn try_from(binance_side: BinanceSide) -> Result<Self, Self::Error> {
        match binance_side {
            BinanceSide::Buy => Ok(Side::Buy),
            BinanceSide::Sell => Ok(Side::Sell),
            BinanceSide::Both => Err("Invalid Data: Side is only buy or sell, BOTH has no permutation"),
        }
    }
}

impl TryFrom<String> for Side {
    type Error = &'static str;

    fn try_from(side: String) -> Result<Self, Self::Error> {
        let side = side.to_lowercase();
        if side == "buy" || side == "long" || side == "bid" {
            Ok(Side::Buy)
        } else if side == "sell" || side == "short" || side == "ask" {
            Ok(Side::Sell)
        } else {
            Err("Invalid Data: side must match a permutation of the Side enum")
        }
    }
}

impl Side {
    /** Helps you deside
    * To avoid lifetime muck, it's best to use this when you can immediately consume the response
    */
    pub fn deside<'a, T>(&self, buy: &'a T, sell: &'a T) -> &'a T {
        match self { Side::Buy => buy, Side::Sell => sell, }
    }
}