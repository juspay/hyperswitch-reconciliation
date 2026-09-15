use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, Mul, Sub},
    str::FromStr,
};

use bigdecimal::{BigDecimal, ToPrimitive as BigDecimalToPrimitive};
use utoipa::ToSchema;

use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    expression::AsExpression,
    serialize::{Output, ToSql},
    sql_types::{self},
    Queryable,
};

use rust_decimal::{prelude::FromPrimitive, Decimal};

use crate::errors::AmountParsingError;
use common_enums::Currency;

/// This Unit struct represents MinorUnit in which core amount works
#[derive(
    Default,
    Debug,
    serde::Deserialize,
    AsExpression,
    serde::Serialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    PartialOrd,
)]
#[diesel(sql_type = sql_types::BigInt)]
pub struct MinorUnit(i64);

impl MinorUnit {
    /// gets amount as i64 value will be removed in future
    pub fn get_amount_as_i64(self) -> i64 {
        self.0
    }

    /// forms a new minor default unit i.e zero
    pub fn zero() -> Self {
        Self(0)
    }

    /// forms a new minor unit from amount
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn from_optional_bigdecimal_sum(value: Option<BigDecimal>) -> Self {
        value
            .and_then(|amount| amount.to_i64())
            .map(Self::new)
            .unwrap_or_else(Self::zero)
    }

    /// checks if the amount is greater than the given value
    pub fn is_greater_than(&self, value: i64) -> bool {
        self.get_amount_as_i64() > value
    }

    /// Convert the amount to its major denomination based on Currency and return String
    /// Paypal Connector accepts Zero and Two decimal currency but not three decimal and it should be updated as required for 3 decimal currencies.
    /// Paypal Ref - https://developer.paypal.com/docs/reports/reference/paypal-supported-currencies/
    pub fn to_major_unit_as_string(
        self,
        currency: Currency,
    ) -> Result<StringMajorUnit, error_stack::Report<AmountParsingError>> {
        let amount_f64 = self.to_major_unit_as_f64(currency)?;
        let amount_string = if currency.is_zero_decimal_currency() {
            amount_f64.0.to_string()
        } else if currency.is_four_decimal_currency() {
            format!("{:.4}", amount_f64.0)
        } else if currency.is_three_decimal_currency() {
            format!("{:.3}", amount_f64.0)
        } else {
            format!("{:.2}", amount_f64.0)
        };
        Ok(StringMajorUnit::new(amount_string))
    }

    /// Convert the amount to its major denomination based on Currency and return f64
    pub fn to_major_unit_as_f64(
        self,
        currency: Currency,
    ) -> Result<FloatMajorUnit, error_stack::Report<AmountParsingError>> {
        let amount_decimal =
            Decimal::from_i64(self.0).ok_or(AmountParsingError::I64ToDecimalConversionFailure)?;

        let amount = if currency.is_zero_decimal_currency() {
            amount_decimal
        } else if currency.is_four_decimal_currency() {
            amount_decimal / Decimal::from(10000)
        } else if currency.is_three_decimal_currency() {
            amount_decimal / Decimal::from(1000)
        } else {
            amount_decimal / Decimal::from(100)
        };
        let amount_f64 = amount
            .to_f64()
            .ok_or(AmountParsingError::FloatToDecimalConversionFailure)?;
        Ok(FloatMajorUnit::new(amount_f64))
    }

    ///Convert minor unit to string minor unit
    pub fn to_minor_unit_as_string(
        self,
    ) -> Result<StringMinorUnit, error_stack::Report<AmountParsingError>> {
        Ok(StringMinorUnit::new(self.0.to_string()))
    }
}

impl Display for MinorUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for FloatMajorUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<DB> FromSql<sql_types::BigInt, DB> for MinorUnit
where
    DB: Backend,
    i64: FromSql<sql_types::BigInt, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = i64::from_sql(value)?;
        Ok(Self(val))
    }
}

impl<DB> ToSql<sql_types::BigInt, DB> for MinorUnit
where
    DB: Backend,
    i64: ToSql<sql_types::BigInt, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> Queryable<sql_types::BigInt, DB> for MinorUnit
where
    DB: Backend,
    Self: FromSql<sql_types::BigInt, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl Add for MinorUnit {
    type Output = Self;
    fn add(self, a2: Self) -> Self {
        Self(self.0 + a2.0)
    }
}

impl Sub for MinorUnit {
    type Output = Self;
    fn sub(self, a2: Self) -> Self {
        Self(self.0 - a2.0)
    }
}

impl Mul<u16> for MinorUnit {
    type Output = Self;

    fn mul(self, a2: u16) -> Self::Output {
        Self(self.0 * i64::from(a2))
    }
}

impl Sum for MinorUnit {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self(0), |a, b| a + b)
    }
}

/// Connector specific types to send
#[derive(
    Default,
    Debug,
    serde::Deserialize,
    AsExpression,
    serde::Serialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    PartialOrd,
)]
#[diesel(sql_type = sql_types::Text)]
pub struct StringMinorUnit(String);

impl StringMinorUnit {
    /// forms a new minor unit in string from amount
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// converts to minor unit i64 from minor unit string value
    pub fn to_minor_unit_as_i64(
        &self,
    ) -> Result<MinorUnit, error_stack::Report<AmountParsingError>> {
        let amount_string = &self.0;
        let amount_decimal = Decimal::from_str(amount_string).map_err(|e| {
            AmountParsingError::StringToDecimalConversionFailure {
                error: e.to_string(),
            }
        })?;
        let amount_i64 = amount_decimal
            .to_i64()
            .ok_or(AmountParsingError::DecimalToI64ConversionFailure)?;
        Ok(MinorUnit::new(amount_i64))
    }
}

impl Display for StringMinorUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for StringMinorUnit
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = String::from_sql(value)?;
        Ok(Self(val))
    }
}

impl<DB> ToSql<sql_types::Text, DB> for StringMinorUnit
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> Queryable<sql_types::Text, DB> for StringMinorUnit
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

/// Connector specific types to send
#[derive(
    Default,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    ToSchema,
)]
pub struct FloatMajorUnit(pub f64);

impl FloatMajorUnit {
    /// forms a new major unit from amount
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    /// forms a new major unit with zero amount
    pub fn zero() -> Self {
        Self(0.0)
    }

    pub fn is_positive(&self) -> bool {
        self.0 > 0.0
    }

    pub fn is_negative(&self) -> bool {
        self.0 < 0.0
    }

    /// converts to minor unit as i64 from FloatMajorUnit
    pub fn to_minor_unit_as_i64(
        self,
        currency: Currency,
    ) -> Result<MinorUnit, error_stack::Report<AmountParsingError>> {
        let amount_decimal =
            Decimal::from_f64(self.0).ok_or(AmountParsingError::FloatToDecimalConversionFailure)?;

        let amount = if currency.is_zero_decimal_currency() {
            amount_decimal
        } else if currency.is_four_decimal_currency() {
            amount_decimal * Decimal::from(10000)
        } else if currency.is_three_decimal_currency() {
            amount_decimal * Decimal::from(1000)
        } else {
            amount_decimal * Decimal::from(100)
        };

        let amount_i64 = amount
            .to_i64()
            .ok_or(AmountParsingError::DecimalToI64ConversionFailure)?;
        Ok(MinorUnit::new(amount_i64))
    }
}

impl Add for FloatMajorUnit {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sum for FloatMajorUnit {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, amount| acc + amount)
    }
}

/// Connector specific types to send
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq)]
pub struct StringMajorUnit(String);

impl StringMajorUnit {
    /// forms a new major unit from amount
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Converts to minor unit as i64 from StringMajorUnit
    pub fn to_minor_unit_as_i64(
        &self,
        currency: Currency,
    ) -> Result<MinorUnit, error_stack::Report<AmountParsingError>> {
        let amount_decimal = Decimal::from_str(&self.0).map_err(|e| {
            AmountParsingError::StringToDecimalConversionFailure {
                error: e.to_string(),
            }
        })?;

        let amount = if currency.is_zero_decimal_currency() {
            amount_decimal
        } else if currency.is_four_decimal_currency() {
            amount_decimal * Decimal::from(10000)
        } else if currency.is_three_decimal_currency() {
            amount_decimal * Decimal::from(1000)
        } else {
            amount_decimal * Decimal::from(100)
        };
        let amount_i64 = amount
            .to_i64()
            .ok_or(AmountParsingError::DecimalToI64ConversionFailure)?;
        Ok(MinorUnit::new(amount_i64))
    }
    /// forms a new StringMajorUnit default unit i.e zero
    pub fn zero() -> Self {
        Self("0".to_string())
    }
    /// Get string amount from struct to be removed in future
    pub fn get_amount_as_string(&self) -> String {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[test]
    fn test_minor_unit_new() {
        let unit = MinorUnit::new(100);
        assert_eq!(unit.get_amount_as_i64(), 100);
    }

    #[test]
    fn test_minor_unit_zero() {
        let unit = MinorUnit::zero();
        assert_eq!(unit.get_amount_as_i64(), 0);
    }

    #[test]
    fn test_minor_unit_display() {
        let unit = MinorUnit::new(1234);
        assert_eq!(format!("{}", unit), "1234");
    }

    #[rstest]
    #[case(MinorUnit::new(100), MinorUnit::new(50), MinorUnit::new(150))]
    #[case(MinorUnit::new(-100), MinorUnit::new(50), MinorUnit::new(-50))]
    #[case(MinorUnit::zero(), MinorUnit::new(50), MinorUnit::new(50))]
    fn test_minor_unit_add(
        #[case] a: MinorUnit,
        #[case] b: MinorUnit,
        #[case] expected: MinorUnit,
    ) {
        assert_eq!(a + b, expected);
    }

    #[rstest]
    #[case(MinorUnit::new(100), MinorUnit::new(50), MinorUnit::new(50))]
    #[case(MinorUnit::new(50), MinorUnit::new(100), MinorUnit::new(-50))]
    #[case(MinorUnit::zero(), MinorUnit::new(50), MinorUnit::new(-50))]
    fn test_minor_unit_sub(
        #[case] a: MinorUnit,
        #[case] b: MinorUnit,
        #[case] expected: MinorUnit,
    ) {
        assert_eq!(a - b, expected);
    }

    #[rstest]
    #[case(MinorUnit::new(100), 2u16, MinorUnit::new(200))]
    #[case(MinorUnit::new(50), 0u16, MinorUnit::new(0))]
    #[case(MinorUnit::new(-10), 3u16, MinorUnit::new(-30))]
    fn test_minor_unit_mul(#[case] a: MinorUnit, #[case] b: u16, #[case] expected: MinorUnit) {
        assert_eq!(a * b, expected);
    }

    #[rstest]
    #[case(vec![MinorUnit::new(100), MinorUnit::new(200), MinorUnit::new(300)], MinorUnit::new(600))]
    #[case(vec![MinorUnit::new(-100), MinorUnit::new(200)], MinorUnit::new(100))]
    #[case(vec![MinorUnit::zero()], MinorUnit::zero())]
    #[case(vec![], MinorUnit::zero())]
    fn test_minor_unit_sum(#[case] units: Vec<MinorUnit>, #[case] expected: MinorUnit) {
        let sum: MinorUnit = units.into_iter().sum();
        assert_eq!(sum, expected);
    }

    #[rstest]
    #[case(MinorUnit::new(100), MinorUnit::new(50), true, false)]
    #[case(MinorUnit::new(50), MinorUnit::new(100), false, true)]
    #[case(MinorUnit::new(100), MinorUnit::new(100), false, false)]
    fn test_minor_unit_ordering(
        #[case] a: MinorUnit,
        #[case] b: MinorUnit,
        #[case] a_greater_than_b: bool,
        #[case] a_less_than_b: bool,
    ) {
        assert_eq!(a > b, a_greater_than_b);
        assert_eq!(a < b, a_less_than_b);
        assert_eq!(a == b, !a_greater_than_b && !a_less_than_b);
    }
}
