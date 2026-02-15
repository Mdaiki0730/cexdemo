use thiserror::Error;

#[derive(Debug, Error)]
pub enum CexError {
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: String, available: String },

    #[error("Invalid order: {0}")]
    InvalidOrder(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Kafka error: {0}")]
    Kafka(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

