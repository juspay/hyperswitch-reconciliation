pub trait ErrorSwitch<T> {
    fn switch(&self) -> T;
}

pub trait ErrorSwitchFrom<T> {
    fn switch_from(error: &T) -> Self;
}

impl<T, S> ErrorSwitch<T> for S
where
    T: ErrorSwitchFrom<Self>,
{
    fn switch(&self) -> T {
        T::switch_from(self)
    }
}

/// Allows [error_stack::Report] to change between error contexts
/// using the dependent [ErrorSwitch] trait to define relations & mappings between traits
pub trait ReportSwitchExt<T, U> {
    /// Switch to the intended report by calling switch
    /// requires error switch to be already implemented on the error type
    fn switch(self) -> Result<T, error_stack::Report<U>>;
}

impl<T, U, V> ReportSwitchExt<T, U> for Result<T, error_stack::Report<V>>
where
    V: ErrorSwitch<U> + error_stack::Context,
    U: error_stack::Context,
{
    #[track_caller]
    fn switch(self) -> Result<T, error_stack::Report<U>> {
        match self {
            Ok(i) => Ok(i),
            Err(er) => {
                let new_c = er.current_context().switch();
                Err(er.change_context(new_c))
            }
        }
    }
}

pub type CustomResult<T, E> = error_stack::Result<T, E>;

/// Cryptographic algorithm errors
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// The cryptographic algorithm was unable to encode the message
    #[error("Failed to encode given message")]
    EncodingFailed,
    /// The cryptographic algorithm was unable to decode the message
    #[error("Failed to decode given message")]
    DecodingFailed,
    /// The cryptographic algorithm was unable to sign the message
    #[error("Failed to sign message")]
    MessageSigningFailed,
    /// The cryptographic algorithm was unable to verify the given signature
    #[error("Failed to verify signature")]
    SignatureVerificationFailed,
    /// The provided key length is invalid for the cryptographic algorithm
    #[error("Invalid key length")]
    InvalidKeyLength,
    /// The provided IV length is invalid for the cryptographic algorithm
    #[error("Invalid IV length")]
    InvalidIvLength,
    /// The provided authentication tag length is invalid for the cryptographic algorithm
    #[error("Invalid authentication tag length")]
    InvalidTagLength,
}

/// Parsing Errors
#[allow(missing_docs)] // Only to prevent warnings about struct fields not being documented
#[derive(Debug, thiserror::Error)]
pub enum ParsingError {
    ///Failed to parse enum
    #[error("Failed to parse enum: {0}")]
    EnumParseFailure(&'static str),
    ///Failed to parse struct
    #[error("Failed to parse struct: {0}")]
    StructParseFailure(&'static str),
    /// Failed to encode data to given format
    #[error("Failed to serialize to {0} format")]
    EncodeError(&'static str),
    /// Failed to parse data
    #[error("Unknown error while parsing")]
    UnknownError,
    /// Failed to parse datetime
    #[error("Failed to parse datetime")]
    DateTimeParsingError,
    /// Failed to parse email
    #[error("Failed to parse email")]
    EmailParsingError,
    /// Failed to parse phone number
    #[error("Failed to parse phone number")]
    PhoneNumberParsingError,
    /// Failed to parse Float value for converting to decimal points
    #[error("Failed to parse Float value for converting to decimal points")]
    FloatToDecimalConversionFailure,
    /// Failed to parse Decimal value for i64 value conversion
    #[error("Failed to parse Decimal value for i64 value conversion")]
    DecimalToI64ConversionFailure,
    /// Failed to parse string value for f64 value conversion
    #[error("Failed to parse string value for f64 value conversion")]
    StringToFloatConversionFailure,
    /// Failed to parse i64 value for f64 value conversion
    #[error("Failed to parse i64 value for f64 value conversion")]
    I64ToDecimalConversionFailure,
    /// Failed to parse String value to Decimal value conversion because `error`
    #[error("Failed to parse String value to Decimal value conversion because {error}")]
    StringToDecimalConversionFailure { error: String },
    /// Failed to convert the given integer because of integer overflow error
    #[error("Integer Overflow error")]
    IntegerOverflow,
}

/// Validation errors.
#[allow(missing_docs)] // Only to prevent warnings about struct fields not being documented
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum ValidationError {
    /// An incorrect value was provided for the field specified by `field_name`.
    #[error("Incorrect value provided for field: {field_name}")]
    IncorrectValueProvided { field_name: &'static str },

    /// An invalid input was provided.
    #[error("{message}")]
    InvalidValue { message: String },
}

#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
pub enum KeyManagerClientError {
    #[error("Failed to construct header from the given value")]
    FailedtoConstructHeader,
    #[error("Failed to send request to Keymanager")]
    RequestNotSent(String),
    #[error("URL encoding of request failed")]
    UrlEncodingFailed,
    #[error("Failed to build the reqwest client ")]
    ClientConstructionFailed,
    #[error("Failed to send the request to Keymanager")]
    RequestSendFailed,
    #[error("Internal Server Error Received {0:?}")]
    InternalServerError(bytes::Bytes),
    #[error("Bad request received {0:?}")]
    BadRequest(bytes::Bytes),
    #[error("Payload too large {0:?}")]
    PayloadTooLarge(bytes::Bytes),
    #[error("Unexpected Error occurred while calling the KeyManager")]
    Unexpected(bytes::Bytes),
    #[error("Response Decoding failed")]
    ResponseDecodingFailed,
}

#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
pub enum KeyManagerError {
    #[error("Failed to add key to the KeyManager")]
    KeyAddFailed,
    #[error("Failed to transfer the key to the KeyManager")]
    KeyTransferFailed,
    #[error("Failed to Encrypt the data in the KeyManager")]
    EncryptionFailed,
    #[error("Failed to Decrypt the data in the KeyManager")]
    DecryptionFailed,
}

#[derive(Debug, thiserror::Error, PartialEq, Clone)]
pub enum ApiClientError {
    #[error("Header map construction failed")]
    HeaderMapConstructionFailed,
    #[error("Invalid proxy configuration")]
    InvalidProxyConfiguration,
    #[error("Client construction failed")]
    ClientConstructionFailed,
    #[error("Certificate decode failed")]
    CertificateDecodeFailed,
    #[error("Request body serialization failed")]
    BodySerializationFailed,
    #[error("Unexpected state reached/Invariants conflicted")]
    UnexpectedState,
    #[error("Failed to parse URL")]
    UrlParsingFailed,
    #[error("URL encoding of request payload failed")]
    UrlEncodingFailed,
    #[error("Failed to send request to connector {0}")]
    RequestNotSent(String),
    #[error("Failed to decode response")]
    ResponseDecodingFailed,
    #[error("Server responded with Request Timeout")]
    RequestTimeoutReceived,
    #[error("connection closed before a message could complete")]
    ConnectionClosedIncompleteMessage,
    #[error("Server responded with Internal Server Error")]
    InternalServerErrorReceived,
    #[error("Server responded with Bad Gateway")]
    BadGatewayReceived,
    #[error("Server responded with Service Unavailable")]
    ServiceUnavailableReceived,
    #[error("Server responded with Gateway Timeout")]
    GatewayTimeoutReceived,
    #[error("Server responded with unexpected response")]
    UnexpectedServerResponse,
}
