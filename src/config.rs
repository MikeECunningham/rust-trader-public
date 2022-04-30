/// This file contains a global instance of readonly values that can be used across the application.
/// 
/// Any fields added to the config struct will inform the application to attempt to load an environment
/// variable from the system by that name.
/// NOTE: The name of the environment variable it loads will be in all uppercase,
/// so 'example_key' becomes 'EXAMPLE_KEY' 
use serde::{Deserialize};


/// Describes configurations that originate from the applications environment
#[derive(Deserialize)]
pub struct Config {
    pub env: String,
    /// Authentication key for bybit
    pub bybit_key: String,
    /// Authentication secret for bybit
    pub bybit_secret: String,
    /// URL for the public perpetuals stream
    pub bybit_perpetuals_url: String,
    /// URL for the private perpetuals stream
    pub bybit_perpetuals_private_url: String,
    /// URL for the rest API
    pub bybit_rest_url: String,
    /// Authentication key for binance
    pub binance_key: String,
    /// Authentication secret for binance
    pub binance_secret: String,
    /// URL for the public perpetuals stream
    pub binance_perpetuals_url: String,
    /// URL for the rest API
    pub binance_rest_url: String,
    /// The style of running code
    pub execution_mode: Option<String>
}

lazy_static! {
    pub static ref CONFIG: Config = envy::from_env::<Config>().expect("Failed to load config from environment");
}