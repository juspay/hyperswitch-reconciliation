use crate::errors::AmountParsingError;
use crate::units::MinorUnit;
use crate::{errors::AmountError, units::FloatMajorUnit};
use common_enums::Currency;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};
use utoipa::ToSchema;

/// Represents a monetary amount with its associated currency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct AmountType {
    pub value: MinorUnit,
    pub currency: Currency,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct AmountResponse {
    #[schema(example = 99.99)]
    pub value: FloatMajorUnit,
    pub currency: Currency,
}

impl AmountResponse {
    pub fn from_minor_unit(
        amount: MinorUnit,
        currency: Currency,
    ) -> Result<Self, error_stack::Report<AmountParsingError>> {
        let amount_f64 = amount.to_major_unit_as_f64(currency)?;

        Ok(Self {
            value: amount_f64,
            currency,
        })
    }
}

impl AmountType {
    /// Creates a new `AmountType` with the specified value and currency
    pub fn new(value: MinorUnit, currency: Currency) -> Self {
        Self { value, currency }
    }

    /// Creates a zero amount for the specified currency
    pub fn zero(currency: Currency) -> Self {
        Self {
            value: MinorUnit::zero(),
            currency,
        }
    }

    /// Creates an `AmountType` from a major unit amount (e.g., dollars, not cents)
    pub fn from_major_unit(
        amount: FloatMajorUnit,
        currency: Currency,
    ) -> Result<Self, error_stack::Report<AmountParsingError>> {
        let minor_unit = amount.to_minor_unit_as_i64(currency)?;

        Ok(Self {
            value: minor_unit,
            currency,
        })
    }

    /// Returns the value in minor units
    pub fn value(&self) -> MinorUnit {
        self.value
    }

    /// Returns the currency
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Converts the amount to a string in major units (e.g., "12.34" for $12.34)
    pub fn to_major_unit_string(&self) -> Result<String, error_stack::Report<AmountParsingError>> {
        self.value
            .to_major_unit_as_string(self.currency)
            .map(|value| value.get_amount_as_string())
    }

    /// Converts the amount to a display string with currency symbol
    pub fn to_display_string(&self) -> Result<String, error_stack::Report<AmountParsingError>> {
        Ok(format!(
            "{} {}",
            self.to_major_unit_string()?,
            self.currency
        ))
    }

    /// Checks if the amount is zero
    pub fn is_zero(&self) -> bool {
        self.value.get_amount_as_i64() == 0
    }

    /// Checks if the amount is positive
    pub fn is_positive(&self) -> bool {
        self.value.get_amount_as_i64() > 0
    }

    /// Checks if the amount is negative
    pub fn is_negative(&self) -> bool {
        self.value.get_amount_as_i64() < 0
    }

    /// Returns the absolute value of the amount
    pub fn abs(&self) -> Self {
        Self {
            value: MinorUnit::new(self.value.get_amount_as_i64().abs()),
            currency: self.currency,
        }
    }

    /// Validates that two amounts have the same currency
    fn validate_same_currency(&self, other: &Self) -> Result<(), AmountError> {
        if self.currency != other.currency {
            Err(AmountError::CurrencyMismatch {
                left: self.currency,
                right: other.currency,
            })
        } else {
            Ok(())
        }
    }
}

impl Add for AmountType {
    type Output = Result<Self, AmountError>;

    /// Adds two amounts of the same currency
    fn add(self, other: Self) -> Self::Output {
        self.validate_same_currency(&other)?;
        Ok(Self {
            value: self.value + other.value,
            currency: self.currency,
        })
    }
}

impl Sub for AmountType {
    type Output = Result<Self, AmountError>;

    /// Subtracts two amounts of the same currency
    fn sub(self, other: Self) -> Self::Output {
        self.validate_same_currency(&other)?;
        Ok(Self {
            value: self.value - other.value,
            currency: self.currency,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_basic_operations() {
        let amount = AmountType::new(MinorUnit::new(1234), Currency::USD);
        assert_eq!(amount.value().get_amount_as_i64(), 1234);
        assert_eq!(amount.currency(), Currency::USD);

        let zero_usd = AmountType::zero(Currency::USD);
        assert!(zero_usd.is_zero());
        assert_eq!(zero_usd.value().get_amount_as_i64(), 0);
    }

    #[rstest]
    #[case(12.34, Currency::USD, 1234)]
    #[case(150.0, Currency::JPY, 150)]
    #[case(12.345, Currency::BHD, 12345)]
    #[case(1.2345, Currency::CLF, 12345)]
    fn test_major_unit_conversions(
        #[case] major_amount: f64,
        #[case] currency: Currency,
        #[case] expected_minor: i64,
    ) {
        let amount =
            AmountType::from_major_unit(FloatMajorUnit::new(major_amount), currency).unwrap();
        assert_eq!(amount.value().get_amount_as_i64(), expected_minor);
        assert_eq!(amount.currency(), currency);
    }

    #[rstest]
    #[case(MinorUnit::new(1234), Currency::USD, "12.34", "12.34 USD")]
    #[case(MinorUnit::new(150), Currency::JPY, "150", "150 JPY")]
    #[case(MinorUnit::new(12345), Currency::BHD, "12.345", "12.345 BHD")]
    fn test_currency_formatting(
        #[case] minor_unit: MinorUnit,
        #[case] currency: Currency,
        #[case] expected_string: &str,
        #[case] expected_display: &str,
    ) {
        let amount = AmountType::new(minor_unit, currency);
        assert_eq!(amount.to_major_unit_string().unwrap(), expected_string);
        assert_eq!(amount.to_display_string().unwrap(), expected_display);
    }

    #[test]
    fn test_arithmetic_operations() {
        let amount1 = AmountType::new(MinorUnit::new(1000), Currency::USD);
        let amount2 = AmountType::new(MinorUnit::new(500), Currency::USD);

        let add_result = (amount1 + amount2).unwrap();
        assert_eq!(add_result.value().get_amount_as_i64(), 1500);

        let sub_result = (amount1 - amount2).unwrap();
        assert_eq!(sub_result.value().get_amount_as_i64(), 500);

        // Test currency mismatch
        let eur_amount = AmountType::new(MinorUnit::new(800), Currency::EUR);
        let mismatch_result = amount1 + eur_amount;
        assert!(mismatch_result.is_err());
    }

    #[rstest]
    #[case(100, true, false, false)]
    #[case(-100, false, true, false)]
    #[case(0, false, false, true)]
    fn test_amount_properties(
        #[case] value: i64,
        #[case] is_positive: bool,
        #[case] is_negative: bool,
        #[case] is_zero: bool,
    ) {
        let amount = AmountType::new(MinorUnit::new(value), Currency::USD);
        assert_eq!(amount.is_positive(), is_positive);
        assert_eq!(amount.is_negative(), is_negative);
        assert_eq!(amount.is_zero(), is_zero);
        assert_eq!(amount.abs().value().get_amount_as_i64(), value.abs());
    }

    #[rstest]
    #[case(f64::INFINITY)]
    #[case(f64::NAN)]
    #[case(f64::NEG_INFINITY)]
    fn test_error_handling(#[case] invalid_amount: f64) {
        assert!(
            AmountType::from_major_unit(FloatMajorUnit::new(invalid_amount), Currency::USD)
                .is_err()
        );
    }

    #[test]
    fn test_serialization() {
        let amount = AmountType::new(MinorUnit::new(1234), Currency::USD);
        let serialized = serde_json::to_string(&amount).unwrap();
        let deserialized: AmountType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(amount, deserialized);
    }
}
