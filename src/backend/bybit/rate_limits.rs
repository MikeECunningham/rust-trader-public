use std::{cmp, time::{SystemTimeError, UNIX_EPOCH, SystemTime}};

/// One minute rate limit interval
const LIMIT_DURATION: u64 = 60;

/// /private/linear/order/create
/// /private/linear/order/replace
/// /private/linear/order/cancel
/// /private/linear/order/cancel-all
/// /private/linear/stop-order/create
/// /private/linear/stop-order/replace
/// /private/linear/stop-order/cancel
/// /private/linear/stop-order/cancel-all
const ORDERS_LIMIT: u64 = 100;
/// /private/linear/position/set-leverage
/// /private/linear/position/switch-isolated
/// /private/linear/tpsl/switch-mode
/// /private/linear/position/set-auto-add-margin
/// /private/linear/position/trading-stop
/// /private/linear/position/add-margin
const LEVERAGE_MARGIN_LIMIT: u64 = 75;
/// /private/linear/position/list
/// /private/linear/trade/closed-pnl/list
/// /private/linear/trade/execution/list
const POSITION_LIMIT: u64 = 120;
/// /private/linear/order/list
/// /private/linear/order/
/// /private/linear/stop-order/list
/// /private/linear/stop-order/
const OPEN_ORDERS_LIMIT: u64 = 600;
/// /private/linear/funding/prev-funding
/// /private/linear/funding/predicted-funding
const FUNDING_LIMIT: u64 = 120;

/// GET Request 2 minute window
const GET_REQUEST_LONG_RATE_DURATION: u64 = 120;
const GET_REQUEST_LONG_RATE_LIMIT: u64 = 50;

/// GET Request 5 second window
const GET_REQUEST_SHORT_RATE_DURATION: u64 = 5;
const GET_REQUEST_SHORT_RATE_LIMIT: u64 = 70;


/// POST Request 2 minute window
const POST_REQUEST_LONG_RATE_DURATION: u64 = 120;
const POST_REQUEST_LONG_RATE_LIMIT: u64 = 20;

/// POST Request 5 second window
const POST_REQUEST_SHORT_RATE_DURATION: u64 = 5;
const POST_REQUEST_SHORT_RATE_LIMIT: u64 = 50;

#[derive(Debug)]
pub struct IPLimits {
    /// The last time a get request was sent
    ip_last_rate_limit_modify_get: u64,
    /// The last time a get request was sent
    ip_last_rate_limit_modify_post: u64,
    /// The long running value calculated at the last timestamp about how many get requests can be sent
    ip_rate_limit_status_get_long: u64,
    /// The long running value calculated at the last timestamp about how many post requests can be sent
    ip_rate_limit_status_post_long: u64,
    /// The short duration value calculated at the last timestamp about how many get requests can be sent
    ip_rate_limit_status_get_short: u64,
    /// The short duration value calculated at the last timestamp about how many post requests can be sent
    ip_rate_limit_status_post_short: u64,
}

impl IPLimits {

    pub fn new() -> Self{
        IPLimits {
            ip_last_rate_limit_modify_get: 0,
            ip_last_rate_limit_modify_post: 0,
            ip_rate_limit_status_get_long: GET_REQUEST_LONG_RATE_DURATION * GET_REQUEST_LONG_RATE_LIMIT,
            ip_rate_limit_status_post_long: POST_REQUEST_LONG_RATE_DURATION * POST_REQUEST_LONG_RATE_LIMIT,
            ip_rate_limit_status_get_short: GET_REQUEST_SHORT_RATE_DURATION * GET_REQUEST_SHORT_RATE_LIMIT,
            ip_rate_limit_status_post_short: POST_REQUEST_SHORT_RATE_DURATION * POST_REQUEST_SHORT_RATE_LIMIT,
        }
    }

    /// Calculates a rolling rate limit for the current time window for the current ip address
    pub fn check_get_ip_rate_limit(&mut self, cost: u64) -> Result<bool, SystemTimeError> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        // Number of seconds since last get rate limit interpolation
        let seconds_passed = timestamp - self.ip_last_rate_limit_modify_get;
        
        // The new limit of how many requests the broker may have over a 2 minute window
        self.ip_rate_limit_status_get_long = cmp::min(
            self.ip_rate_limit_status_get_long + (seconds_passed * GET_REQUEST_LONG_RATE_LIMIT), 
            GET_REQUEST_LONG_RATE_DURATION * GET_REQUEST_LONG_RATE_LIMIT
        );

        // The new limit of how many requests the broker may have over a 5 second window
        self.ip_rate_limit_status_get_short = cmp::min(
            self.ip_rate_limit_status_get_short + (seconds_passed * GET_REQUEST_SHORT_RATE_LIMIT), 
            GET_REQUEST_SHORT_RATE_DURATION * GET_REQUEST_SHORT_RATE_LIMIT
        );

        // Update the last update time to the current time
        self.ip_last_rate_limit_modify_get = timestamp;

        // Lastly if we can afford the cost and return Ok
        return Ok(cost < self.ip_rate_limit_status_get_long && cost < self.ip_rate_limit_status_get_short);
    }

    /// When calling this function, make sure you call check_get_ip_rate_limit first to
    /// ensure that the cost will not exceed the limit
    pub fn apply_get_ip_cost(&mut self, cost: u64) {
        self.ip_rate_limit_status_get_long -= cost;
        self.ip_rate_limit_status_get_short -= cost;
    }

    /// Calculates a rolling rate limit for the current time window for the current ip address
    pub fn check_post_ip_rate_limit(&mut self, cost: u64) -> Result<bool, SystemTimeError> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // Number of seconds since last get rate limit interpolation
        let seconds_passed = timestamp - self.ip_last_rate_limit_modify_post;

        // The new limit of how many requests the broker may have over a 2 minute window
        self.ip_rate_limit_status_post_long = cmp::min(
            self.ip_rate_limit_status_post_long + (seconds_passed * POST_REQUEST_LONG_RATE_LIMIT), 
            POST_REQUEST_LONG_RATE_DURATION * POST_REQUEST_LONG_RATE_LIMIT
        );

        // The new limit of how many requests the broker may have over a 5 second window
        self.ip_rate_limit_status_post_short = cmp::min(
            self.ip_rate_limit_status_post_short + (seconds_passed * POST_REQUEST_SHORT_RATE_LIMIT), 
            POST_REQUEST_SHORT_RATE_DURATION * POST_REQUEST_SHORT_RATE_LIMIT
        );

        // Update the last update time to the current time
        self.ip_last_rate_limit_modify_post = timestamp;

        // Lastly if we can afford the cost, subtract the cost and return Ok
        return Ok(cost < self.ip_rate_limit_status_post_long && cost < self.ip_rate_limit_status_post_short);
    }

    /// When calling this function, make sure you call check_post_ip_rate_limit first to
    /// ensure that the cost will not exceed the limit
    pub fn apply_post_ip_cost(&mut self, cost: u64) {
        self.ip_rate_limit_status_post_long -= cost;
        self.ip_rate_limit_status_post_short -= cost;
    }
}

#[derive(Debug)]
pub struct EndpointLimits {
    orders_limit: u64,
    leverage_margin_limit: u64,
    position_limit: u64,
    open_orders_limit: u64,
    funding_limit: u64,

    last_orders: u64,
    last_leverage_margin: u64,
    last_position: u64,
    last_open_orders: u64,
    last_funding: u64,
}

impl EndpointLimits {

    pub fn new() -> Self {
        EndpointLimits {
            orders_limit: ORDERS_LIMIT,
            leverage_margin_limit: LEVERAGE_MARGIN_LIMIT,
            position_limit: POSITION_LIMIT,
            open_orders_limit: OPEN_ORDERS_LIMIT,
            funding_limit: FUNDING_LIMIT,
            last_orders: 0,
            last_leverage_margin: 0,
            last_position: 0,
            last_open_orders: 0,
            last_funding: 0,
        }
    }



    fn check_rate_limit(&mut self, last_modify: u64, limit: u64, limit_ps: u64) -> Result<(u64, u64), SystemTimeError> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        // Number of seconds since last get rate limit interpolation
        let seconds_passed = timestamp - last_modify;
        
        // The new limit of how many requests the broker may have over a 2 minute window
        let new_limit = cmp::min(
            limit + (seconds_passed * limit_ps), 
            LIMIT_DURATION * limit_ps
        );

        return Ok((new_limit, timestamp));
    }

    pub fn check_orders_limit(&mut self, cost: u64) -> Result<bool, SystemTimeError> {
        let (orders_limit, last_orders) = self.check_rate_limit(self.last_orders, self.orders_limit, ORDERS_LIMIT)?;
        self.orders_limit = orders_limit;
        self.last_orders = last_orders;
        return Ok(cost < orders_limit);
    }

    pub fn check_leverage_margin_limit(&mut self, cost: u64) -> Result<bool, SystemTimeError> {
        let (leverage_margin_limit, last_leverage_margin) = self.check_rate_limit(self.last_orders, self.orders_limit, LEVERAGE_MARGIN_LIMIT)?;
        self.leverage_margin_limit = leverage_margin_limit;
        self.last_leverage_margin = last_leverage_margin;
        return Ok(cost < last_leverage_margin);
    }

    pub fn check_position_limit(&mut self, cost: u64) -> Result<bool, SystemTimeError> {
        let (position_limit, last_position) = self.check_rate_limit(self.last_position, self.position_limit, POSITION_LIMIT)?;
        self.position_limit = position_limit;
        self.last_position = last_position;
        return Ok(cost < position_limit);
    }

    pub fn check_open_orders_limit(&mut self, cost: u64) -> Result<bool, SystemTimeError> {
        let (open_orders_limit, last_open_orders) = self.check_rate_limit(self.last_open_orders, self.open_orders_limit, OPEN_ORDERS_LIMIT)?;
        self.open_orders_limit = open_orders_limit;
        self.last_open_orders = last_open_orders;
        return Ok(cost < open_orders_limit);
    }

    pub fn check_funding_limit(&mut self, cost: u64) -> Result<bool, SystemTimeError> {
        let (funding_limit, last_funding) = self.check_rate_limit(self.last_funding, self.funding_limit, FUNDING_LIMIT)?;
        self.funding_limit = funding_limit;
        self.last_funding = last_funding;
        return Ok(cost < funding_limit);
    }

    pub fn apply_order_cost(&mut self, cost: u64) {
        self.orders_limit -= cost;
    }

    pub fn apply_leverage_margin_cost(&mut self, cost: u64) {
        self.leverage_margin_limit -= cost;
    }

    pub fn apply_position_cost(&mut self, cost: u64) {
        self.position_limit -= cost;
    }

    pub fn apply_open_order_cost(&mut self, cost: u64) {
        self.open_orders_limit -= cost;
    }

    pub fn apply_funding_cost(&mut self, cost: u64) {
        self.funding_limit -= cost;
    }
}