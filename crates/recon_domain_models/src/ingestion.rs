use crate::behaviour::ReverseConversion;
use crate::encryption_handler::EncryptionHandler;
use crate::{behaviour::Conversion, encryption::ApplicationEncryptionInterface};
use api_models::ingestion::{
    IngestionConfigData as IngestionConfigDataApi,
    IngestionConnectionData as IngestionConnectionDataApi,
};
use common_utils::{
    errors::{CustomResult, ValidationError},
    id_type::MerchantId,
};
use diesel_models::ingestion_config::IngestionConfigNew;
use diesel_models::ingestion_config::{
    adyen::AdyenBasicAuth as DieselAdyenBasicAuth,
    adyen::AdyenIngestionConfig as DieselAdyenIngestionConfig,
    sftp_internal::SftpInternalIngestionConfig as DieselSftpInternalIngestionConfig,
    IngestionConfig as DieselIngestionConfig, IngestionConfigData as DieselIngestionConfigData,
    IngestionConfigPsp as DieselIngestionConfigPsp, IngestionConfigV1 as DieselIngestionConfigV1,
};
use masking::ExposeInterface;
use serde::Serialize;
use time::PrimitiveDateTime;

pub mod adyen;
pub mod sftp_internal;

#[derive(Clone, Debug, Serialize)]
pub struct DomainIngestionConfig {
    pub ingestion_id: String,
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub account_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub config: DomainIngestionConfigData,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "ingestion_config_version", rename_all = "snake_case")]
pub enum DomainIngestionConfigData {
    V1(IngestionConfigV1),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "ingestion_type", rename_all = "snake_case")]
pub enum IngestionConfigV1 {
    Psp(IngestionConfigPsp),
    Manual,
    SftpInternal(sftp_internal::SftpInternalIngestionConfig),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "psp", rename_all = "snake_case")]
pub enum IngestionConfigPsp {
    Adyen(adyen::AdyenIngestionConfig),
}

/// Convert API model to Diesel model using EncryptionHandler
pub async fn to_diesel_config(
    api_config: IngestionConfigDataApi,
    encryption_handler: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> CustomResult<DieselIngestionConfigData, ValidationError> {
    match api_config {
        IngestionConfigDataApi::Adyen {
            hmac_secret,
            webhook_basic_auth_username,
            webhook_basic_auth_password,
            report_basic_auth_username,
            report_basic_auth_password,
        } => {
            let encrypted_hmac = encryption_handler
                .encrypt_string(hmac_secret.expose())
                .await?;
            let encrypted_webhook_password = encryption_handler
                .encrypt_string(webhook_basic_auth_password.expose())
                .await?;

            let encrypted_report_password = encryption_handler
                .encrypt_string(report_basic_auth_password.expose())
                .await?;
            Ok(DieselIngestionConfigData::V1(DieselIngestionConfigV1::Psp(
                DieselIngestionConfigPsp::Adyen(DieselAdyenIngestionConfig {
                    hmac_secret: encrypted_hmac.into(),
                    webhook_basic_auth: DieselAdyenBasicAuth {
                        username: webhook_basic_auth_username,
                        password: encrypted_webhook_password.into(),
                    },
                    report_basic_auth: DieselAdyenBasicAuth {
                        username: report_basic_auth_username,
                        password: encrypted_report_password.into(),
                    },
                }),
            )))
        }
        IngestionConfigDataApi::SftpInternal { file_path } => Ok(DieselIngestionConfigData::V1(
            DieselIngestionConfigV1::SftpInternal(DieselSftpInternalIngestionConfig { file_path }),
        )),
        IngestionConfigDataApi::Manual => Ok(DieselIngestionConfigData::V1(
            DieselIngestionConfigV1::Manual,
        )),
    }
}

#[async_trait::async_trait]
impl Conversion for DomainIngestionConfig {
    type DstType = DieselIngestionConfig;
    type NewDstType = IngestionConfigNew;

    /// Converts diesel model back to domain model (decrypts sensitive fields from database)
    async fn convert_back(
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
        item: Self::DstType,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let domain_config = item.config.convert_back(encryptor).await?;

        Ok(Self {
            ingestion_id: item.id,
            merchant_id: item.merchant_id,
            profile_id: item.profile_id,
            account_id: item.account_id,
            name: item.name,
            description: item.description,
            is_active: item.is_active,
            config: domain_config,
            created_at: item.created_at,
            last_modified_at: item.last_modified_at,
        })
    }

    /// Convert domain ingestion config to diesel IngestionConfigNew for insertion
    fn construct_new(self) -> Self::NewDstType {
        IngestionConfigNew {
            id: self.ingestion_id,
            profile_id: self.profile_id,
            account_id: self.account_id,
            name: self.name,
            description: self.description,
            config: self.config.construct_new(),
            is_active: self.is_active,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            merchant_id: self.merchant_id,
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        DieselIngestionConfig {
            id: self.ingestion_id,
            profile_id: self.profile_id,
            account_id: self.account_id,
            name: self.name,
            description: self.description,
            config: self.config.convert(),
            is_active: self.is_active,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            merchant_id: self.merchant_id,
        }
    }
}

#[async_trait::async_trait]
impl Conversion for DomainIngestionConfigData {
    type DstType = DieselIngestionConfigData;
    type NewDstType = DieselIngestionConfigData;

    async fn convert_back(
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
        item: Self::DstType,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        match item {
            DieselIngestionConfigData::V1(v1) => {
                let domain_v1 = match v1 {
                    DieselIngestionConfigV1::Psp(psp) => {
                        let domain_psp = match psp {
                            DieselIngestionConfigPsp::Adyen(adyen) => {
                                let adyen_config = adyen.convert_back(encryptor).await?;
                                IngestionConfigPsp::Adyen(adyen_config)
                            }
                        };
                        IngestionConfigV1::Psp(domain_psp)
                    }
                    DieselIngestionConfigV1::SftpInternal(sftp) => IngestionConfigV1::SftpInternal(
                        sftp_internal::SftpInternalIngestionConfig {
                            file_path: sftp.file_path,
                        },
                    ),
                    DieselIngestionConfigV1::Manual => IngestionConfigV1::Manual,
                };
                Ok(Self::V1(domain_v1))
            }
        }
    }

    fn construct_new(self) -> Self::NewDstType {
        match self {
            Self::V1(v1) => match v1 {
                IngestionConfigV1::Psp(psp) => match psp {
                    IngestionConfigPsp::Adyen(adyen) => {
                        DieselIngestionConfigData::V1(DieselIngestionConfigV1::Psp(
                            DieselIngestionConfigPsp::Adyen(adyen.construct_new()),
                        ))
                    }
                },
                IngestionConfigV1::SftpInternal(sftp) => DieselIngestionConfigData::V1(
                    DieselIngestionConfigV1::SftpInternal(DieselSftpInternalIngestionConfig {
                        file_path: sftp.file_path,
                    }),
                ),
                IngestionConfigV1::Manual => {
                    DieselIngestionConfigData::V1(DieselIngestionConfigV1::Manual)
                }
            },
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        match self {
            Self::V1(v1) => match v1 {
                IngestionConfigV1::Psp(psp) => match psp {
                    IngestionConfigPsp::Adyen(adyen) => {
                        DieselIngestionConfigData::V1(DieselIngestionConfigV1::Psp(
                            DieselIngestionConfigPsp::Adyen(adyen.convert()),
                        ))
                    }
                },
                IngestionConfigV1::SftpInternal(sftp) => DieselIngestionConfigData::V1(
                    DieselIngestionConfigV1::SftpInternal(DieselSftpInternalIngestionConfig {
                        file_path: sftp.file_path,
                    }),
                ),
                IngestionConfigV1::Manual => {
                    DieselIngestionConfigData::V1(DieselIngestionConfigV1::Manual)
                }
            },
        }
    }
}

impl DomainIngestionConfigData {
    pub fn to_api_connection_data(self) -> IngestionConnectionDataApi {
        match self {
            Self::V1(v1) => match v1 {
                IngestionConfigV1::Psp(psp) => match psp {
                    IngestionConfigPsp::Adyen(adyen) => IngestionConnectionDataApi::Adyen {
                        hmac_secret: adyen.hmac_secret.clone(),
                        webhook_basic_auth_username: adyen.webhook_basic_auth.username,
                        webhook_basic_auth_password: adyen.webhook_basic_auth.password,
                        report_basic_auth_username: adyen.report_basic_auth.username,
                        report_basic_auth_password: adyen.report_basic_auth.password,
                    },
                },
                IngestionConfigV1::SftpInternal(sftp) => IngestionConnectionDataApi::SftpInternal {
                    file_path: sftp.file_path,
                },
                IngestionConfigV1::Manual => IngestionConnectionDataApi::Manual,
            },
        }
    }
}
