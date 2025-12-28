//! Error types for smart contracts

use alloc::string::String;
use core::fmt;

/// Contract execution errors
#[derive(Debug, Clone)]
pub enum ContractError {
    /// Storage read operation failed
    StorageReadFailed,

    /// Storage write operation failed
    StorageWriteFailed,

    /// Failed to serialize data
    SerializationFailed,

    /// Failed to deserialize data
    DeserializationFailed,

    /// Unauthorized access attempt
    Unauthorized,

    /// Insufficient balance for operation
    InsufficientBalance { required: u64, available: u64 },

    /// Invalid argument provided
    InvalidArgument(String),

    /// Contract call failed
    ContractCallFailed(String),

    /// Token transfer failed
    TransferFailed,

    /// Failed to read call data for the current invocation
    CallDataUnavailable,

    /// Failed to return data to the runtime
    ReturnDataWriteFailed,

    /// Signature verification failed
    InvalidSignature,

    /// Arithmetic overflow
    Overflow,

    /// Arithmetic underflow
    Underflow,

    /// Reentrancy attack detected
    ReentrancyDetected,

    /// Custom error with message
    Custom(String),
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractError::StorageReadFailed => write!(f, "Storage read failed"),
            ContractError::StorageWriteFailed => write!(f, "Storage write failed"),
            ContractError::SerializationFailed => write!(f, "Serialization failed"),
            ContractError::DeserializationFailed => write!(f, "Deserialization failed"),
            ContractError::Unauthorized => write!(f, "Unauthorized access"),
            ContractError::InsufficientBalance {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient balance: required {}, available {}",
                    required, available
                )
            }
            ContractError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            ContractError::ContractCallFailed(msg) => write!(f, "Contract call failed: {}", msg),
            ContractError::TransferFailed => write!(f, "Transfer failed"),
            ContractError::CallDataUnavailable => write!(f, "Call data unavailable"),
            ContractError::ReturnDataWriteFailed => write!(f, "Unable to write return data"),
            ContractError::InvalidSignature => write!(f, "Invalid signature"),
            ContractError::Overflow => write!(f, "Arithmetic overflow"),
            ContractError::Underflow => write!(f, "Arithmetic underflow"),
            ContractError::ReentrancyDetected => write!(f, "Reentrancy attack detected"),
            ContractError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

/// Result type for contract operations
pub type ContractResult<T> = Result<T, ContractError>;
