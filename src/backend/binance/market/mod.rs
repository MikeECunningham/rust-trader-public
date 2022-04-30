pub mod orderbook;

use reqwest::{Client, Error};

pub struct Market {
    pub client: Client
}

impl Market {
    pub fn new() -> Result<Self, Error> {
        Ok(Market {
            client: reqwest::Client::builder().https_only(true).pool_max_idle_per_host(4).pool_idle_timeout(None).use_rustls_tls().build()?,
        })
    }
}

lazy_static! {
    pub static ref MARKET: Market = Market::new().expect("Failed to create market due to an issue building the request pool");
}