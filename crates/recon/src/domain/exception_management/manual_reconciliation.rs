use std::collections::{HashMap, HashSet};

use api_models::transaction::{EntryOp, NewEntry, StagingEntryOverrides};
use common_enums::{BalanceDirection, Currency, DieselTransactionStatus, EntryStatus};
use common_types::{
    amount::AmountType,
    entry::{EntryData, EntryDataV2},
    metadata_schema::{ExposedKeys, ExposedKeysProvider, NO_EXPOSED_KEYS},
    staging_entry::{status::DomainStagingEntryStatus, StagingEntryData, StagingEntryDataV2},
    transaction::{
        ManuallyMatchedData, ManuallyMatchedDataV1, MatchedData, MatchedDataV1,
        PartiallyReconciledData, PartiallyReconciledDataV1, TransactionData, TransactionDataV3,
    },
};
use common_utils::auth::AuthContext;
use diesel_models::{
    entry::{Entry as DieselEntry, EntryNew},
    search_duplicate::SearchDuplicateNew,
    search_entry_key::SearchEntryKey,
    staging_entry::{
        StagingEntry as DieselStagingEntry, StagingEntryNew, StagingEntryVersionUpdate,
    },
    transaction::Transaction as DieselTransaction,
};
use error_stack::{Report, ResultExt};
use futures::{future, stream, StreamExt, TryStreamExt};
use masking::{ExposeInterface, PeekInterface};
use recon_domain_models::{
    behaviour::ReverseConversion, search_entry_key::SearchEntryKeyCreation,
    staging_entry::DomainStagingEntry, ApplicationEncryptionInterface, EncryptionHandler,
};
use storage_impl::{
    entry::EntryInterface, metadata_schema::MetadataSchemaInterface,
    search_entry_key::SearchEntryKeyInterface, transformation::TransformationConfigInterface,
};

use crate::{
    domain::recon_engine::process_duplicate::build_search_duplicate,
    errors::transaction::{TransactionError, TransactionResult},
    routes::AppState,
    utils::{exception_management::EntryOpsVecExt, metadata_schema::project_metadata},
};

const PREPARATION_MAX_CONCURRENT: usize = 5;

pub const MANUALLY_RECONCILABLE_STATUSES: [DieselTransactionStatus; 9] = [
    DieselTransactionStatus::OverAmountExpected,
    DieselTransactionStatus::OverAmountMismatch,
    DieselTransactionStatus::UnderAmountExpected,
    DieselTransactionStatus::UnderAmountMismatch,
    DieselTransactionStatus::DataMismatch,
    DieselTransactionStatus::CurrencyMismatch,
    DieselTransactionStatus::SplitMismatch,
    DieselTransactionStatus::Expected,
    DieselTransactionStatus::PartiallyReconciled,
];

pub struct ExistingEntry {
    pub entry: DieselEntry,
    pub search_keys: Vec<SearchEntryKey>,
}

pub struct PreparedStagingLink {
    pub snapshot: DomainStagingEntry,
    pub processed: StagingEntryNew,
    pub search_duplicate: SearchDuplicateNew,
}

pub struct PreparedEntry {
    pub entry: EntryNew,
    pub search_keys: Vec<SearchEntryKeyCreation>,
}

pub async fn prepare_staging_links(
    state: &AppState,
    auth_context: &AuthContext,
    entry_ops: &[EntryOp],
    transaction_currency: Currency,
    merchant_encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> TransactionResult<HashMap<String, PreparedStagingLink>> {
    let staging_entry_ids: Vec<String> = entry_ops
        .linked_staging_entry_ids()
        .into_iter()
        .map(str::to_string)
        .collect();
    if staging_entry_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let staging_entries = state
        .store
        .list_staging_entries_by_primary_ids_and_profile(
            &staging_entry_ids,
            &auth_context.profile_id,
            None,
        )
        .await
        .change_context(TransactionError::InternalServerError)?;
    let staging_entries_by_id: HashMap<&str, &DieselStagingEntry> = staging_entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();

    for staging_entry_id in &staging_entry_ids {
        let staging_entry = staging_entries_by_id
            .get(staging_entry_id.as_str())
            .ok_or_else(|| TransactionError::StagingEntryNotFound {
                staging_entry_id: staging_entry_id.clone(),
            })?;

        if !DomainStagingEntryStatus::from(staging_entry.status).is_linkable() {
            return Err(TransactionError::StagingEntryNotLinkable {
                staging_entry_id: staging_entry.id.clone(),
            }
            .into());
        }
        if staging_entry.currency != transaction_currency {
            return Err(TransactionError::StagingEntryCurrencyMismatch {
                staging_entry_id: staging_entry.id.clone(),
                staging_currency: staging_entry.currency,
                transaction_currency,
            }
            .into());
        }
    }

    let config_ids = staging_entries
        .iter()
        .map(|entry| entry.transformation_config_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let configs = state
        .store
        .list_transformation_configs_by_ids_and_profile_id(config_ids, &auth_context.profile_id)
        .await
        .change_context(TransactionError::InternalServerError)?;
    let schema_ids = configs
        .iter()
        .map(|config| config.metadata_schema_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let schemas = state
        .store
        .list_metadata_schemas_by_ids_and_profile_id(schema_ids, &auth_context.profile_id)
        .await
        .change_context(TransactionError::InternalServerError)?;
    let configs_by_id: HashMap<_, _> = configs
        .iter()
        .map(|config| (config.id.as_str(), config))
        .collect();
    let schemas_by_id: HashMap<_, _> = schemas
        .iter()
        .map(|schema| (schema.id.as_str(), schema))
        .collect();

    stream::iter(staging_entries)
        .map(|staging_entry| {
            let configs_by_id = &configs_by_id;
            let schemas_by_id = &schemas_by_id;
            async move {
                let config = configs_by_id
                    .get(staging_entry.transformation_config_id.as_str())
                    .ok_or(TransactionError::InternalServerError)?;
                let schema = schemas_by_id
                    .get(config.metadata_schema_id.as_str())
                    .ok_or(TransactionError::InternalServerError)?;
                let staging_entry_id = staging_entry.id.clone();
                let domain_staging_entry: DomainStagingEntry = staging_entry
                    .convert_back(merchant_encryptor)
                    .await
                    .change_context(TransactionError::InternalServerError)?;
                let processed = StagingEntryNew::new_versioned(
                    domain_staging_entry.clone().convert(),
                    StagingEntryVersionUpdate::ProcessedUpdate {
                        data: StagingEntryData::V2(StagingEntryDataV2::Processed),
                    },
                    Some(auth_context.user_id.clone()),
                );
                let search_duplicate = build_search_duplicate(
                    &domain_staging_entry,
                    &processed,
                    config,
                    schema,
                    merchant_encryptor,
                )
                .await
                .change_context(TransactionError::InternalServerError)?;
                Ok::<_, Report<TransactionError>>((
                    staging_entry_id,
                    PreparedStagingLink {
                        snapshot: domain_staging_entry,
                        processed,
                        search_duplicate,
                    },
                ))
            }
        })
        .buffer_unordered(PREPARATION_MAX_CONCURRENT)
        .try_collect()
        .await
}

impl PreparedEntry {
    pub fn projected(entry: EntryNew, exposed_keys: &ExposedKeys) -> Self {
        let search_keys = SearchEntryKeyCreation::rows_from_exposed_metadata(
            &entry.exposed_metadata,
            &exposed_keys.searchable,
        );

        Self { entry, search_keys }
    }

    pub fn carried(entry: EntryNew, search_keys: Vec<SearchEntryKey>) -> Self {
        let search_keys = search_keys
            .into_iter()
            .map(|key| SearchEntryKeyCreation {
                metadata_key: key.metadata_key,
                metadata_value: key.metadata_value,
            })
            .collect();

        Self { entry, search_keys }
    }
}

pub struct ConfigProjection {
    pub account_id: String,
    pub exposed_keys: ExposedKeys,
}

pub struct MetadataProjection {
    configs: HashMap<String, ConfigProjection>,
}

impl MetadataProjection {
    pub async fn resolve(
        state: &AppState,
        profile_id: &str,
        config_ids: HashSet<String>,
    ) -> TransactionResult<Self> {
        if config_ids.is_empty() {
            return Ok(Self {
                configs: HashMap::new(),
            });
        }

        let configs = state
            .store
            .list_transformation_configs_by_ids_and_profile_id(
                config_ids.into_iter().collect(),
                profile_id,
            )
            .await
            .change_context(TransactionError::InternalServerError)?;

        let schemas = state
            .store
            .list_metadata_schemas_by_ids_and_profile_id(
                configs
                    .iter()
                    .map(|config| config.metadata_schema_id.clone())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect(),
                profile_id,
            )
            .await
            .change_context(TransactionError::InternalServerError)?;

        let keys_by_schema: HashMap<String, ExposedKeys> = schemas
            .into_iter()
            .map(|schema| (schema.id, schema.schema_data.exposed_keys()))
            .collect();

        let configs = configs
            .into_iter()
            .map(|config| {
                keys_by_schema
                    .get(&config.metadata_schema_id)
                    .cloned()
                    .map(|exposed_keys| {
                        (
                            config.id,
                            ConfigProjection {
                                account_id: config.account_id,
                                exposed_keys,
                            },
                        )
                    })
                    .ok_or(TransactionError::InternalServerError)
            })
            .collect::<Result<_, _>>()?;

        Ok(Self { configs })
    }

    pub fn config(&self, config_id: &str) -> TransactionResult<&ConfigProjection> {
        self.configs.get(config_id).ok_or_else(|| {
            TransactionError::TransformationConfigNotFound {
                config_id: config_id.to_string(),
            }
            .into()
        })
    }

    pub fn exposed_keys(&self, transformation_id: Option<&str>) -> TransactionResult<&ExposedKeys> {
        transformation_id.map_or(Ok(&NO_EXPOSED_KEYS), |config_id| {
            self.config(config_id).map(|config| &config.exposed_keys)
        })
    }
}

pub async fn load_existing_entries_with_search_keys(
    state: &AppState,
    profile_id: &str,
    transaction: &DieselTransaction,
) -> TransactionResult<HashMap<String, ExistingEntry>> {
    let entries = state
        .store
        .list_entries_by_transaction_id_and_version(
            &transaction.transaction_id,
            Some(transaction.version),
        )
        .await
        .change_context(TransactionError::InternalServerError)?;

    let entry_ids: Vec<String> = entries.iter().map(|entry| entry.id.clone()).collect();

    let mut search_keys = state
        .store
        .list_search_entry_keys_by_entry_ids_and_profile_id(entry_ids, profile_id, None)
        .await
        .change_context(TransactionError::InternalServerError)?
        .into_iter()
        .fold(
            HashMap::<String, Vec<SearchEntryKey>>::new(),
            |mut keys_by_entry, key| {
                keys_by_entry
                    .entry(key.entry_id.clone())
                    .or_default()
                    .push(key);
                keys_by_entry
            },
        );

    Ok(entries
        .into_iter()
        .map(|entry| {
            let search_keys = search_keys.remove(&entry.id).unwrap_or_default();

            (entry.id.clone(), ExistingEntry { entry, search_keys })
        })
        .collect())
}

pub async fn build_new_entries(
    existing_entries: HashMap<String, ExistingEntry>,
    entry_ops: Vec<EntryOp>,
    transaction: &DieselTransaction,
    auth: &AuthContext,
    staging_links: &HashMap<String, PreparedStagingLink>,
    projection: &MetadataProjection,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> TransactionResult<Vec<PreparedEntry>> {
    let mut entries = carry_forward_entries(
        existing_entries,
        &entry_ops,
        transaction,
        projection,
        encryptor,
    )
    .await?;

    let added_entries: Vec<PreparedEntry> = stream::iter(entry_ops)
        .map(|op| build_added_entry(op, transaction, auth, staging_links, projection, encryptor))
        .buffer_unordered(PREPARATION_MAX_CONCURRENT)
        .try_filter_map(|entry| future::ready(Ok(entry)))
        .try_collect()
        .await?;

    entries.extend(added_entries);

    Ok(entries)
}

async fn build_added_entry(
    op: EntryOp,
    transaction: &DieselTransaction,
    auth: &AuthContext,
    staging_links: &HashMap<String, PreparedStagingLink>,
    projection: &MetadataProjection,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> TransactionResult<Option<PreparedEntry>> {
    match op {
        EntryOp::CreateEntry { entry } => {
            build_created_entry(entry, transaction, auth, projection, encryptor)
                .await
                .map(Some)
        }
        EntryOp::CreateWithStagingEntry {
            staging_entry_id,
            overrides,
        }
        | EntryOp::ReplaceWithStagingEntry {
            staging_entry_id,
            overrides,
            ..
        } => {
            let link = staging_links
                .get(&staging_entry_id)
                .ok_or(TransactionError::InternalServerError)?;

            build_linked_entry(link, overrides, transaction, auth, projection, encryptor)
                .await
                .map(Some)
        }
        EntryOp::UpdateEntry { .. }
        | EntryOp::MarkReceived { .. }
        | EntryOp::RemoveEntry { .. } => Ok(None),
    }
}

async fn carry_forward_entries(
    existing_entries: HashMap<String, ExistingEntry>,
    entry_ops: &[EntryOp],
    transaction: &DieselTransaction,
    projection: &MetadataProjection,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> TransactionResult<Vec<PreparedEntry>> {
    let ops_by_entry_id: HashMap<&str, &EntryOp> = entry_ops
        .iter()
        .filter_map(|op| match op {
            EntryOp::UpdateEntry { entry_id, .. }
            | EntryOp::MarkReceived { entry_id, .. }
            | EntryOp::ReplaceWithStagingEntry { entry_id, .. }
            | EntryOp::RemoveEntry { entry_id } => Some((entry_id.as_str(), op)),
            EntryOp::CreateEntry { .. } | EntryOp::CreateWithStagingEntry { .. } => None,
        })
        .collect();

    stream::iter(existing_entries)
        .map(|(entry_id, existing)| {
            let op = ops_by_entry_id.get(entry_id.as_str()).copied();

            carry_forward_entry(existing, op, transaction, projection, encryptor)
        })
        .buffer_unordered(PREPARATION_MAX_CONCURRENT)
        .try_filter_map(|entry| future::ready(Ok(entry)))
        .try_collect()
        .await
}

async fn carry_forward_entry(
    existing: ExistingEntry,
    op: Option<&EntryOp>,
    transaction: &DieselTransaction,
    projection: &MetadataProjection,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> TransactionResult<Option<PreparedEntry>> {
    let ExistingEntry { entry, search_keys } = existing;

    let (overrides, status, data) = match op {
        Some(EntryOp::RemoveEntry { .. }) | Some(EntryOp::ReplaceWithStagingEntry { .. }) => {
            return Ok(None)
        }
        Some(EntryOp::MarkReceived { overrides, .. }) => (
            Some(overrides),
            EntryStatus::Pending,
            EntryData::V2(EntryDataV2::Pending),
        ),
        Some(EntryOp::UpdateEntry { overrides, .. }) => (Some(overrides), entry.status, entry.data),
        Some(EntryOp::CreateEntry { .. }) | Some(EntryOp::CreateWithStagingEntry { .. }) | None => {
            (None, entry.status, entry.data)
        }
    };

    let overridden_transformation_id =
        overrides.and_then(|overrides| overrides.transformation_id.clone());
    let supplied_metadata = overrides.and_then(|overrides| overrides.metadata.clone());
    let reproject = supplied_metadata.is_some() || overridden_transformation_id.is_some();
    let transformation_id = overridden_transformation_id.or(entry.transformation_config_id);

    let exposed_keys = if reproject {
        Some(projection.exposed_keys(transformation_id.as_deref())?)
    } else {
        None
    };

    let (metadata, exposed_metadata) = match (exposed_keys, supplied_metadata) {
        (Some(exposed_keys), Some(metadata)) => {
            let raw_metadata = metadata.expose();
            let exposed_metadata = project_metadata(&raw_metadata, &exposed_keys.exposed);
            let encrypted = encryptor
                .encrypt_json_value(raw_metadata)
                .await
                .change_context(TransactionError::InternalServerError)?;

            (encrypted.into(), exposed_metadata)
        }
        (Some(exposed_keys), None) => {
            let raw_metadata = encryptor
                .decrypt_json_value(entry.metadata.clone())
                .await
                .change_context(TransactionError::InternalServerError)?;

            (
                entry.metadata,
                project_metadata(raw_metadata.get_inner().peek(), &exposed_keys.exposed),
            )
        }
        (None, _) => (entry.metadata, entry.exposed_metadata),
    };

    let amount = match overrides.and_then(|overrides| overrides.amount) {
        Some(amount) => AmountType::from_major_unit(amount, transaction.currency)
            .change_context(TransactionError::InvalidAmount {
                details: "Invalid amount provided".to_string(),
            })?
            .value(),
        None => entry.amount,
    };

    let new_entry = EntryNew::new(
        entry.account_id,
        transaction.transaction_id.clone(),
        overrides
            .and_then(|overrides| overrides.entry_type)
            .unwrap_or(entry.balance_direction),
        amount,
        transaction.currency,
        status,
        overrides
            .and_then(|overrides| overrides.effective_at)
            .unwrap_or(entry.effective_at),
        entry.merchant_id,
        metadata,
        transaction.version + 1,
        data,
        entry.primary_staging_entry_id,
        transformation_id,
        overrides
            .and_then(|overrides| overrides.order_id.clone())
            .unwrap_or(entry.entry_identifier),
        exposed_metadata,
    );

    Ok(Some(match exposed_keys {
        Some(exposed_keys) => PreparedEntry::projected(new_entry, exposed_keys),
        None => PreparedEntry::carried(new_entry, search_keys),
    }))
}

async fn build_created_entry(
    new_entry: NewEntry,
    transaction: &DieselTransaction,
    auth: &AuthContext,
    projection: &MetadataProjection,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> TransactionResult<PreparedEntry> {
    let (account_id, entry_type, amount, effective_at, metadata, order_id, transformation_id, data) =
        match new_entry {
            NewEntry::Direct {
                account_id,
                entry_type,
                amount,
                effective_at,
                metadata,
                order_id,
                transformation_id,
            } => (
                account_id,
                entry_type,
                amount,
                effective_at,
                metadata,
                order_id,
                transformation_id,
                EntryData::V2(EntryDataV2::Pending),
            ),
            NewEntry::Expected {
                account_id,
                entry_type,
                amount,
                effective_at,
                metadata,
                order_id,
            } => (
                account_id,
                entry_type,
                amount,
                effective_at,
                metadata,
                order_id,
                None,
                EntryData::V2(EntryDataV2::Expected),
            ),
        };

    let amount = AmountType::from_major_unit(amount, transaction.currency)
        .change_context(TransactionError::InvalidAmount {
            details: "Invalid amount provided".to_string(),
        })?
        .value();

    let exposed_keys = projection.exposed_keys(transformation_id.as_deref())?;
    let raw_metadata = metadata.expose();
    let exposed_metadata = project_metadata(&raw_metadata, &exposed_keys.exposed);
    let metadata = encryptor
        .encrypt_json_value(raw_metadata)
        .await
        .change_context(TransactionError::InternalServerError)?
        .into();

    let entry = EntryNew::new(
        account_id,
        transaction.transaction_id.clone(),
        entry_type,
        amount,
        transaction.currency,
        (&data).into(),
        effective_at,
        auth.merchant_id.clone(),
        metadata,
        transaction.version + 1,
        data,
        None,
        transformation_id,
        order_id,
        exposed_metadata,
    );

    Ok(PreparedEntry::projected(entry, exposed_keys))
}

async fn build_linked_entry(
    link: &PreparedStagingLink,
    overrides: StagingEntryOverrides,
    transaction: &DieselTransaction,
    auth: &AuthContext,
    projection: &MetadataProjection,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> TransactionResult<PreparedEntry> {
    let staging = &link.snapshot;
    let exposed_keys = projection.exposed_keys(Some(&staging.transformation_id))?;

    let (metadata, exposed_metadata) = match overrides.metadata {
        Some(metadata) => {
            let raw_metadata = metadata.expose();
            let exposed_metadata = project_metadata(&raw_metadata, &exposed_keys.exposed);
            let encrypted = encryptor
                .encrypt_json_value(raw_metadata)
                .await
                .change_context(TransactionError::InternalServerError)?;

            (encrypted.into(), exposed_metadata)
        }
        None => (
            staging.metadata.clone().into(),
            staging.exposed_metadata.clone(),
        ),
    };

    let data = EntryData::V2(EntryDataV2::Pending);
    let entry = EntryNew::new(
        staging.account_id.clone(),
        transaction.transaction_id.clone(),
        staging.entry_type,
        staging.amount,
        transaction.currency,
        (&data).into(),
        overrides.effective_at.unwrap_or(staging.effective_at),
        auth.merchant_id.clone(),
        metadata,
        transaction.version + 1,
        data,
        Some(link.processed.id.clone()),
        Some(staging.transformation_id.clone()),
        overrides
            .order_id
            .unwrap_or_else(|| staging.order_id.clone()),
        exposed_metadata,
    );

    Ok(PreparedEntry::projected(entry, exposed_keys))
}

pub fn resolve_transaction_data(
    entries: &mut [PreparedEntry],
    reason: Option<String>,
) -> TransactionData {
    let has_expected = entries
        .iter()
        .any(|prepared| prepared.entry.status == EntryStatus::Expected);
    let (credit, debit) = entries
        .iter()
        .fold((0_i128, 0_i128), |(credit, debit), prepared| {
            let amount = i128::from(prepared.entry.amount.get_amount_as_i64());
            match prepared.entry.balance_direction {
                BalanceDirection::Credit => (credit + amount, debit),
                BalanceDirection::Debit => (credit, debit + amount),
            }
        });

    if !has_expected && credit == debit {
        for prepared in entries {
            prepared.entry.data = EntryData::V2(EntryDataV2::Matched);
            prepared.entry.status = EntryStatus::Matched;
        }
        TransactionData::V3(TransactionDataV3::Matched(MatchedData::V1(
            MatchedDataV1::Manual(ManuallyMatchedData::V1(ManuallyMatchedDataV1 { reason })),
        )))
    } else {
        TransactionData::V3(TransactionDataV3::PartiallyReconciled(
            PartiallyReconciledData::V1(PartiallyReconciledDataV1 { reason }),
        ))
    }
}
