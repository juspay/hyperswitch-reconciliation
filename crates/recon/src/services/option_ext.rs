use error_stack::Report;

use crate::errors::ReconAppError;

type Result<T, E> = core::result::Result<T, Report<E>>;

/// Extension trait for extracting required values from `Option<T>` with useful errors
pub trait OptionExt<T> {
    fn get_required_value(self, field_name: &'static str) -> Result<T, ReconAppError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn get_required_value(self, field_name: &'static str) -> Result<T, ReconAppError> {
        self.ok_or_else(|| {
            Report::new(ReconAppError::Unauthorized)
                .attach(format!("Missing required field or header: {field_name}"))
        })
    }
}
