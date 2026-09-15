use common_enums::Currency;

/// Errors that can occur when working with `AmountType`
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AmountError {
    /// Attempted to perform an operation on amounts with different currencies
    #[error("Currency mismatch: cannot operate on {left} and {right}")]
    CurrencyMismatch { left: Currency, right: Currency },

    /// Invalid amount conversion
    #[error("Invalid amount conversion: {message}")]
    InvalidConversion { message: String },

    /// Amount overflow
    #[error("Amount overflow")]
    Overflow,

    /// Amount underflow
    #[error("Amount underflow")]
    Underflow,
}

/// Parsing Errors
#[allow(missing_docs)] // Only to prevent warnings about struct fields not being documented
#[derive(Debug, thiserror::Error)]
pub enum AmountParsingError {
    /// Failed to parse Float value for converting to decimal points
    #[error("Failed to parse Float value for converting to decimal points")]
    FloatToDecimalConversionFailure,
    /// Failed to parse Decimal value for i64 value conversion
    #[error("Failed to parse Decimal value for i64 value conversion")]
    DecimalToI64ConversionFailure,
    /// Failed to parse i64 value for f64 value conversion
    #[error("Failed to parse i64 value for f64 value conversion")]
    I64ToDecimalConversionFailure,
    /// Failed to parse String value to Decimal value conversion because `error`
    #[error("Failed to parse String value to Decimal value conversion because {error}")]
    StringToDecimalConversionFailure { error: String },
}
