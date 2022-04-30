use crate::backend::{types::Side, binance::types::BinanceSide};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stage {
    Entry,
    Exit,
}

impl From<bool> for Stage {
    fn from(reduce_only: bool) -> Self {
        match reduce_only {
            true => Stage::Exit,
            false => Stage::Entry,
        }
    }
}

impl Stage {
    pub fn from_binance_side(side: BinanceSide, position_side: BinanceSide) -> Self {
        match position_side {
            BinanceSide::Buy => match side {
                BinanceSide::Buy => Stage::Entry,
                BinanceSide::Sell => Stage::Exit,
                BinanceSide::Both => todo!(),
            },
            BinanceSide::Sell => match side {
                BinanceSide::Buy => Stage::Exit,
                BinanceSide::Sell => Stage::Entry,
                BinanceSide::Both => todo!(),
            },
            BinanceSide::Both => todo!(),
        }
    }

    /// Helps you aggress
    pub fn aggress<'a, T>(&self, ingress: &'a T, egress: &'a T) -> &'a T {
        match self { Stage::Entry => ingress, Stage::Exit => egress, }
    }

    /// Mutable helper
    pub fn aggress_mut<'a, T>(&self, ingress: &'a mut T, egress: &'a mut T) -> &'a mut T {
        match self { Stage::Entry => ingress, Stage::Exit => egress, }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OrderClassification {
    Top,
    Rebase,
    Exit,
    None,
}