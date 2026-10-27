use storage_impl::search_expectation::SearchCriteria;

use crate::{
    define_recon_flow,
    domain::{
        recon_engine::rule_orchestration::{ConfirmationContext, TransactionContext},
        recon_engine::ProcessingResult,
    },
    routes::AppState,
};
use common_types::recon_rule::{
    entry_field::ExtractFromEntryField,
    strategies::{
        one_to_many_strategy::{
            OneToManySingleSingle, OneToManySingleSingleV1, OneToManyStrategy, OneToManyStrategyV1,
        },
        one_to_one_strategy::{
            OneToOneManyMany, OneToOneManyManyV1, OneToOneManySingle, OneToOneManySingleV1,
            OneToOneSingleMany, OneToOneSingleManyV1, OneToOneSingleSingle, OneToOneSingleSingleV1,
            OneToOneStrategy, OneToOneStrategyV1,
        },
    },
    ReconStrategy, ReconStrategyV1,
};
use diesel_models::transaction::Transaction;
use recon_domain_models::{
    search_group::DomainSearchGroup, staging_entry::DomainStagingEntry,
    transaction::DomainTransaction,
};

pub mod one_to_many_account;
pub mod one_to_one_account;

define_recon_flow! {
    OneToOneSingleSingleV1,
    OneToOneSingleManyV1,
    OneToOneManySingleV1,
    OneToOneManyManyV1,
    OneToManySingleSingleV1,
}

#[async_trait::async_trait]
pub trait ReconEngine {
    type PrepTypeTransaction: Send;
    type PrepTypeConfirmation: Send;
    type ContextTypeTransaction: Send;
    type ContextTypeConfirmation: Send;
    type StatusDeterminationInput: Send;
    type StatusDeterminationOutput: Send;

    async fn prep_transaction(
        self,
        state: &AppState,
        staging_entry: DomainStagingEntry,
        transaction_context: TransactionContext,
    ) -> ProcessingResult<ReconPrepTransaction>;

    async fn prep_confirmation(
        self,
        state: &AppState,
        staging_entry: DomainStagingEntry,
        confirmation_context: ConfirmationContext,
    ) -> ProcessingResult<ReconPrepConfirmation>;

    async fn create_transaction_context(
        self,
        state: &AppState,
        data: Self::PrepTypeTransaction,
    ) -> ProcessingResult<ReconContextTransaction>;

    async fn create_confirmation_context(
        self,
        state: &AppState,
        data: Self::PrepTypeConfirmation,
    ) -> ProcessingResult<ReconContextConfirmation>;

    async fn process_transaction_context(
        self,
        state: &AppState,
        context: Self::ContextTypeTransaction,
    ) -> ProcessingResult<Transaction>;

    async fn process_confirmation_context(
        self,
        state: &AppState,
        context: Self::ContextTypeConfirmation,
    ) -> ProcessingResult<Transaction>;

    fn get_search_criteria(
        &self,
        data: &impl ExtractFromEntryField,
        rule_id: &str,
    ) -> ProcessingResult<Vec<SearchCriteria>>;

    fn evaluate_trigger(&self, data: &impl ExtractFromEntryField) -> bool;

    async fn determine_status(
        &self,
        state: &AppState,
        data: Self::StatusDeterminationInput,
    ) -> ProcessingResult<Self::StatusDeterminationOutput>;
}

#[async_trait::async_trait]
pub trait ReconEngineSearchGroupInterface: ReconEngine {
    async fn create_transaction_context_without_search_group(
        &self,
        state: &AppState,
        data: Self::PrepTypeTransaction,
    ) -> ProcessingResult<Self::ContextTypeTransaction>;

    async fn create_transaction_context_with_search_group(
        &self,
        state: &AppState,
        data: Self::PrepTypeTransaction,
        search_group: DomainSearchGroup,
    ) -> ProcessingResult<Self::ContextTypeTransaction>;
}

pub trait ReconEngineIncrementalTransactionInterface: ReconEngine {
    fn check_incremental_transaction_append_eligibility(
        &self,
        transaction: &DomainTransaction,
        staging_entry: &DomainStagingEntry,
    ) -> bool;

    async fn create_incremental_transaction_context(
        &self,
        state: &AppState,
        data: Self::PrepTypeTransaction,
        transaction: DomainTransaction,
    ) -> ProcessingResult<Self::ContextTypeTransaction>;
}

pub trait ReconStrategyExt {
    fn get_recon_engine_type(&self) -> ReconEngineType;
}

impl ReconStrategyExt for ReconStrategy {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::V1(strategy_v1) => strategy_v1.get_recon_engine_type(),
        }
    }
}

impl ReconStrategyExt for ReconStrategyV1 {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::OneToOne(one_to_one_strategy) => one_to_one_strategy.get_recon_engine_type(),
            Self::OneToMany(one_to_many_strategy) => one_to_many_strategy.get_recon_engine_type(),
        }
    }
}

impl ReconStrategyExt for OneToOneStrategy {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::V1(one_to_one_strategy_v1) => one_to_one_strategy_v1.get_recon_engine_type(),
        }
    }
}

impl ReconStrategyExt for OneToOneStrategyV1 {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::SingleSingle(single_single_strategy) => {
                single_single_strategy.get_recon_engine_type()
            }
            Self::SingleMany(single_many_strategy) => single_many_strategy.get_recon_engine_type(),
            Self::ManySingle(many_single_strategy) => many_single_strategy.get_recon_engine_type(),
            Self::ManyMany(many_many_strategy) => many_many_strategy.get_recon_engine_type(),
        }
    }
}

impl ReconStrategyExt for OneToOneSingleSingle {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::V1(engine) => ReconEngineType::OneToOneSingleSingleV1(engine.clone()),
        }
    }
}

impl ReconStrategyExt for OneToOneSingleMany {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::V1(engine) => ReconEngineType::OneToOneSingleManyV1(engine.clone()),
        }
    }
}

impl ReconStrategyExt for OneToOneManySingle {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::V1(engine) => ReconEngineType::OneToOneManySingleV1(engine.clone()),
        }
    }
}

impl ReconStrategyExt for OneToOneManyMany {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::V1(engine) => ReconEngineType::OneToOneManyManyV1(engine.clone()),
        }
    }
}

impl ReconStrategyExt for OneToManyStrategy {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::V1(engine) => engine.get_recon_engine_type(),
        }
    }
}

impl ReconStrategyExt for OneToManyStrategyV1 {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::SingleSingle(single_single) => single_single.get_recon_engine_type(),
        }
    }
}

impl ReconStrategyExt for OneToManySingleSingle {
    fn get_recon_engine_type(&self) -> ReconEngineType {
        match self {
            Self::V1(engine) => ReconEngineType::OneToManySingleSingleV1(engine.clone()),
        }
    }
}
