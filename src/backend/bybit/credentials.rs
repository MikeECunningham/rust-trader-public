
/// Stores credentials needed to manage a bybit account
pub struct BybitCredentials {
    /// Authentication key for bybit
    pub bybit_key: String,
    /// Authentication secret for bybit
    pub bybit_secret: String,
    /// URL for the public perpetuals stream
    pub bybit_perpetuals_url: String,
    /// URL for the private perpetuals stream
    pub bybit_perpetuals_private_url: String,
    /// URL for the rest API
    pub bybit_rest_url: String
}