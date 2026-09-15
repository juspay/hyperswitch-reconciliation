use api_models::account::GetAccountResponse;
use common_enums::BalanceDirection;
use common_types::{amount::AmountResponse, units::MinorUnit};
use diesel_models::account::Account;
use error_stack::ResultExt;

use crate::errors::account::{AccountError, AccountResult};

/// Calculate pending balance for an account
fn calculate_pending_balance(account: &Account) -> MinorUnit {
    let initial_balance = account.initial_balance;

    match account.balance_direction {
        BalanceDirection::Debit => {
            // Debit accounts: increase with debits, decrease with credits
            initial_balance + account.pending_debits - account.pending_credits
        }
        BalanceDirection::Credit => {
            // Credit accounts: increase with credits, decrease with debits
            initial_balance + account.pending_credits - account.pending_debits
        }
    }
}

/// Calculate posted balance for an account
fn calculate_posted_balance(account: &Account) -> MinorUnit {
    let initial_balance = account.initial_balance;

    match account.balance_direction {
        BalanceDirection::Debit => {
            // Debit accounts: increase with debits, decrease with credits
            initial_balance + account.posted_debits - account.posted_credits
        }
        BalanceDirection::Credit => {
            // Credit accounts: increase with credits, decrease with debits
            initial_balance + account.posted_credits - account.posted_debits
        }
    }
}

/// Calculate expected balance for an account
fn calculate_expected_balance(account: &Account) -> MinorUnit {
    match account.balance_direction {
        BalanceDirection::Debit => {
            // Debit accounts: increase with debits, decrease with credits
            account.expected_debits - account.expected_credits
        }
        BalanceDirection::Credit => {
            // Credit accounts: increase with credits, decrease with debits
            account.expected_credits - account.expected_debits
        }
    }
}

/// Calculate mismatched balance for an account
fn calculate_mismatched_balance(account: &Account) -> MinorUnit {
    match account.balance_direction {
        BalanceDirection::Debit => {
            // Debit accounts: increase with debits, decrease with credits
            account.mismatched_debits - account.mismatched_credits
        }
        BalanceDirection::Credit => {
            // Credit accounts: increase with credits, decrease with debits
            account.mismatched_credits - account.mismatched_debits
        }
    }
}

/// Calculate matched balance for an account
fn calculate_matched_balance(account: &Account) -> MinorUnit {
    match account.balance_direction {
        BalanceDirection::Debit => {
            // Debit accounts: increase with debits, decrease with credits
            account.matched_debits - account.matched_credits
        }
        BalanceDirection::Credit => {
            // Credit accounts: increase with credits, decrease with debits
            account.matched_credits - account.matched_debits
        }
    }
}

pub trait AccountExt {
    fn to_account_response(self) -> AccountResult<GetAccountResponse>;
}

impl AccountExt for Account {
    fn to_account_response(self) -> AccountResult<GetAccountResponse> {
        // Calculate balances using separate functions
        let calculated_pending = calculate_pending_balance(&self);
        let calculated_posted = calculate_posted_balance(&self);
        let calculated_expected = calculate_expected_balance(&self);
        let calculated_mismatched = calculate_mismatched_balance(&self);
        let calculated_matched = calculate_matched_balance(&self);

        // Convert balance amounts from minor units to AmountResponse for API response
        let initial_balance = AmountResponse::from_minor_unit(self.initial_balance, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let pending_balance = AmountResponse::from_minor_unit(calculated_pending, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let posted_balance = AmountResponse::from_minor_unit(calculated_posted, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let expected_balance = AmountResponse::from_minor_unit(calculated_expected, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let mismatched_balance =
            AmountResponse::from_minor_unit(calculated_mismatched, self.currency)
                .change_context(AccountError::InternalServerError)?;
        let matched_balance = AmountResponse::from_minor_unit(calculated_matched, self.currency)
            .change_context(AccountError::InternalServerError)?;

        // Convert debit/credit components to AmountResponse for detailed breakdown
        let posted_debits = AmountResponse::from_minor_unit(self.posted_debits, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let posted_credits = AmountResponse::from_minor_unit(self.posted_credits, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let pending_debits = AmountResponse::from_minor_unit(self.pending_debits, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let pending_credits = AmountResponse::from_minor_unit(self.pending_credits, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let expected_debits = AmountResponse::from_minor_unit(self.expected_debits, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let expected_credits =
            AmountResponse::from_minor_unit(self.expected_credits, self.currency)
                .change_context(AccountError::InternalServerError)?;
        let mismatched_debits =
            AmountResponse::from_minor_unit(self.mismatched_debits, self.currency)
                .change_context(AccountError::InternalServerError)?;
        let mismatched_credits =
            AmountResponse::from_minor_unit(self.mismatched_credits, self.currency)
                .change_context(AccountError::InternalServerError)?;
        let matched_debits = AmountResponse::from_minor_unit(self.matched_debits, self.currency)
            .change_context(AccountError::InternalServerError)?;
        let matched_credits = AmountResponse::from_minor_unit(self.matched_credits, self.currency)
            .change_context(AccountError::InternalServerError)?;

        Ok(GetAccountResponse {
            account_id: self.id,
            account_name: self.name,
            account_type: self.balance_direction,
            initial_balance,
            pending_balance,
            posted_balance,
            matched_balance,
            expected_balance,
            mismatched_balance,
            posted_debits,
            posted_credits,
            pending_debits,
            pending_credits,
            expected_debits,
            expected_credits,
            mismatched_debits,
            mismatched_credits,
            matched_debits,
            matched_credits,
            profile_id: self.profile_id,
            created_at: self.created_at,
        })
    }
}
