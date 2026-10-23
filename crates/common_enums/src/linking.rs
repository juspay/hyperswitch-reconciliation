use utoipa::ToSchema;

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
pub enum LinkingProcessorStatus {
    Running,
    Stopped,
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
    Default,
)]
#[recon_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EntryLinkingStatus {
    #[default]
    Pending,
    Processed,
    Failed,
    NotApplicable,
}

impl EntryLinkingStatus {
    pub fn enqueuable_statuses() -> Vec<Self> {
        vec![Self::Failed]
    }
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
#[recon_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EntryLinkSide {
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
pub enum EntryLinkStatus {
    Provisional,
    Final,
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
#[recon_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EntryLinkKeyStatus {
    Open,
    Matched,
    PostedOpen,
    PostedMatched,
}

impl EntryLinkKeyStatus {
    pub fn is_posted(&self) -> bool {
        match self {
            Self::Open | Self::Matched => false,
            Self::PostedOpen | Self::PostedMatched => true,
        }
    }

    pub fn from_posted_status(is_posted: bool) -> Vec<Self> {
        if is_posted {
            vec![Self::PostedMatched, Self::PostedOpen]
        } else {
            vec![Self::Open, Self::Matched]
        }
    }
}
