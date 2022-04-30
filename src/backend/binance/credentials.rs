
/// Stores credentials needed to manage a bybit account
pub struct BinanceCredentials {
    /// Authentication key for binance
    pub binance_key: String,
    /// Authentication secret for binance
    pub binance_secret: String,
    /// URL for the public perpetuals stream
    pub binance_perpetuals_url: String,
    /// URL for the rest API
    pub binance_rest_url: String
}