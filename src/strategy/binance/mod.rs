mod account;
mod portfolio;
mod order_list;
mod message;
mod order;
mod position;
pub mod strategy;

use dec::D128;

pub use self::account::*;
pub use self::order::*;
pub use self::position::*;
pub use self::message::*;
pub use self::portfolio::*;

lazy_static! {
    pub static ref REBATE: D128 = D128::from(0.00025);
    pub static ref MAX_OPEN_DIST: D128 = D128::from(30);
    pub static ref TOP_OPEN_DIST: D128 = D128::from(6);
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum StratBranch {
    /// No actives, no inventory
    NNN,
    /// No actives, have inventory
    NNS,
    /// No opens, active closes, no inventory; LIKELY ERROR STATE
    NSN,
    /// No opens, active closes, have inventory
    NSS,
    /// Active opens, no closes, no inventory
    SNN,
    /// Active opens, no closes, have inventory
    SNS,
    /// Active opens, active closes, no inventory; LIKELY ERROR STATE
    SSN,
    /// Active opens, active closes, have inventory
    SSS
}

impl From<(bool, bool, bool)> for StratBranch {
    fn from(opens_closes_inv: (bool, bool, bool)) -> Self {
        let opens = opens_closes_inv.0;
        let closes = opens_closes_inv.1;
        let inventory = opens_closes_inv.2;
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
}