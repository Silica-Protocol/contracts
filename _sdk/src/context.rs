//! Execution context for smart contracts

use crate::error::{ContractError, ContractResult};
use crate::ffi;
use crate::security::validation;
use alloc::string::String;
use alloc::vec::Vec;
use serde::Serialize;

const MAX_BLOCK_HEIGHT: u64 = 1_000_000_000_000_000; // ~10^15 blocks
const MAX_BLOCK_TIMESTAMP: u64 = 10_000_000_000_000; // ~300 years of seconds

/// Execution context for the current transaction
#[derive(Clone, Debug)]
pub struct Context {
    sender: String,
    contract_address: String,
    block_height: u64,
    block_timestamp: u64,
    value: u64,
}

impl Context {
    /// Get the transaction sender address
    pub fn sender(&self) -> &str {
        &self.sender
    }

    /// Get the current contract address
    pub fn contract_address(&self) -> &str {
        &self.contract_address
    }

    /// Get the current block height
    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    /// Get the current block timestamp (Unix timestamp)
    pub fn block_timestamp(&self) -> u64 {
        self.block_timestamp
    }

    /// Get the amount of tokens sent with the transaction
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Read the call data payload provided with this invocation.
    pub fn call_data(&self) -> ContractResult<Vec<u8>> {
        ffi::read_call_data()
    }

    /// Write return data back to the host in postcard-encoded form.
    pub fn return_data<T: Serialize>(&self, value: &T) -> ContractResult<()> {
        let payload =
            postcard::to_allocvec(value).map_err(|_| ContractError::SerializationFailed)?;
        ffi::write_return_data(&payload)
    }

    /// Write raw return bytes to the host without additional serialization.
    pub fn return_bytes(&self, data: &[u8]) -> ContractResult<()> {
        ffi::write_return_data(data)
    }

    /// Transfer tokens from the current contract to a recipient.
    pub fn transfer_tokens(&self, recipient: &str, amount: u64) -> ContractResult<()> {
        validation::validate_address(recipient)?;
        validation::validate_positive_amount(amount)?;
        ffi::transfer_tokens(recipient, amount)
    }

    /// Ensure the attached value is at least the requested amount.
    pub fn require_min_value(&self, required: u64) -> ContractResult<()> {
        if self.value < required {
            return Err(ContractError::InsufficientBalance {
                required,
                available: self.value,
            });
        }
        Ok(())
    }
}

/// Get the current execution context, panicking if the host data is invalid.
pub fn context() -> Context {
    try_context().expect("execution context must be valid")
}

/// Attempt to fetch the current execution context with validation.
///
/// This fetches context data once per invocation and performs defensive validation
/// against obviously malformed host inputs.
pub fn try_context() -> ContractResult<Context> {
    let sender = ffi::get_sender_address();
    let contract_address = ffi::get_contract_addr();
    let block_height = ffi::get_block_height();
    let block_timestamp = ffi::get_block_timestamp();
    let value = ffi::get_value();

    validation::validate_address(&sender)?;
    validation::validate_address(&contract_address)?;
    ensure_block_parameters(block_height, block_timestamp)?;

    Ok(Context {
        sender,
        contract_address,
        block_height,
        block_timestamp,
        value,
    })
}

fn ensure_block_parameters(height: u64, timestamp: u64) -> ContractResult<()> {
    if height > MAX_BLOCK_HEIGHT {
        return Err(ContractError::InvalidArgument(
            "Block height exceeds maximum bound".into(),
        ));
    }
    if timestamp > MAX_BLOCK_TIMESTAMP {
        return Err(ContractError::InvalidArgument(
            "Block timestamp exceeds maximum bound".into(),
        ));
    }
    if timestamp == 0 {
        return Err(ContractError::InvalidArgument(
            "Block timestamp cannot be zero".into(),
        ));
    }
    if height == u64::MAX {
        return Err(ContractError::InvalidArgument(
            "Block height is invalid".into(),
        ));
    }
    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::ffi::mock;

    fn prepare_mock_env() {
        mock::reset();
        mock::set_sender("chert1sender000000000000000000");
        mock::set_contract_address("chert1contract0000000000000000");
        mock::set_block_height(42);
        mock::set_block_timestamp(1_700_000_000);
        mock::set_value(1_000);
    }

    #[test]
    fn context_initializes_from_mock_runtime() {
        prepare_mock_env();

        let ctx = try_context().expect("context should be available");

        assert_eq!(ctx.sender(), "chert1sender000000000000000000");
        assert_eq!(ctx.contract_address(), "chert1contract0000000000000000");
        assert_eq!(ctx.block_height(), 42);
        assert_eq!(ctx.block_timestamp(), 1_700_000_000);
        assert_eq!(ctx.value(), 1_000);
    }

    #[test]
    fn context_rejects_invalid_addresses() {
        prepare_mock_env();
        mock::set_sender("");

        let err = try_context().expect_err("empty sender must be rejected");
        assert!(matches!(err, ContractError::InvalidArgument(_)));
    }

    #[test]
    fn return_data_roundtrip() {
        prepare_mock_env();
        let ctx = try_context().expect("context should be available");

        ctx.return_data(&64u32).expect("return data must succeed");

        let bytes = mock::take_return_data();
        let decoded: u32 = postcard::from_bytes(&bytes).expect("postcard decode");
        assert_eq!(decoded, 64);
    }

    #[test]
    fn call_data_roundtrip() {
        prepare_mock_env();
        let payload = b"call-data";
        mock::set_call_data(payload);

        let ctx = try_context().expect("context should be available");
        let data = ctx.call_data().expect("call data retrieval");
        assert_eq!(data, payload);
    }

    #[test]
    fn require_min_value_enforces_bound() {
        prepare_mock_env();
        let ctx = try_context().expect("context should be available");

        assert!(ctx.require_min_value(500).is_ok());
        let err = ctx
            .require_min_value(2_000)
            .expect_err("insufficient balance expected");
        match err {
            ContractError::InsufficientBalance {
                required,
                available,
            } => {
                assert_eq!(required, 2_000);
                assert_eq!(available, 1_000);
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn transfer_tokens_validates_inputs() {
        prepare_mock_env();
        let ctx = try_context().expect("context should be available");

        assert!(
            ctx.transfer_tokens("chert1recipient000000000000", 500)
                .is_ok()
        );

        let address_err = ctx
            .transfer_tokens("", 10)
            .expect_err("empty recipient should fail");
        assert!(matches!(address_err, ContractError::InvalidArgument(_)));

        let amount_err = ctx
            .transfer_tokens("chert1recipient000000000000", 0)
            .expect_err("zero amount should fail");
        assert!(matches!(amount_err, ContractError::InvalidArgument(_)));
    }
}
