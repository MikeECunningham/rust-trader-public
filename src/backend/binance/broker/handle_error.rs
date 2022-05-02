use crate::backend::binance::errors::FilterOtherErrors;
use crate::backend::binance::errors::ProcessingErrors;
use crate::backend::binance::errors::RequestErrors;
use crate::backend::binance::errors::ServerNetworkErrors;
use crate::backend::binance::types::BinanceError;
use crate::backend::binance::errors::ErrorCode::*;
use crate::backend::binance::errors::ServerNetworkErrors::*;
use crate::backend::binance::errors::RequestErrors::*;
use crate::backend::binance::errors::ProcessingErrors::*;
use crate::backend::binance::errors::FilterOtherErrors::*;

use super::BROKER;
use super::Broker;

impl Broker {
    pub fn error(&self, error: &BinanceError) {
        info!("{:?}", error);
        match error.code {
            ServerNetworkErrors(sne) => self.server_network_error(sne, &error.msg),
            RequestErrors(re) => self.request_error(re, &error.msg),
            ProcessingErrors(pe) => self.processing_error(pe, &error.msg),
            FilterOtherErrors(foe) => self.filter_other_error(foe, &error.msg),
        };
    }

    fn server_network_error(&self, sne: ServerNetworkErrors, msg: &str) {
        match sne {
            Unknown => todo!(),
            Disconnected => todo!(),
            Unauthorized => todo!(),
            TooManyRequests => todo!(),
            DuplicateIp => todo!(),
            NoSuchIp => todo!(),
            UnexpectedResponse => todo!(),
            Timeout => todo!(),
            ErrorMessageReceived => todo!(),
            NonWhiteList => todo!(),
            InvalidMessage => todo!(),
            UnknownOrderComposition => todo!(),
            TooManyOrders => todo!(),
            ServiceShuttingDown => todo!(),
            UnsupportedOperation => todo!(),
            InvalidTimestamp => {
                match msg.contains("1000ms ahead") {
                    true => BROKER.set_server_offset(-1000).unwrap(),
                    false => todo!(),
                }
            },
            InvalidSignature => todo!(),
            StartTimeGreaterThanEndTime => todo!(),
        };
    }

    fn request_error(&self, re: RequestErrors, msg: &str) {
        match re {
            IllegalChars => todo!(),
            TooManyParameters => todo!(),
            MandatoryParamEmptyOrMalformed => todo!(),
            UnknownParam => todo!(),
            UnreadParameters => todo!(),
            ParamEmpty => todo!(),
            ParamNotRequired => todo!(),
            BadAsset => todo!(),
            BadAccount => todo!(),
            BadInstrumentType => todo!(),
            BadPrecision => todo!(),
            NoDepth => todo!(),
            WithdrawNotNegative => todo!(),
            TIFNotRequired => todo!(),
            InvalidTIF => todo!(),
            InvalidOrderType => todo!(),
            InvalidSide => todo!(),
            EmptyNewClientOrderId => todo!(),
            EmptyOriginalClientOrderId => todo!(),
            BadInterval => todo!(),
            BadSymbol => todo!(),
            InvalidListenKey => todo!(),
            LookupIntervalTooBig => todo!(),
            OptionalParamsBadCombo => todo!(),
            InvalidParameter => todo!(),
            InvalidNewOrderResponseType => todo!(),
        };
    }

    fn processing_error(&self, pe: ProcessingErrors, msg: &str) {
        match pe {
            NewOrderRejected => todo!(),
            CancelRejected => { /*debug!("Cancel Rejected, hopefully this was because of a fill")*/},
            NoSuchOrder => todo!(),
            BadApiKeyFormat => todo!(),
            RejectedMBXKey => todo!(),
            NoTradingWindow => todo!(),
            BalanceNotSufficient => todo!(),
            MarginNotSufficient => todo!(),
            UnableToFill => todo!(),
            OrderWouldImmediatelyTrigger => todo!(),
            ReduceOnlyRejected => {},
            UserInLiquidation => todo!(),
            PositionNotSufficient => todo!(),
            MaxOpenOrderExceeded => todo!(),
            ReduceOnlyOrderTypeNotSupported => todo!(),
            MaxLeverageRatio => todo!(),
            MinLeverageRatio => todo!(),
        };
    }

    fn filter_other_error(&self, foe: FilterOtherErrors, msg: &str) {
        match foe {
            InvalidOrderStatus => todo!(),
            PriceLessThanZero => todo!(),
            PriceGreaterThanMax => todo!(),
            QuantityLessThanZero => todo!(),
            QuantityLessThanMin => todo!(),
            QuantityGreaterThanMax => todo!(),
            StopPriceLessThanZero => todo!(),
            StopPriceGreaterThanMax => todo!(),
            TickSizeLessThanZero => todo!(),
            MaxPriceLessThanMinPrice => todo!(),
            MaxQuantityLessThanMinQuantity => todo!(),
            StepSizeLessThanZero => todo!(),
            MaxNumberOfOrdersLessThanZero => todo!(),
            PriceLessThanMinPrice => todo!(),
            PriceNotIncreasedByTickSize => todo!(),
            InvalidClientOrderIdLength => todo!(),
            PriceHigherThanMarkMultiplierCap => todo!(),
            MultiplierUpLessThanZero => todo!(),
            MultiplierDownLessThanZero => todo!(),
            CompositeScaleOverflow => todo!(),
            TargetStrategyInvalid => todo!(),
            InvalidDepthLimit => todo!(),
            WrongMarketStatus => todo!(),
            QuantityNotIncreasedByStepSize => todo!(),
            PriceLowerThanMarkMultiplierFloor => todo!(),
            MultiplierDecimalLessThanZero => todo!(),
            CommissionInvalid => todo!(),
            InvalidAccountType => todo!(),
            InvalidLeverage => todo!(),
            InvalidTickSizePrecision => todo!(),
            InvalidStepSizePrecision => todo!(),
            InvalidWorkingType => todo!(),
            ExceedMaxCancelOrderSize => todo!(),
            InsuranceAccountNotFound => todo!(),
            InvalidBalanceType => todo!(),
            MaxStopOrderExceeded => todo!(),
            NoNeedToChangeMarginType => todo!(),
            ThereExistsOpenOrders => todo!(),
            ThereExistsQuantity => todo!(),
            AddIsolatedMarginReject => todo!(),
            CrossBalanceInsufficient => todo!(),
            IsolatedBalanceInsufficient => todo!(),
            NoNeedToChangeAutoAddMargin => todo!(),
            AutoAddCrossedMarginReject => todo!(),
            AddIsolatedMarginNoPositionReject => todo!(),
            AmountMustBePosition => todo!(),
            InvalidApiKeyType => todo!(),
            InvalidRsaPublicKey => todo!(),
            MaxPriceTooLarge => todo!(),
            NoNeedToChangePositionSide => todo!(),
            InvalidPositionSide => todo!(),
            PositionSideNotMatch => todo!(),
            ReduceOnlyConflict => todo!(),
            InvalidOptionsRequestType => todo!(),
            InvalidOptionsTimeFrame => todo!(),
            InvalidOptionsAmount => todo!(),
            InvalidOptionsEventType => todo!(),
            PositionSideChangeExistsOpenOrders => todo!(),
            PositionSideChangeExistsQuantity => todo!(),
            InvalidOptionsPremiumFee => todo!(),
            InvalidClientOptionsIdLength => todo!(),
            InvalidOptionsDirection => todo!(),
            OptionsPremiumNotUpdated => todo!(),
            OptionsPremiumInputLessThanZero => todo!(),
            OptionsAmountBiggerThanUpper => todo!(),
            OptionsPremiumOutputZero => todo!(),
            OptionsPremiumTooDiff => todo!(),
            OptionsPremiumReachLimit => todo!(),
            OptionsCommonError => todo!(),
            InvalidOptionsId => todo!(),
            OptionsUserNotFound => todo!(),
            OptionsNotFound => todo!(),
            InvalidBatchPlaceOrderSize => todo!(),
            PlaceBatchOrdersFail => todo!(),
            UpcomingMethod => todo!(),
            InvalidNotionalLimitCoef => todo!(),
            InvalidPriceSpreadThreshold => todo!(),
            ReduceOnlyOrderPermission => todo!(),
            NoPlaceOrderPermission => todo!(),
            InvalidContractType => todo!(),
            InvalidClientTranIdLength => todo!(),
            DuplicatedClientTranId => todo!(),
            ReduceOnlyMarginCheckFailed => todo!(),
            MarketOrderReject => todo!(),
            InvalidActivationPrice => todo!(),
            QuantityExistsWithClosePosition => todo!(),
            ReduceOnlyMustBeTrue => todo!(),
            OrderTypeCannotBeMarket => todo!(),
            InvalidOpeningPositionStatus => todo!(),
            SymbolAlreadyClosed => todo!(),
            StrategyInvalidTriggerPrice => todo!(),
            InvalidPair => todo!(),
            IsolatedLeverageRejectWithPosition => todo!(),
            MinNotional => todo!(),
            InvalidTimeInterval => todo!(),
            PriceHigherThanStopMultiplierUp => todo!(),
            PriceLowerThanStopMultiplierDown => todo!(),
        };
    }
}