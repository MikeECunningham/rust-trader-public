use serde::Deserialize;
use serde_repr::Deserialize_repr;

#[derive(Debug, Deserialize_repr, PartialEq, Clone, Copy)]
#[repr(i32)]
pub enum ServerNetworkErrors {
    /// An unknown error occured while processing the request.
    Unknown = -1000,
    /// Internal error; unable to process your request. Please try again.
    Disconnected = -1001,
    /// You are not authorized to execute this request.
    Unauthorized = -1002,
    /// Too many requests queued.
    /// Too many requests; please use the websocket for live updates.
    /// Too many requests; current limit is %s requests per minute. Please use the websocket for live updates to avoid polling the API.
    /// Way too many requests; IP banned until %s. Please use the websocket for live updates to avoid bans.
    TooManyRequests = -1003,
    /// This IP is already on the white list
    DuplicateIp = -1004,
    /// No such IP has been white listed
    NoSuchIp = -1005,
    /// An unexpected response was received from the message bus. Execution status unknown.
    UnexpectedResponse = -1006,
    /// Timeout waiting for response from backend server. Send status unknown; execution status unknown.
    Timeout = -1007,
    /// ERROR_MSG_RECEIVED.
    ErrorMessageReceived = -1010,
    /// This IP cannot access this route.
    NonWhiteList = -1011,
    /// INVALID_MESSAGE.
    InvalidMessage = -1013,
    /// Unsupported order combination.
    UnknownOrderComposition = -1014,
    /// Too many new orders.
    /// Too many new orders; current limit is %s orders per %s.
    TooManyOrders = -1015,
    /// This service is no longer available.
    ServiceShuttingDown = -1016,
    /// This operation is not supported.
    UnsupportedOperation = -1020,
    /// Timestamp for this request is outside of the recvWindow.
    /// Timestamp for this request was 1000ms ahead of the server's time.
    InvalidTimestamp = -1021,
    /// Signature for this request is not valid.
    InvalidSignature = -1022,
    /// Start time is greater than end time.
    StartTimeGreaterThanEndTime = -1023,
}

#[derive(Debug, Deserialize_repr, PartialEq, Clone, Copy)]
#[repr(i32)]
pub enum RequestErrors {
    /// Illegal characters found in a parameter.
    /// Illegal characters found in parameter '%s'; legal range is '%s'.
    IllegalChars = -1100,
    /// Too many parameters sent for this endpoint.
    /// Too many parameters; expected '%s' and received '%s'.
    /// Duplicate values for a parameter detected.
    TooManyParameters = -1101,
    /// A mandatory parameter was not sent, was empty/null, or malformed.
    /// Mandatory parameter '%s' was not sent, was empty/null, or malformed.
    /// Param '%s' or '%s' must be sent, but both were empty/null!
    MandatoryParamEmptyOrMalformed = -1102,
    /// An unknown parameter was sent.
    UnknownParam = -1103,
    /// Not all sent parameters were read.
    /// Not all sent parameters were read; read '%s' parameter(s) but was sent '%s'.
    UnreadParameters = -1104,
    /// A parameter was empty.
    /// Parameter '%s' was empty.
    ParamEmpty = -1105,
    /// A parameter was sent when not required.
    /// Parameter '%s' sent when not required.
    ParamNotRequired = -1106,
    /// Invalid asset.
    BadAsset = -1108,
    /// Invalid account.
    BadAccount = -1109,
    /// Invalid symbolType.
    BadInstrumentType = -1110,
    /// Precision is over the maximum defined for this asset.
    BadPrecision = -1111,
    /// No orders on book for symbol.
    NoDepth = -1112,
    /// Withdrawal amount must be negative.
    WithdrawNotNegative = -1113,
    /// TimeInForce parameter sent when not required.
    TIFNotRequired = -1114,
    /// Invalid timeInForce.
    InvalidTIF = -1115,
    /// Invalid orderType.
    InvalidOrderType = -1116,
    /// Invalid side.
    InvalidSide = -1117,
    /// New client order ID was empty.
    EmptyNewClientOrderId = -1118,
    /// Original client order ID was empty.
    EmptyOriginalClientOrderId = -1119,
    /// Invalid interval.
    BadInterval = -1120,
    /// Invalid symbol.
    BadSymbol = -1121,
    /// This listenKey does not exist.
    InvalidListenKey = -1125,
    /// Lookup interval is too big.
    /// More than %s hours between startTime and endTime.
    LookupIntervalTooBig = -1127,
    /// Combination of optional parameters invalid.
    OptionalParamsBadCombo = -1128,
    /// Invalid data sent for a parameter.
    /// Data sent for parameter '%s' is not valid.
    InvalidParameter = -1130,
    /// Invalid newOrderRespType.
    InvalidNewOrderResponseType = -1136,
}

#[derive(Debug, Deserialize_repr, PartialEq, Clone, Copy)]
#[repr(i32)]
pub enum ProcessingErrors {
    /// NEW_ORDER_REJECTED
    NewOrderRejected = -2010,
    /// CANCEL_REJECTED
    CancelRejected = -2011,
    /// Order does not exist.
    NoSuchOrder = -2013,
    /// API-key format invalid.
    BadApiKeyFormat = -2014,
    /// Invalid API-key, IP, or permissions for action.
    RejectedMBXKey = -2015,
    /// No trading window could be found for the symbol. Try ticker/24hrs instead.
    NoTradingWindow = -2016,
    /// Balance is insufficient.
    BalanceNotSufficient = -2018,
    /// Margin is insufficient.
    MarginNotSufficient = -2019,
    /// Unable to fill.
    UnableToFill = -2020,
    /// Order would immediately trigger.
    OrderWouldImmediatelyTrigger = -2021,
    /// ReduceOnly Order is rejected.
    ReduceOnlyRejected = -2022,
    /// User in liquidation mode now.
    UserInLiquidation = -2023,
    /// Position is not sufficient.
    PositionNotSufficient = -2024,
    /// Reach max open order limit.
    MaxOpenOrderExceeded = -2025,
    /// This OrderType is not supported when reduceOnly.
    ReduceOnlyOrderTypeNotSupported = -2026,
    /// Exceeded the maximum allowable position at current leverage.
    MaxLeverageRatio = -2027,
    /// Leverage is smaller than permitted: insufficient margin balance.
    MinLeverageRatio = -2028,
}

#[derive(Debug, Deserialize_repr, PartialEq, Clone, Copy)]
#[repr(i32)]
pub enum FilterOtherErrors {
    /// 
    InvalidOrderStatus = -4000,
    /// 
    PriceLessThanZero = -4001,
    /// 
    PriceGreaterThanMax = -4002,
    /// 
    QuantityLessThanZero = -4003,
    /// 
    QuantityLessThanMin = -4004,
    /// 
    QuantityGreaterThanMax = -4005,
    /// 
    StopPriceLessThanZero = -4006,
    /// 
    StopPriceGreaterThanMax = -4007,
    /// 
    TickSizeLessThanZero = -4008,
    /// 
    MaxPriceLessThanMinPrice = -4009,
    /// 
    MaxQuantityLessThanMinQuantity = -4010,
    /// 
    StepSizeLessThanZero = -4011,
    /// 
    MaxNumberOfOrdersLessThanZero = -4012,
    /// 
    PriceLessThanMinPrice = -4013,
    /// 
    PriceNotIncreasedByTickSize = -4014,
    /// 
    InvalidClientOrderIdLength = -4015,
    /// 
    PriceHigherThanMarkMultiplierCap = -4016,
    /// 
    MultiplierUpLessThanZero = -4017,
    /// 
    MultiplierDownLessThanZero = -4018,
    /// 
    CompositeScaleOverflow = -4019,
    /// 
    TargetStrategyInvalid = -4020,
    /// 
    InvalidDepthLimit = -4021,
    /// 
    WrongMarketStatus = -4022,
    /// 
    QuantityNotIncreasedByStepSize = -4023,
    /// 
    PriceLowerThanMarkMultiplierFloor = -4024,
    /// 
    MultiplierDecimalLessThanZero = -4025,
    /// 
    CommissionInvalid = -4026,
    /// 
    InvalidAccountType = -4027,
    /// 
    InvalidLeverage = -4028,
    /// 
    InvalidTickSizePrecision = -4029,
    /// 
    InvalidStepSizePrecision = -4030,
    /// 
    InvalidWorkingType = -4031,
    /// 
    ExceedMaxCancelOrderSize = -4032,
    /// 
    InsuranceAccountNotFound = -4033,
    /// 
    InvalidBalanceType = -4044,
    /// 
    MaxStopOrderExceeded = -4045,
    /// 
    NoNeedToChangeMarginType = -4046,
    /// 
    ThereExistsOpenOrders = -4047,
    /// 
    ThereExistsQuantity = -4048,
    /// 
    AddIsolatedMarginReject = -4049,
    /// 
    CrossBalanceInsufficient = -4050,
    /// 
    IsolatedBalanceInsufficient = -4051,
    /// 
    NoNeedToChangeAutoAddMargin = -4052,
    /// 
    AutoAddCrossedMarginReject = -4053,
    /// 
    AddIsolatedMarginNoPositionReject = -4054,
    /// 
    AmountMustBePosition = -4055,
    /// 
    InvalidApiKeyType = -4056,
    /// 
    InvalidRsaPublicKey = -4057,
    /// 
    MaxPriceTooLarge = -4058,
    /// 
    NoNeedToChangePositionSide = -4059,
    /// 
    InvalidPositionSide = -4060,
    /// 
    PositionSideNotMatch = -4061,
    /// 
    ReduceOnlyConflict = -4062,
    /// 
    InvalidOptionsRequestType = -4063,
    /// 
    InvalidOptionsTimeFrame = -4064,
    /// 
    InvalidOptionsAmount = -4065,
    /// 
    InvalidOptionsEventType = -4066,
    /// 
    PositionSideChangeExistsOpenOrders = -4067,
    /// 
    PositionSideChangeExistsQuantity = -4068,
    /// 
    InvalidOptionsPremiumFee = -4069,
    /// 
    InvalidClientOptionsIdLength = -4070,
    /// 
    InvalidOptionsDirection = -4071,
    /// 
    OptionsPremiumNotUpdated = -4072,
    /// 
    OptionsPremiumInputLessThanZero = -4073,
    /// 
    OptionsAmountBiggerThanUpper = -4074,
    /// 
    OptionsPremiumOutputZero = -4075,
    /// 
    OptionsPremiumTooDiff = -4076,
    /// 
    OptionsPremiumReachLimit = -4077,
    /// 
    OptionsCommonError = -4078,
    /// 
    InvalidOptionsId = -4079,
    /// 
    OptionsUserNotFound = -4080,
    /// 
    OptionsNotFound = -4081,
    /// 
    InvalidBatchPlaceOrderSize = -4082,
    /// 
    PlaceBatchOrdersFail = -4083,
    /// 
    UpcomingMethod = -4084,
    /// 
    InvalidNotionalLimitCoef = -4085,
    /// 
    InvalidPriceSpreadThreshold = -4086,
    /// 
    ReduceOnlyOrderPermission = -4087,
    /// 
    NoPlaceOrderPermission = -4088,
    /// 
    InvalidContractType = -4104,
    /// 
    InvalidClientTranIdLength = -4114,
    /// 
    DuplicatedClientTranId = -4115,
    /// 
    ReduceOnlyMarginCheckFailed = -4118,
    /// 
    MarketOrderReject = -4131,
    /// 
    InvalidActivationPrice = -4135,
    /// 
    QuantityExistsWithClosePosition = -4137,
    /// 
    ReduceOnlyMustBeTrue = -4138,
    /// 
    OrderTypeCannotBeMarket = -4139,
    /// 
    InvalidOpeningPositionStatus = -4140,
    /// 
    SymbolAlreadyClosed = -4141,
    /// 
    StrategyInvalidTriggerPrice = -4142,
    /// 
    InvalidPair = -4144,
    /// 
    IsolatedLeverageRejectWithPosition = -4161,
    /// 
    MinNotional = -4164,
    /// 
    InvalidTimeInterval = -4165,
    /// 
    PriceHigherThanStopMultiplierUp = -4183,
    /// 
    PriceLowerThanStopMultiplierDown = -4184,
}

#[derive(Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(untagged)]
pub enum ErrorCode {
    ServerNetworkErrors(ServerNetworkErrors),
    RequestErrors(RequestErrors),
    ProcessingErrors(ProcessingErrors),
    FilterOtherErrors(FilterOtherErrors)
}