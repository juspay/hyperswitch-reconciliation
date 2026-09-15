use std::num::{ParseFloatError, TryFromIntError};
use utoipa::ToSchema;

/// Entry type for staging entries
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BalanceDirection {
    Credit,
    Debit,
}

impl BalanceDirection {
    pub fn opposite(&self) -> Self {
        match self {
            Self::Credit => Self::Debit,
            Self::Debit => Self::Credit,
        }
    }
}

/// Status of entries
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EntryStatus {
    Pending,
    Expected,
    Posted,
    Mismatched,
    Archived,
    Void,
    Matched,
}

/// The three letter ISO currency code in uppercase. Eg: 'USD' for the United States Dollar.
#[allow(clippy::upper_case_acronyms)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
pub enum Currency {
    AED,
    AFN,
    ALL,
    AMD,
    ANG,
    AOA,
    ARS,
    AUD,
    AWG,
    AZN,
    BAM,
    BBD,
    BDT,
    BGN,
    BHD,
    BIF,
    BMD,
    BND,
    BOB,
    BRL,
    BSD,
    BTN,
    BWP,
    BYN,
    BZD,
    CAD,
    CDF,
    CHF,
    CLF,
    CLP,
    CNY,
    COP,
    CRC,
    CUC,
    CUP,
    CVE,
    CZK,
    DJF,
    DKK,
    DOP,
    DZD,
    EGP,
    ERN,
    ETB,
    EUR,
    FJD,
    FKP,
    GBP,
    GEL,
    GHS,
    GIP,
    GMD,
    GNF,
    GTQ,
    GYD,
    HKD,
    HNL,
    HRK,
    HTG,
    HUF,
    IDR,
    ILS,
    INR,
    IQD,
    IRR,
    ISK,
    JMD,
    JOD,
    JPY,
    KES,
    KGS,
    KHR,
    KMF,
    KPW,
    KRW,
    KWD,
    KYD,
    KZT,
    LAK,
    LBP,
    LKR,
    LRD,
    LSL,
    LYD,
    MAD,
    MDL,
    MGA,
    MKD,
    MMK,
    MNT,
    MOP,
    MRU,
    MUR,
    MVR,
    MWK,
    MXN,
    MYR,
    MZN,
    NAD,
    NGN,
    NIO,
    NOK,
    NPR,
    NZD,
    OMR,
    PAB,
    PEN,
    PGK,
    PHP,
    PKR,
    PLN,
    PYG,
    QAR,
    RON,
    RSD,
    RUB,
    RWF,
    SAR,
    SBD,
    SCR,
    SDG,
    SEK,
    SGD,
    SHP,
    SLE,
    SLL,
    SOS,
    SRD,
    SSP,
    STD,
    STN,
    SVC,
    SYP,
    SZL,
    THB,
    TJS,
    TMT,
    TND,
    TOP,
    TRY,
    TTD,
    TWD,
    TZS,
    UAH,
    UGX,
    #[default]
    USD,
    UYU,
    UZS,
    VES,
    VND,
    VUV,
    WST,
    XAF,
    XCD,
    XOF,
    XPF,
    YER,
    ZAR,
    ZMW,
    ZWL,
}

impl Currency {
    /// Convert the amount to its base denomination based on Currency and return String
    pub fn to_currency_base_unit(self, amount: i64) -> Result<String, TryFromIntError> {
        let amount_f64 = self.to_currency_base_unit_asf64(amount)?;
        Ok(format!("{amount_f64:.2}"))
    }

    /// Convert the amount to its base denomination based on Currency and return f64
    pub fn to_currency_base_unit_asf64(self, amount: i64) -> Result<f64, TryFromIntError> {
        let amount_f64: f64 = u32::try_from(amount)?.into();
        let amount = if self.is_zero_decimal_currency() {
            amount_f64
        } else if self.is_three_decimal_currency() {
            amount_f64 / 1000.00
        } else {
            amount_f64 / 100.00
        };
        Ok(amount)
    }

    ///Convert the higher decimal amount to its base absolute units
    pub fn to_currency_lower_unit(self, amount: String) -> Result<String, ParseFloatError> {
        let amount_f64 = amount.parse::<f64>()?;
        let amount_string = if self.is_zero_decimal_currency() {
            amount_f64
        } else if self.is_three_decimal_currency() {
            amount_f64 * 1000.00
        } else {
            amount_f64 * 100.00
        };
        Ok(amount_string.to_string())
    }

    /// Convert the amount to its base denomination based on Currency and check for zero decimal currency and return String
    /// Paypal Connector accepts Zero and Two decimal currency but not three decimal and it should be updated as required for 3 decimal currencies.
    /// Paypal Ref - https://developer.paypal.com/docs/reports/reference/paypal-supported-currencies/
    pub fn to_currency_base_unit_with_zero_decimal_check(
        self,
        amount: i64,
    ) -> Result<String, TryFromIntError> {
        let amount_f64 = self.to_currency_base_unit_asf64(amount)?;
        if self.is_zero_decimal_currency() {
            Ok(amount_f64.to_string())
        } else {
            Ok(format!("{amount_f64:.2}"))
        }
    }

    pub fn iso_4217(self) -> &'static str {
        match self {
            Self::AED => "784",
            Self::AFN => "971",
            Self::ALL => "008",
            Self::AMD => "051",
            Self::ANG => "532",
            Self::AOA => "973",
            Self::ARS => "032",
            Self::AUD => "036",
            Self::AWG => "533",
            Self::AZN => "944",
            Self::BAM => "977",
            Self::BBD => "052",
            Self::BDT => "050",
            Self::BGN => "975",
            Self::BHD => "048",
            Self::BIF => "108",
            Self::BMD => "060",
            Self::BND => "096",
            Self::BOB => "068",
            Self::BRL => "986",
            Self::BSD => "044",
            Self::BTN => "064",
            Self::BWP => "072",
            Self::BYN => "933",
            Self::BZD => "084",
            Self::CAD => "124",
            Self::CDF => "976",
            Self::CHF => "756",
            Self::CLF => "990",
            Self::CLP => "152",
            Self::COP => "170",
            Self::CRC => "188",
            Self::CUC => "931",
            Self::CUP => "192",
            Self::CVE => "132",
            Self::CZK => "203",
            Self::DJF => "262",
            Self::DKK => "208",
            Self::DOP => "214",
            Self::DZD => "012",
            Self::EGP => "818",
            Self::ERN => "232",
            Self::ETB => "230",
            Self::EUR => "978",
            Self::FJD => "242",
            Self::FKP => "238",
            Self::GBP => "826",
            Self::GEL => "981",
            Self::GHS => "936",
            Self::GIP => "292",
            Self::GMD => "270",
            Self::GNF => "324",
            Self::GTQ => "320",
            Self::GYD => "328",
            Self::HKD => "344",
            Self::HNL => "340",
            Self::HTG => "332",
            Self::HUF => "348",
            Self::HRK => "191",
            Self::IDR => "360",
            Self::ILS => "376",
            Self::INR => "356",
            Self::IQD => "368",
            Self::IRR => "364",
            Self::ISK => "352",
            Self::JMD => "388",
            Self::JOD => "400",
            Self::JPY => "392",
            Self::KES => "404",
            Self::KGS => "417",
            Self::KHR => "116",
            Self::KMF => "174",
            Self::KPW => "408",
            Self::KRW => "410",
            Self::KWD => "414",
            Self::KYD => "136",
            Self::KZT => "398",
            Self::LAK => "418",
            Self::LBP => "422",
            Self::LKR => "144",
            Self::LRD => "430",
            Self::LSL => "426",
            Self::LYD => "434",
            Self::MAD => "504",
            Self::MDL => "498",
            Self::MGA => "969",
            Self::MKD => "807",
            Self::MMK => "104",
            Self::MNT => "496",
            Self::MOP => "446",
            Self::MRU => "929",
            Self::MUR => "480",
            Self::MVR => "462",
            Self::MWK => "454",
            Self::MXN => "484",
            Self::MYR => "458",
            Self::MZN => "943",
            Self::NAD => "516",
            Self::NGN => "566",
            Self::NIO => "558",
            Self::NOK => "578",
            Self::NPR => "524",
            Self::NZD => "554",
            Self::OMR => "512",
            Self::PAB => "590",
            Self::PEN => "604",
            Self::PGK => "598",
            Self::PHP => "608",
            Self::PKR => "586",
            Self::PLN => "985",
            Self::PYG => "600",
            Self::QAR => "634",
            Self::RON => "946",
            Self::CNY => "156",
            Self::RSD => "941",
            Self::RUB => "643",
            Self::RWF => "646",
            Self::SAR => "682",
            Self::SBD => "090",
            Self::SCR => "690",
            Self::SDG => "938",
            Self::SEK => "752",
            Self::SGD => "702",
            Self::SHP => "654",
            Self::SLE => "925",
            Self::SLL => "694",
            Self::SOS => "706",
            Self::SRD => "968",
            Self::SSP => "728",
            Self::STD => "678",
            Self::STN => "930",
            Self::SVC => "222",
            Self::SYP => "760",
            Self::SZL => "748",
            Self::THB => "764",
            Self::TJS => "972",
            Self::TMT => "934",
            Self::TND => "788",
            Self::TOP => "776",
            Self::TRY => "949",
            Self::TTD => "780",
            Self::TWD => "901",
            Self::TZS => "834",
            Self::UAH => "980",
            Self::UGX => "800",
            Self::USD => "840",
            Self::UYU => "858",
            Self::UZS => "860",
            Self::VES => "928",
            Self::VND => "704",
            Self::VUV => "548",
            Self::WST => "882",
            Self::XAF => "950",
            Self::XCD => "951",
            Self::XOF => "952",
            Self::XPF => "953",
            Self::YER => "886",
            Self::ZAR => "710",
            Self::ZMW => "967",
            Self::ZWL => "932",
        }
    }

    pub fn is_zero_decimal_currency(self) -> bool {
        match self {
            Self::BIF
            | Self::CLP
            | Self::DJF
            | Self::GNF
            | Self::IRR
            | Self::JPY
            | Self::KMF
            | Self::KRW
            | Self::MGA
            | Self::PYG
            | Self::RWF
            | Self::UGX
            | Self::VND
            | Self::VUV
            | Self::XAF
            | Self::XOF
            | Self::XPF => true,
            Self::AED
            | Self::AFN
            | Self::ALL
            | Self::AMD
            | Self::ANG
            | Self::AOA
            | Self::ARS
            | Self::AUD
            | Self::AWG
            | Self::AZN
            | Self::BAM
            | Self::BBD
            | Self::BDT
            | Self::BGN
            | Self::BHD
            | Self::BMD
            | Self::BND
            | Self::BOB
            | Self::BRL
            | Self::BSD
            | Self::BTN
            | Self::BWP
            | Self::BYN
            | Self::BZD
            | Self::CAD
            | Self::CDF
            | Self::CHF
            | Self::CLF
            | Self::CNY
            | Self::COP
            | Self::CRC
            | Self::CUC
            | Self::CUP
            | Self::CVE
            | Self::CZK
            | Self::DKK
            | Self::DOP
            | Self::DZD
            | Self::EGP
            | Self::ERN
            | Self::ETB
            | Self::EUR
            | Self::FJD
            | Self::FKP
            | Self::GBP
            | Self::GEL
            | Self::GHS
            | Self::GIP
            | Self::GMD
            | Self::GTQ
            | Self::GYD
            | Self::HKD
            | Self::HNL
            | Self::HRK
            | Self::HTG
            | Self::HUF
            | Self::IDR
            | Self::ILS
            | Self::INR
            | Self::IQD
            | Self::ISK
            | Self::JMD
            | Self::JOD
            | Self::KES
            | Self::KGS
            | Self::KHR
            | Self::KPW
            | Self::KWD
            | Self::KYD
            | Self::KZT
            | Self::LAK
            | Self::LBP
            | Self::LKR
            | Self::LRD
            | Self::LSL
            | Self::LYD
            | Self::MAD
            | Self::MDL
            | Self::MKD
            | Self::MMK
            | Self::MNT
            | Self::MOP
            | Self::MRU
            | Self::MUR
            | Self::MVR
            | Self::MWK
            | Self::MXN
            | Self::MYR
            | Self::MZN
            | Self::NAD
            | Self::NGN
            | Self::NIO
            | Self::NOK
            | Self::NPR
            | Self::NZD
            | Self::OMR
            | Self::PAB
            | Self::PEN
            | Self::PGK
            | Self::PHP
            | Self::PKR
            | Self::PLN
            | Self::QAR
            | Self::RON
            | Self::RSD
            | Self::RUB
            | Self::SAR
            | Self::SBD
            | Self::SCR
            | Self::SDG
            | Self::SEK
            | Self::SGD
            | Self::SHP
            | Self::SLE
            | Self::SLL
            | Self::SOS
            | Self::SRD
            | Self::SSP
            | Self::STD
            | Self::STN
            | Self::SVC
            | Self::SYP
            | Self::SZL
            | Self::THB
            | Self::TJS
            | Self::TMT
            | Self::TND
            | Self::TOP
            | Self::TRY
            | Self::TTD
            | Self::TWD
            | Self::TZS
            | Self::UAH
            | Self::USD
            | Self::UYU
            | Self::UZS
            | Self::VES
            | Self::WST
            | Self::XCD
            | Self::YER
            | Self::ZAR
            | Self::ZMW
            | Self::ZWL => false,
        }
    }

    pub fn is_three_decimal_currency(self) -> bool {
        match self {
            Self::BHD | Self::IQD | Self::JOD | Self::KWD | Self::LYD | Self::OMR | Self::TND => {
                true
            }
            Self::AED
            | Self::AFN
            | Self::ALL
            | Self::AMD
            | Self::AOA
            | Self::ANG
            | Self::ARS
            | Self::AUD
            | Self::AWG
            | Self::AZN
            | Self::BAM
            | Self::BBD
            | Self::BDT
            | Self::BGN
            | Self::BIF
            | Self::BMD
            | Self::BND
            | Self::BOB
            | Self::BRL
            | Self::BSD
            | Self::BTN
            | Self::BWP
            | Self::BYN
            | Self::BZD
            | Self::CAD
            | Self::CDF
            | Self::CHF
            | Self::CLF
            | Self::CLP
            | Self::CNY
            | Self::COP
            | Self::CRC
            | Self::CUC
            | Self::CUP
            | Self::CVE
            | Self::CZK
            | Self::DJF
            | Self::DKK
            | Self::DOP
            | Self::DZD
            | Self::EGP
            | Self::ERN
            | Self::ETB
            | Self::EUR
            | Self::FJD
            | Self::FKP
            | Self::GBP
            | Self::GEL
            | Self::GHS
            | Self::GIP
            | Self::GMD
            | Self::GNF
            | Self::GTQ
            | Self::GYD
            | Self::HKD
            | Self::HNL
            | Self::HRK
            | Self::HTG
            | Self::HUF
            | Self::IDR
            | Self::ILS
            | Self::INR
            | Self::IRR
            | Self::ISK
            | Self::JMD
            | Self::JPY
            | Self::KES
            | Self::KGS
            | Self::KHR
            | Self::KMF
            | Self::KPW
            | Self::KRW
            | Self::KYD
            | Self::KZT
            | Self::LAK
            | Self::LBP
            | Self::LKR
            | Self::LRD
            | Self::LSL
            | Self::MAD
            | Self::MDL
            | Self::MGA
            | Self::MKD
            | Self::MMK
            | Self::MNT
            | Self::MOP
            | Self::MRU
            | Self::MUR
            | Self::MVR
            | Self::MWK
            | Self::MXN
            | Self::MYR
            | Self::MZN
            | Self::NAD
            | Self::NGN
            | Self::NIO
            | Self::NOK
            | Self::NPR
            | Self::NZD
            | Self::PAB
            | Self::PEN
            | Self::PGK
            | Self::PHP
            | Self::PKR
            | Self::PLN
            | Self::PYG
            | Self::QAR
            | Self::RON
            | Self::RSD
            | Self::RUB
            | Self::RWF
            | Self::SAR
            | Self::SBD
            | Self::SCR
            | Self::SDG
            | Self::SEK
            | Self::SGD
            | Self::SHP
            | Self::SLE
            | Self::SLL
            | Self::SOS
            | Self::SRD
            | Self::SSP
            | Self::STD
            | Self::STN
            | Self::SVC
            | Self::SYP
            | Self::SZL
            | Self::THB
            | Self::TJS
            | Self::TMT
            | Self::TOP
            | Self::TRY
            | Self::TTD
            | Self::TWD
            | Self::TZS
            | Self::UAH
            | Self::UGX
            | Self::USD
            | Self::UYU
            | Self::UZS
            | Self::VES
            | Self::VND
            | Self::VUV
            | Self::WST
            | Self::XAF
            | Self::XCD
            | Self::XPF
            | Self::XOF
            | Self::YER
            | Self::ZAR
            | Self::ZMW
            | Self::ZWL => false,
        }
    }

    pub fn is_four_decimal_currency(self) -> bool {
        match self {
            Self::CLF => true,
            Self::AED
            | Self::AFN
            | Self::ALL
            | Self::AMD
            | Self::AOA
            | Self::ANG
            | Self::ARS
            | Self::AUD
            | Self::AWG
            | Self::AZN
            | Self::BAM
            | Self::BBD
            | Self::BDT
            | Self::BGN
            | Self::BHD
            | Self::BIF
            | Self::BMD
            | Self::BND
            | Self::BOB
            | Self::BRL
            | Self::BSD
            | Self::BTN
            | Self::BWP
            | Self::BYN
            | Self::BZD
            | Self::CAD
            | Self::CDF
            | Self::CHF
            | Self::CLP
            | Self::CNY
            | Self::COP
            | Self::CRC
            | Self::CUC
            | Self::CUP
            | Self::CVE
            | Self::CZK
            | Self::DJF
            | Self::DKK
            | Self::DOP
            | Self::DZD
            | Self::EGP
            | Self::ERN
            | Self::ETB
            | Self::EUR
            | Self::FJD
            | Self::FKP
            | Self::GBP
            | Self::GEL
            | Self::GHS
            | Self::GIP
            | Self::GMD
            | Self::GNF
            | Self::GTQ
            | Self::GYD
            | Self::HKD
            | Self::HNL
            | Self::HRK
            | Self::HTG
            | Self::HUF
            | Self::IDR
            | Self::ILS
            | Self::INR
            | Self::IQD
            | Self::IRR
            | Self::ISK
            | Self::JMD
            | Self::JOD
            | Self::JPY
            | Self::KES
            | Self::KGS
            | Self::KHR
            | Self::KMF
            | Self::KPW
            | Self::KRW
            | Self::KWD
            | Self::KYD
            | Self::KZT
            | Self::LAK
            | Self::LBP
            | Self::LKR
            | Self::LRD
            | Self::LSL
            | Self::LYD
            | Self::MAD
            | Self::MDL
            | Self::MGA
            | Self::MKD
            | Self::MMK
            | Self::MNT
            | Self::MOP
            | Self::MRU
            | Self::MUR
            | Self::MVR
            | Self::MWK
            | Self::MXN
            | Self::MYR
            | Self::MZN
            | Self::NAD
            | Self::NGN
            | Self::NIO
            | Self::NOK
            | Self::NPR
            | Self::NZD
            | Self::OMR
            | Self::PAB
            | Self::PEN
            | Self::PGK
            | Self::PHP
            | Self::PKR
            | Self::PLN
            | Self::PYG
            | Self::QAR
            | Self::RON
            | Self::RSD
            | Self::RUB
            | Self::RWF
            | Self::SAR
            | Self::SBD
            | Self::SCR
            | Self::SDG
            | Self::SEK
            | Self::SGD
            | Self::SHP
            | Self::SLE
            | Self::SLL
            | Self::SOS
            | Self::SRD
            | Self::SSP
            | Self::STD
            | Self::STN
            | Self::SVC
            | Self::SYP
            | Self::SZL
            | Self::THB
            | Self::TJS
            | Self::TMT
            | Self::TND
            | Self::TOP
            | Self::TRY
            | Self::TTD
            | Self::TWD
            | Self::TZS
            | Self::UAH
            | Self::UGX
            | Self::USD
            | Self::UYU
            | Self::UZS
            | Self::VES
            | Self::VND
            | Self::VUV
            | Self::WST
            | Self::XAF
            | Self::XCD
            | Self::XPF
            | Self::XOF
            | Self::YER
            | Self::ZAR
            | Self::ZMW
            | Self::ZWL => false,
        }
    }

    pub fn number_of_digits_after_decimal_point(self) -> u8 {
        if self.is_zero_decimal_currency() {
            0
        } else if self.is_three_decimal_currency() {
            3
        } else if self.is_four_decimal_currency() {
            4
        } else {
            2
        }
    }
}

/// Status of staging entries
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DieselStagingEntryStatus {
    Pending,
    NeedsManualReviewNoRulesFound,
    NeedsManualReviewCurrencyMismatch,
    NeedsManualReviewMissingSearchIdentifierValue,
    NeedsManualReviewDuplicateEntry,
    NeedsManualReviewNoExpectationEntryFound,
    NeedsManualReviewMultipleExpectedEntriesFound,
    NeedsManualReviewMissingMatchField,
    NeedsManualReviewMissingUniqueField,
    NeedsManualReviewMissingGroupingField,
    NeedsManualReviewInternalError,
    Processed,
    Void,
    Archived,
}

/// ProcessingMode of staging entries
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum StagingEntryProcessingMode {
    Transaction,
    Confirmation,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    Default,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DieselTransactionStatus {
    #[default]
    Expected,
    OverAmountExpected,
    OverAmountMismatch,
    UnderAmountExpected,
    UnderAmountMismatch,
    DataMismatch,
    CurrencyMismatch,
    SplitMismatch,
    Archived,
    Void,
    PartiallyReconciled,
    MatchedAuto,
    MatchedManual,
    MatchedForce,
    MatchedWithTolerance,
    PostedManual,
}

impl DieselTransactionStatus {
    pub fn is_voidable(&self) -> bool {
        match self {
            Self::Expected
            | Self::OverAmountExpected
            | Self::OverAmountMismatch
            | Self::UnderAmountExpected
            | Self::UnderAmountMismatch
            | Self::SplitMismatch
            | Self::PartiallyReconciled
            | Self::DataMismatch
            | Self::CurrencyMismatch
            | Self::MatchedAuto
            | Self::MatchedManual
            | Self::MatchedForce
            | Self::MatchedWithTolerance => true,
            Self::Archived | Self::Void | Self::PostedManual => false,
        }
    }

    pub fn is_manually_postable(&self) -> bool {
        match self {
            Self::MatchedAuto
            | Self::MatchedManual
            | Self::MatchedForce
            | Self::MatchedWithTolerance => true,
            Self::Expected
            | Self::OverAmountExpected
            | Self::OverAmountMismatch
            | Self::UnderAmountExpected
            | Self::UnderAmountMismatch
            | Self::SplitMismatch
            | Self::PartiallyReconciled
            | Self::DataMismatch
            | Self::CurrencyMismatch
            | Self::Archived
            | Self::Void
            | Self::PostedManual => false,
        }
    }
}

/// Status for expectations
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ExpectationStatus {
    Expected,
    Archived,
}

/// Status for recon processor
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReconProcessorStatus {
    Running,
    Stopped,
}

/// Status of ingestion operations
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum IngestionStatus {
    Pending,
    Processing,
    Processed,
    Failed,
    Discarded,
}

/// Status for transformation processing
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TransformationStatus {
    Pending,
    Processing,
    Processed,
    Failed,
}

/// Types of resolution actions available for exceptions
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResolutionActionType {
    /// Create a new reconciliation rule
    CreateRule,
    /// Create a new account with appropriate currency
    CreateAccount,
    /// Mark staging entry as void
    VoidStagingEntry,
    /// Mark transaction and all entries as void
    VoidTransaction,
    /// Process duplicate staging entry (bypass duplicate checks)
    ProcessDuplicateStagingEntry,
    /// Resolve mismatched transaction by adding new balancing entries
    CreateEntries,
    /// Replace specific entry in mismatched transaction
    ReplaceEntries,
    /// Force reconcile by ignoring non-critical field mismatches
    ForceReconcile,
    /// Link staging entries to replace expected entries in transaction
    LinkStagingEntriesToTransaction,
    /// Edit staging entry details or account
    EditStagingEntry,
}
/// Status for Search Duplicate
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SearchDuplicateStatus {
    Active,
    Archived,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SearchGroupStatus {
    Active,
    Archived,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::VariantNames,
    ToSchema,
)]
#[recon_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReconRuleAccountType {
    Source,
    Target,
}

/// Granularity at which an overview time series is bucketed.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Granularity {
    Hour,
    Day,
    Week,
    Month,
}
