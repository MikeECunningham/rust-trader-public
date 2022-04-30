/// One day we'll use this
use std::{collections::HashMap};

use super::{Portfolio, strategy::Strategy};

use thiserror::Error;
// #[derive(Debug)]
pub struct Account {
    /// A mapping of asset pair names to the current state of the asset
    pub asset_pairs: HashMap<String, Strategy>,
}


impl Account {

    pub fn new() -> Account {
        Account {
            asset_pairs: HashMap::new(),
        }
    }
}
