use anchor_lang::prelude::error_code;

#[error_code]
pub enum MaxiFarmError {
    #[msg("Unauthorised")]
    Unauthorised,
    #[msg("Already became an owner")]
    AlreadyBecameOwner,

    #[msg("Invalid tax")]
    InvalidTax,
    #[msg("Invalid max. fee tokens")]
    InvalidMaxFeeTokens,
    #[msg("Invalid private sale period")]
    InvalidPrivSalePeriod,
    #[msg("Invalid trading fee")]
    InvalidTradingFee,

    #[msg("Invalid total supply")]
    InvalidTotalSupply,
    #[msg("Invalid initial virtual base reserves")]
    InvalidInitVirtBaseReserves,
    #[msg("Invalid initial virtual quote reserves")]
    InvalidInitVirtQuoteReserves,
    #[msg("Invalid real quote threshold")]
    InvalidRealQuoteThreshold,

    #[msg("Wrong base amount on creation")]
    WrongBaseAmountOnCreation,
    #[msg("Base token must not be mintable")]
    BaseTokenMustNotBeMintable,
    #[msg("Base token must not be freezable")]
    BaseTokenMustNotBeFreezable,

    #[msg("Quote amount must be greater than 0")]
    WrongQuoteAmount,
    #[msg("Base amount must be greater than 0")]
    WrongBaseAmount,

    #[msg("Insufficient fund")]
    InsufficientFund,

    #[msg("One token should be Sol")]
    UnknownToken,
    #[msg("Invalid token pair")]
    InvalidTokenPair,

    #[msg("Not elapsed Priv sale period")]
    NotElapsedPrivSalePeriod,
    #[msg("Missing signature")]
    MissingSignature,
    #[msg("Invalid message format")]
    InvalidMessageFormat,
    #[msg("Wrong signature params")]
    WrongSignatureParams,
    #[msg("Too short data len")]
    TooShortDataLen,
    #[msg("Invalid Pubkey len")]
    InvalidPubkeyLen,
    #[msg("Invalid Sig Len")]
    InvalidSigLen,
    #[msg("Signature verification failed")]
    SigVerificationFailed,

    #[msg("Too few output tokens")]
    TooFewOutputTokens,
    #[msg("Too much input sol")]
    TooMuchInputSol,
    #[msg("Too low output sol")]
    TooLowOuputSol,
    #[msg("Exceeded maximum buy amount")]
    ExceededMaxBuy,

    #[msg("BondingCurve incomplete")]
    BondingCurveIncomplete,
    #[msg("BondingCurve complete")]
    BondingCurveComplete,
    #[msg("BondingCurve already withdrawn")]
    BondingCurveAlreadyWithdrawn,
    #[msg("Insufficient Real Quote Reserves")]
    InsufficientRealQuoteReserves,

    #[msg("No rewards available")]
    NoRewardsAvailable
}
