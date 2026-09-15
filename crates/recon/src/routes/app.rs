use actix_multipart::form::MultipartFormConfig;
use actix_web::web;
use actix_web::Scope;
use tokio::sync::watch;
use utoipa_swagger_ui::SwaggerUi;

use common_enums::ReconProcessorStatus;
use recon_interfaces::secrets_interface::secret_state::RawSecret;

use crate::config::{Config, IngestionServiceConfig};
use crate::db::Store;
use crate::openapi::create_openapi;
use crate::routes::{search_entry_key, search_expectation, search_group};
use external_services::{
    faas::FaasHandler, file_storage::FileStorageHandler, ApplicationEncryptionClient,
};

const INGESTION_UPLOAD_TOTAL_LIMIT_BYTES: usize = 25 * 1024 * 1024;
const INGESTION_UPLOAD_MEMORY_LIMIT_BYTES: usize = 25 * 1024 * 1024;

use super::{
    account, api_docs, audit_trail, auth, business_profile, entry, exception_management, health,
    ingestion, merchant_account, metadata_schema, overview, recon_engine, recon_rule, reporting,
    staging_entry, transaction, transformation,
};

#[derive(Clone)]
pub struct IngestionState {
    pub config: IngestionServiceConfig,
    pub sftp_file_storage: FileStorageHandler,
}

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub config: Config<RawSecret>,
    pub file_storage: FileStorageHandler,
    pub faas: FaasHandler,
    pub ingestion: IngestionState,
    pub recon_processor_status: watch::Sender<ReconProcessorStatus>,
    pub encryption_client: ApplicationEncryptionClient,
    pub user_client: external_services::UserClient,
}

pub struct Health;

impl Health {
    pub fn server(state: AppState) -> Scope {
        web::scope("/health")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::get().to(health::get_health)))
    }
}

pub struct MerchantAccount;

impl MerchantAccount {
    pub fn server(state: AppState) -> Scope {
        web::scope("/merchant_accounts")
            .app_data(web::Data::new(state))
            .service(
                web::resource("").route(web::post().to(merchant_account::create_merchant_account)),
            )
            .service(web::scope("/{merchant_id}").service(
                web::resource("").route(web::get().to(merchant_account::get_merchant_account)),
            ))
    }
}

pub struct BusinessProfile;

impl BusinessProfile {
    pub fn server(state: AppState) -> Scope {
        web::scope("/business_profiles")
            .app_data(web::Data::new(state))
            .service(
                web::resource("")
                    .route(web::get().to(business_profile::list_business_profiles))
                    .route(web::post().to(business_profile::create_business_profile)),
            )
            .service(
                web::scope("/{profile_id}")
                    .service(
                        web::resource("")
                            .route(web::get().to(business_profile::get_business_profile)),
                    )
                    .service(
                        web::resource("/recon_engine/status")
                            .route(web::get().to(business_profile::get_recon_status)),
                    ),
            )
    }
}

pub struct Auth;

impl Auth {
    pub fn server(state: AppState) -> Scope {
        web::scope("/auth")
            .app_data(web::Data::new(state))
            .service(web::resource("bootstrap").route(web::post().to(auth::bootstrap)))
            .service(
                web::resource("bootstrap_with_ids")
                    .route(web::post().to(auth::bootstrap_with_merchant_id_profile_id)),
            )
    }
}

pub struct Account;

impl Account {
    pub fn server(state: AppState) -> Scope {
        web::scope("/accounts")
            .app_data(web::Data::new(state))
            .service(
                web::resource("")
                    .route(web::post().to(account::create_account))
                    .route(web::get().to(account::list_accounts_by_profile_id)),
            )
            .service(
                web::scope("/{account_id}")
                    .service(
                        web::resource("")
                            .route(web::get().to(account::get_account))
                            .route(web::put().to(account::update_account)),
                    )
                    .service(
                        web::resource("/entries")
                            .route(web::get().to(entry::list_entries_by_account_id)),
                    ),
            )
    }
}

pub struct StagingEntry;

impl StagingEntry {
    pub fn server(state: AppState) -> Scope {
        web::scope("/staging_entries")
            .app_data(web::Data::new(state.clone()))
            .service(
                web::resource("")
                    .route(web::post().to(staging_entry::create_staging_entry))
                    .route(web::get().to(staging_entry::list_staging_entries_by_profile_id)),
            )
            .service(
                web::scope("/v2")
                    .service(
                        web::resource("").route(
                            web::post().to(staging_entry::create_staging_entry_with_validation),
                        ),
                    )
                    .service(
                        web::resource("/list")
                            .route(web::post().to(staging_entry::list_staging_entries_v2)),
                    ),
            )
            .service(
                web::resource("/bulk_operations")
                    .route(web::post().to(staging_entry::execute_staging_entry_bulk_operations)),
            )
            .service(
                web::scope("/{id}")
                    .service(
                        web::resource("")
                            .route(web::get().to(staging_entry::get_staging_entry_by_id))
                            .route(web::put().to(staging_entry::update_staging_entry)),
                    )
                    .service(
                        web::resource("/void")
                            .route(web::put().to(staging_entry::void_staging_entry)),
                    ),
            )
    }
}

pub struct Entry;

impl Entry {
    pub fn server(state: AppState) -> Scope {
        web::scope("/entries")
            .app_data(web::Data::new(state.clone()))
            .service(web::resource("").route(web::post().to(entry::create_entry)))
            .service(web::resource("/list").route(web::post().to(entry::list_entries)))
            .service(web::resource("/{entry_id}").route(web::get().to(entry::get_entry_by_id)))
    }
}

pub struct Transaction;

impl Transaction {
    pub fn server(state: AppState) -> Scope {
        web::scope("/transactions")
            .app_data(web::Data::new(state))
            .service(
                web::resource("")
                    .route(web::post().to(transaction::create_transaction))
                    .route(web::get().to(transaction::list_transactions)),
            )
            .service(
                // Cursor Pagination should ensure a single point of sorting.
                web::resource("/v2/list").route(web::post().to(transaction::list_transactions_v2)),
            )
            .service(
                web::resource("/bulk_operations")
                    .route(web::post().to(transaction::execute_transaction_bulk_operations)),
            )
            .service(
                web::scope("/{id}")
                    .service(web::resource("").route(web::get().to(transaction::get_transaction)))
                    .service(
                        web::resource("/void").route(web::put().to(transaction::void_transaction)),
                    )
                    .service(
                        web::resource("/entries")
                            .route(web::get().to(entry::list_entries_by_transaction_id)),
                    ),
            )
    }
}

pub struct Overview;

impl Overview {
    pub fn server(state: AppState) -> Scope {
        web::scope("/overview")
            .app_data(web::Data::new(state))
            .service(
                web::scope("/transactions")
                    .service(
                        web::resource("").route(web::get().to(overview::get_transactions_overview)),
                    )
                    .service(
                        web::resource("/time_series")
                            .route(web::get().to(overview::get_transactions_time_series)),
                    )
                    .service(
                        web::resource("/rule_account_breakdown").route(
                            web::get().to(overview::get_transactions_rule_account_breakdown),
                        ),
                    ),
            )
            .service(web::scope("/staging_entries").service(
                web::resource("").route(web::get().to(overview::get_staging_entries_overview)),
            ))
    }
}

pub struct ReconRule;

impl ReconRule {
    pub fn server(state: AppState) -> Scope {
        web::scope("/recon_rules")
            .app_data(web::Data::new(state))
            .service(
                web::scope("/v2")
                    .service(
                        web::resource("")
                            .route(web::post().to(recon_rule::create_recon_rule_v2))
                            .route(web::get().to(recon_rule::list_recon_rules_by_profile_id_v2)),
                    )
                    .service(
                        web::resource("/{rule_id}")
                            .route(web::get().to(recon_rule::get_recon_rule_v2_by_id)),
                    ),
            )
            .service(
                web::resource("/{rule_id}").route(web::put().to(recon_rule::update_recon_rule)),
            )
    }
}

pub struct ReconEngine;

impl ReconEngine {
    pub fn server(state: AppState) -> Scope {
        web::scope("/recon_engine")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/processor/status")
                    .route(web::get().to(recon_engine::get_processor_status))
                    .route(web::post().to(recon_engine::set_processor_status)),
            )
            .service(
                web::scope("/{id}")
                    .service(
                        web::resource("/process")
                            .route(web::post().to(recon_engine::process_staging_entry)),
                    )
                    .service(
                        web::resource("/process_duplicate")
                            .route(web::post().to(recon_engine::process_duplicate_staging_entry)),
                    ),
            )
    }
}

pub struct Ingestion;

impl Ingestion {
    pub fn server(state: AppState) -> Scope {
        web::scope("/ingestions")
            .app_data(web::Data::new(state))
            // /ingestions/config and /ingestions/config/{ingestion_id}
            .service(
                web::scope("/config")
                    .service(
                        web::resource("")
                            .route(web::post().to(ingestion::create_ingestion_config))
                            .route(web::get().to(ingestion::list_ingestion_configs)),
                    )
                    .service(
                        web::resource("/{ingestion_id}")
                            .route(web::get().to(ingestion::get_ingestion_config))
                            .route(web::put().to(ingestion::update_ingestion_config)),
                    ),
            )
            .service(
                web::scope("/history")
                    .service(
                        web::resource("").route(web::get().to(ingestion::list_ingestion_history)),
                    )
                    .service(
                        web::scope("/{id}")
                            .service(web::resource("").route(
                                web::get().to(ingestion::find_ingestion_config_and_history_by_id),
                            ))
                            .service(
                                web::resource("/download")
                                    .route(web::get().to(ingestion::download_ingested_file)),
                            ),
                    ),
            )
            // /ingestions/{ingestion_id}/trigger and /ingestions/{ingestion_id}/upload
            .service(
                web::resource("/{ingestion_id}/trigger")
                    .route(web::post().to(ingestion::trigger_ingestion)),
            )
            .service(
                web::resource("/{ingestion_id}/upload")
                    .app_data(
                        MultipartFormConfig::default()
                            .total_limit(INGESTION_UPLOAD_TOTAL_LIMIT_BYTES)
                            .memory_limit(INGESTION_UPLOAD_MEMORY_LIMIT_BYTES),
                    )
                    .route(web::post().to(ingestion::upload_file_to_ingestion)),
            )
    }
}

pub struct Transformation;

impl Transformation {
    pub fn server(state: AppState) -> Scope {
        web::scope("/transformations")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/{ingestion_id}/{ingestion_history_id}/trigger")
                    .route(web::post().to(transformation::execute_transformation)),
            )
            .service(
                web::resource("/history/{transformation_history_id}/reports/{report_format}")
                    .route(web::get().to(transformation::download_transformation_report)),
            )
            .service(web::resource("/history/{transformation_history_id}").route(
                web::get().to(transformation::find_transformation_history_with_config_by_id),
            ))
            .service(
                web::scope("/configs")
                    .service(web::resource("").route(
                        web::get().to(transformation::list_transformation_configs_by_profile_id),
                    ))
                    .service(
                        web::scope("/{transformation_id}")
                            .service(
                                web::resource("")
                                    .route(
                                        web::get()
                                            .to(transformation::get_transformation_config_by_id),
                                    )
                                    .route(
                                        web::put().to(transformation::update_transformation_config),
                                    ),
                            )
                            .service(
                                web::resource("/metadata_schema").route(
                                    web::get().to(
                                        transformation::get_metadata_schema_by_transformation_id,
                                    ),
                                ),
                            ),
                    ),
            )
            .service(
                web::scope("/v2/configs").service(
                    web::resource("")
                        .route(web::post().to(transformation::create_transformation_config_v2)),
                ),
            )
            .service(
                web::resource("/history").route(
                    web::get()
                        .to(transformation::list_transformation_history_with_config_by_profile_id),
                ),
            )
    }
}

pub struct Report;

impl Report {
    pub fn server(state: AppState) -> Scope {
        web::scope("/report")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/transactions")
                    .route(web::post().to(reporting::trigger_transaction_report)),
            )
            .service(
                web::resource("/exceptions")
                    .route(web::post().to(reporting::trigger_exception_report)),
            )
    }
}

pub struct MetadataSchema;

impl MetadataSchema {
    pub fn server(state: AppState) -> Scope {
        web::scope("/metadata_schemas")
            .app_data(web::Data::new(state))
            .service(
                web::resource("")
                    .route(web::post().to(metadata_schema::create_metadata_schema))
                    .route(web::get().to(metadata_schema::list_metadata_schemas)),
            )
            .service(
                web::resource("/{schema_id}")
                    .route(web::get().to(metadata_schema::get_metadata_schema_by_id))
                    .route(web::put().to(metadata_schema::update_metadata_schema)),
            )
    }
}

pub struct SearchGroup;

impl SearchGroup {
    pub fn server(state: AppState) -> Scope {
        web::scope("/search_groups")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/{search_group_id}")
                    .route(web::get().to(search_group::get_search_group_by_id)),
            )
            .service(
                web::resource("/primary_transaction/{primary_transaction_id}")
                    .route(web::get().to(search_group::get_search_group_by_primary_transaction_id)),
            )
    }
}

pub struct SearchEntryKey;

impl SearchEntryKey {
    pub fn server(state: AppState) -> Scope {
        web::scope("/search_entry_keys")
            .app_data(web::Data::new(state))
            .service(
                web::resource("")
                    .route(web::post().to(search_entry_key::list_search_entry_keys_by_entry_ids)),
            )
    }
}

pub struct SearchExpectation;

impl SearchExpectation {
    pub fn server(state: AppState) -> Scope {
        web::scope("/search_expectations")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/primary_transaction/{primary_transaction_id}")
                    .route(web::get().to(
                        search_expectation::list_search_expectations_by_primary_transaction_id,
                    )),
            )
    }
}

pub struct ExceptionManagement;

impl ExceptionManagement {
    #[allow(deprecated)]
    pub fn server(state: AppState) -> Scope {
        web::scope("/exception_management")
            .app_data(web::Data::new(state))
            .service(
                web::scope("/staging_entries").service(
                    web::resource("/{id}/resolutions")
                        .route(web::get().to(exception_management::get_staging_entry_resolutions)),
                ),
            )
            .service(
                web::scope("/transactions").service(
                    web::scope("/{id}")
                        .service(web::resource("/resolutions").route(
                            web::get().to(exception_management::get_transaction_resolutions),
                        ))
                        .service(web::resource("/force_reconcile").route(
                            web::put().to(exception_management::force_reconcile_transaction),
                        ))
                        // TODO: remove the deprecated unpaginated route after clients migrate to v2.
                        .service(
                            web::resource("/linkable_staging_entries").route(web::get().to(
                                exception_management::list_linkable_staging_entries_for_transaction,
                            )),
                        )
                        .service(
                            web::resource("/linkable_staging_entries/v2/list").route(web::post().to(
                                exception_management::list_linkable_staging_entries_for_transaction_v2,
                            )),
                        )
                        .service(web::resource("/manual_reconciliation").route(
                            web::post().to(exception_management::manual_transaction_reconciliation),
                        )),
                ),
            )
            .service(
                web::resource("/validate_entry_data")
                    .route(web::post().to(exception_management::validate_raw_entry_data)),
            )
            .service(
                web::resource("/assignees")
                    .route(web::get().to(exception_management::list_assignees)),
            )
    }
}

pub struct AuditTrail;

impl AuditTrail {
    pub fn server(state: AppState) -> Scope {
        web::scope("/audit_trail")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::get().to(audit_trail::get_audit_trail)))
    }
}

pub struct ApiDocs;

impl ApiDocs {
    pub fn server(state: AppState) -> Scope {
        web::scope("/api_docs")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/openapi.json").route(web::get().to(api_docs::get_openapi_spec)),
            )
            .service(SwaggerUi::new("/ui/{_:.*}").url("../openapi.json", create_openapi()))
    }
}
