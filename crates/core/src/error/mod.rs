use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoodleError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Outlook error: {0}")]
    Outlook(String),

    #[error("AI error: {0}")]
    AI(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, NoodleError>;
