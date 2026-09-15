use error_stack::Report;

#[derive(Debug, thiserror::Error)]
pub enum UserClientError {
    #[error("Internal Server Error while fetching user")]
    InternalServerError,
    #[error("Invalid JWT token was used")]
    Unauthorized,
    #[error("Not authorized to access this resource")]
    Forbidden,
}

pub type UserClientResult<T> = Result<T, Report<UserClientError>>;
