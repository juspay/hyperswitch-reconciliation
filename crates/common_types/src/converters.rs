use crate::errors::AmountParsingError;
use crate::{
    units::MinorUnit,
    units::{FloatMajorUnit, StringMajorUnit, StringMinorUnit},
};
use common_enums::Currency;

/// Amount convertor trait for connector
pub trait AmountConvertor: Send {
    /// Output type for the connector
    type Output;
    /// helps in conversion of connector required amount type
    fn convert(
        &self,
        amount: MinorUnit,
        currency: Currency,
    ) -> Result<Self::Output, error_stack::Report<AmountParsingError>>;

    /// helps in converting back connector required amount type to core minor unit
    fn convert_back(
        &self,
        amount: Self::Output,
        currency: Currency,
    ) -> Result<MinorUnit, error_stack::Report<AmountParsingError>>;
}

/// Connector required amount type
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct StringMinorUnitForConnector;

impl AmountConvertor for StringMinorUnitForConnector {
    type Output = StringMinorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        _currency: Currency,
    ) -> Result<Self::Output, error_stack::Report<AmountParsingError>> {
        amount.to_minor_unit_as_string()
    }

    fn convert_back(
        &self,
        amount: Self::Output,
        _currency: Currency,
    ) -> Result<MinorUnit, error_stack::Report<AmountParsingError>> {
        amount.to_minor_unit_as_i64()
    }
}

/// Core required conversion type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct StringMajorUnitForCore;
impl AmountConvertor for StringMajorUnitForCore {
    type Output = StringMajorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        currency: Currency,
    ) -> Result<Self::Output, error_stack::Report<AmountParsingError>> {
        amount.to_major_unit_as_string(currency)
    }

    fn convert_back(
        &self,
        amount: StringMajorUnit,
        currency: Currency,
    ) -> Result<MinorUnit, error_stack::Report<AmountParsingError>> {
        amount.to_minor_unit_as_i64(currency)
    }
}

/// Connector required amount type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct StringMajorUnitForConnector;

impl AmountConvertor for StringMajorUnitForConnector {
    type Output = StringMajorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        currency: Currency,
    ) -> Result<Self::Output, error_stack::Report<AmountParsingError>> {
        amount.to_major_unit_as_string(currency)
    }

    fn convert_back(
        &self,
        amount: StringMajorUnit,
        currency: Currency,
    ) -> Result<MinorUnit, error_stack::Report<AmountParsingError>> {
        amount.to_minor_unit_as_i64(currency)
    }
}

/// Connector required amount type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct FloatMajorUnitForConnector;

impl AmountConvertor for FloatMajorUnitForConnector {
    type Output = FloatMajorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        currency: Currency,
    ) -> Result<Self::Output, error_stack::Report<AmountParsingError>> {
        amount.to_major_unit_as_f64(currency)
    }
    fn convert_back(
        &self,
        amount: FloatMajorUnit,
        currency: Currency,
    ) -> Result<MinorUnit, error_stack::Report<AmountParsingError>> {
        amount.to_minor_unit_as_i64(currency)
    }
}

/// Connector required amount type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct MinorUnitForConnector;

impl AmountConvertor for MinorUnitForConnector {
    type Output = MinorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        _currency: Currency,
    ) -> Result<Self::Output, error_stack::Report<AmountParsingError>> {
        Ok(amount)
    }
    fn convert_back(
        &self,
        amount: MinorUnit,
        _currency: Currency,
    ) -> Result<MinorUnit, error_stack::Report<AmountParsingError>> {
        Ok(amount)
    }
}
