use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::errors::{CustomResult, ParsingError};

// Extending functionalities of `[u8]` for performing parsing
pub trait ByteSliceExt {
    /// Convert `[u8]` into type `<T>` by using `serde::Deserialize`
    fn parse_struct<'de, T>(&'de self, type_name: &'static str) -> CustomResult<T, ParsingError>
    where
        T: Deserialize<'de>;
}

impl ByteSliceExt for [u8] {
    #[track_caller]
    fn parse_struct<'de, T>(&'de self, type_name: &'static str) -> CustomResult<T, ParsingError>
    where
        T: Deserialize<'de>,
    {
        serde_json::from_slice(self)
            .change_context(ParsingError::StructParseFailure(type_name))
            .attach_printable_lazy(|| format!("Unable to parse {type_name} from &[u8] {:?}", self))
    }
}

/// Encode interface
/// An interface for performing type conversions and serialization
pub trait Encode<'e>
where
    Self: 'e + std::fmt::Debug,
{
    // If needed get type information/custom error implementation.
    ///
    /// Converting `Self` into an intermediate representation `<P>`
    /// and then performing encoding operation using the `Serialize` trait from `serde`
    /// Specifically to convert into json, by using `serde_json`
    fn convert_and_encode<P>(&'e self) -> CustomResult<String, ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize;

    /// Converting `Self` into an intermediate representation `<P>`
    /// and then performing encoding operation using the `Serialize` trait from `serde`
    /// Specifically, to convert into urlencoded, by using `serde_urlencoded`
    fn convert_and_url_encode<P>(&'e self) -> CustomResult<String, ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize;

    /// Functionality, for specifically encoding `Self` into `String`
    /// after serialization by using `serde::Serialize`
    fn url_encode(&'e self) -> CustomResult<String, ParsingError>
    where
        Self: Serialize;

    /// Functionality, for specifically encoding `Self` into `String`
    /// after serialization by using `serde::Serialize`
    /// specifically, to convert into JSON `String`.
    fn encode_to_string_of_json(&'e self) -> CustomResult<String, ParsingError>
    where
        Self: Serialize;

    /// Functionality, for specifically encoding `Self` into `String`
    /// after serialization by using `serde::Serialize`
    /// specifically, to convert into XML `String`.
    fn encode_to_string_of_xml(&'e self) -> CustomResult<String, ParsingError>
    where
        Self: Serialize;

    /// Functionality, for specifically encoding `Self` into `serde_json::Value`
    /// after serialization by using `serde::Serialize`
    fn encode_to_value(&'e self) -> CustomResult<serde_json::Value, ParsingError>
    where
        Self: Serialize;

    /// Functionality, for specifically encoding `Self` into `Vec<u8>`
    /// after serialization by using `serde::Serialize`
    fn encode_to_vec(&'e self) -> CustomResult<Vec<u8>, ParsingError>
    where
        Self: Serialize;
}

impl<'e, A> Encode<'e> for A
where
    Self: 'e + std::fmt::Debug,
{
    fn convert_and_encode<P>(&'e self) -> CustomResult<String, ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_json::to_string(&P::try_from(self).change_context(ParsingError::UnknownError)?)
            .change_context(ParsingError::EncodeError("string"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn convert_and_url_encode<P>(&'e self) -> CustomResult<String, ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_urlencoded::to_string(&P::try_from(self).change_context(ParsingError::UnknownError)?)
            .change_context(ParsingError::EncodeError("url-encoded"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    // Check without two functions can we combine this
    fn url_encode(&'e self) -> CustomResult<String, ParsingError>
    where
        Self: Serialize,
    {
        serde_urlencoded::to_string(self)
            .change_context(ParsingError::EncodeError("url-encoded"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn encode_to_string_of_json(&'e self) -> CustomResult<String, ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_string(self)
            .change_context(ParsingError::EncodeError("json"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn encode_to_string_of_xml(&'e self) -> CustomResult<String, ParsingError>
    where
        Self: Serialize,
    {
        quick_xml::se::to_string(self)
            .change_context(ParsingError::EncodeError("xml"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn encode_to_value(&'e self) -> CustomResult<serde_json::Value, ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_value(self)
            .change_context(ParsingError::EncodeError("json-value"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a value"))
    }

    fn encode_to_vec(&'e self) -> CustomResult<Vec<u8>, ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_vec(self)
            .change_context(ParsingError::EncodeError("byte-vec"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a value"))
    }
}
