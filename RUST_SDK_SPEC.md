# Silica Contract SDK - Rust Implementation Specification

## Overview

This document provides detailed specifications for implementing the `silica-contract-sdk` - the Rust SDK for writing smart contracts on Chert Coin.

## Crate Structure

```
contracts/sdk/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # Public API exports
│   ├── prelude.rs          # Common imports
│   ├── storage/
│   │   ├── mod.rs          # Storage abstractions
│   │   ├── map.rs          # Type-safe map storage
│   │   ├── vector.rs       # Type-safe vector storage
│   │   └── set.rs          # Type-safe set storage
│   ├── context.rs          # Execution context API
│   ├── crypto.rs           # Cryptographic utilities
│   ├── events.rs           # Event emission
│   ├── token.rs            # Token utilities
│   ├── error.rs            # Error types
│   ├── ffi.rs              # Host function bindings
│   └── macros/
│       ├── mod.rs
│       └── contract.rs     # Procedural macros
├── examples/
│   ├── simple_storage.rs
│   ├── token.rs
│   └── nft.rs
└── tests/
    ├── storage_tests.rs
    └── integration_tests.rs
```

## Cargo.toml

```toml
[package]
name = "silica-contract-sdk"
version = "0.1.0"
edition = "2024"
authors = ["Chert Team"]
license = "MIT"
description = "Rust SDK for Chert Coin smart contracts"
repository = "https://github.com/Dedme/chert"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
serde = { workspace = true }
postcard = { workspace = true }
blake3 = { workspace = true }
thiserror = { workspace = true }

# Procedural macros
chert-contract-macros = { path = "./macros" }

[dev-dependencies]
tokio = { workspace = true }

[profile.release]
opt-level = "z"         # Optimize for size
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit for better optimization
strip = true            # Strip symbols
panic = "abort"         # Smaller binary size
overflow-checks = true  # Keep overflow checks for security
```

## Core Components

### 1. FFI Layer (`ffi.rs`)

**Host Function Bindings:**

```rust
// src/ffi.rs
//! Foreign Function Interface for host functions
//! 
//! These functions are provided by the Chert runtime and allow
//! contracts to interact with the blockchain state.

#[link(wasm_import_module = "env")]
extern "C" {
    /// Read a value from contract storage
    /// 
    /// # Arguments
    /// * `account_ptr` - Pointer to account address string
    /// * `account_len` - Length of account address
    /// * `key_ptr` - Pointer to storage key string
    /// * `key_len` - Length of storage key
    /// * `value_ptr` - Pointer to write value to
    /// * `value_len_ptr` - Pointer to write value length to
    /// 
    /// # Returns
    /// * 0 on success, -1 on error
    pub fn state_read(
        account_ptr: i32,
        account_len: i32,
        key_ptr: i32,
        key_len: i32,
        value_ptr: i32,
        value_len_ptr: i32,
    ) -> i32;

    /// Write a value to contract storage
    /// 
    /// # Returns
    /// * 0 on success, -1 on error
    pub fn state_write(
        account_ptr: i32,
        account_len: i32,
        key_ptr: i32,
        key_len: i32,
        value_ptr: i32,
        value_len: i32,
    ) -> i32;

    /// Emit a log message (for debugging)
    pub fn log(msg_ptr: i32, msg_len: i32);

    /// Emit an event (for indexing)
    pub fn emit_event(topic_ptr: i32, topic_len: i32, data_ptr: i32, data_len: i32);

    /// Call another contract
    /// 
    /// # Returns
    /// * Length of return data, or -1 on error
    pub fn call_contract(
        address_ptr: i32,
        address_len: i32,
        method_ptr: i32,
        method_len: i32,
        args_ptr: i32,
        args_len: i32,
        result_ptr: i32,
        result_len_ptr: i32,
    ) -> i32;

    /// Transfer tokens from contract to address
    /// 
    /// # Returns
    /// * 0 on success, -1 on error
    pub fn transfer(to_ptr: i32, to_len: i32, amount: u64) -> i32;

    /// Hash data with BLAKE3
    pub fn hash_blake3(data_ptr: i32, data_len: i32, output_ptr: i32);

    /// Verify an Ed25519 signature
    /// 
    /// # Returns
    /// * 1 if valid, 0 if invalid, -1 on error
    pub fn verify_signature(
        pubkey_ptr: i32,
        message_ptr: i32,
        message_len: i32,
        signature_ptr: i32,
    ) -> i32;

    /// Get current block height
    pub fn get_block_height() -> u64;

    /// Get current block timestamp (Unix timestamp)
    pub fn get_block_timestamp() -> u64;

    /// Get transaction sender address
    /// 
    /// # Returns
    /// * Length of address written to buffer
    pub fn get_sender(buffer_ptr: i32) -> i32;

    /// Get current contract address
    /// 
    /// # Returns
    /// * Length of address written to buffer
    pub fn get_contract_address(buffer_ptr: i32) -> i32;

    /// Get amount of tokens sent with transaction
    pub fn get_value() -> u64;
}

/// Safe wrapper for reading storage
pub(crate) fn read_storage(account: &str, key: &str) -> Result<Vec<u8>, ContractError> {
    let mut value = vec![0u8; 65536]; // Max 64KB value
    let mut value_len: i32 = 0;

    let result = unsafe {
        state_read(
            account.as_ptr() as i32,
            account.len() as i32,
            key.as_ptr() as i32,
            key.len() as i32,
            value.as_mut_ptr() as i32,
            &mut value_len as *mut i32 as i32,
        )
    };

    if result == 0 {
        value.truncate(value_len as usize);
        Ok(value)
    } else {
        Err(ContractError::StorageReadFailed)
    }
}

/// Safe wrapper for writing storage
pub(crate) fn write_storage(account: &str, key: &str, value: &[u8]) -> Result<(), ContractError> {
    let result = unsafe {
        state_write(
            account.as_ptr() as i32,
            account.len() as i32,
            key.as_ptr() as i32,
            key.len() as i32,
            value.as_ptr() as i32,
            value.len() as i32,
        )
    };

    if result == 0 {
        Ok(())
    } else {
        Err(ContractError::StorageWriteFailed)
    }
}
```

### 2. Storage Abstraction (`storage/mod.rs`)

```rust
// src/storage/mod.rs
use crate::context::context;
use crate::error::ContractError;
use crate::ffi;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Low-level storage access
pub struct Storage;

impl Storage {
    /// Get a value from storage
    pub fn get<T>(&self, key: &str) -> Result<Option<T>, ContractError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let contract_address = context().contract_address();
        
        match ffi::read_storage(&contract_address, key) {
            Ok(data) if data.is_empty() => Ok(None),
            Ok(data) => {
                let value = postcard::from_bytes(&data)
                    .map_err(|_| ContractError::DeserializationFailed)?;
                Ok(Some(value))
            }
            Err(ContractError::StorageReadFailed) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Set a value in storage
    pub fn set<T>(&mut self, key: &str, value: &T) -> Result<(), ContractError>
    where
        T: Serialize,
    {
        let contract_address = context().contract_address();
        let data = postcard::to_allocvec(value)
            .map_err(|_| ContractError::SerializationFailed)?;
        
        ffi::write_storage(&contract_address, key, &data)
    }

    /// Remove a value from storage
    pub fn remove(&mut self, key: &str) -> Result<(), ContractError> {
        let contract_address = context().contract_address();
        ffi::write_storage(&contract_address, key, &[])
    }

    /// Check if a key exists
    pub fn has(&self, key: &str) -> bool {
        let contract_address = context().contract_address();
        match ffi::read_storage(&contract_address, key) {
            Ok(data) => !data.is_empty(),
            Err(_) => false,
        }
    }
}

/// Global storage instance
pub fn storage() -> Storage {
    Storage
}
```

### 3. Type-Safe Storage Structures (`storage/map.rs`)

```rust
// src/storage/map.rs
use crate::error::ContractError;
use crate::storage::Storage;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Type-safe key-value map in storage
pub struct Map<K, V> {
    prefix: String,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> Map<K, V>
where
    K: Serialize,
    V: Serialize + for<'de> Deserialize<'de>,
{
    /// Create a new map with a unique prefix
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            _phantom: PhantomData,
        }
    }

    /// Generate storage key for a map entry
    fn storage_key(&self, key: &K) -> Result<String, ContractError> {
        let key_bytes = postcard::to_allocvec(key)
            .map_err(|_| ContractError::SerializationFailed)?;
        let key_hash = blake3::hash(&key_bytes);
        Ok(format!("{}:{}", self.prefix, hex::encode(key_hash.as_bytes())))
    }

    /// Get a value from the map
    pub fn get(&self, key: &K) -> Result<Option<V>, ContractError> {
        let storage_key = self.storage_key(key)?;
        Storage.get(&storage_key)
    }

    /// Set a value in the map
    pub fn set(&mut self, key: &K, value: &V) -> Result<(), ContractError> {
        let storage_key = self.storage_key(key)?;
        Storage.set(&storage_key, value)
    }

    /// Remove a value from the map
    pub fn remove(&mut self, key: &K) -> Result<(), ContractError> {
        let storage_key = self.storage_key(key)?;
        Storage.remove(&storage_key)
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &K) -> Result<bool, ContractError> {
        let storage_key = self.storage_key(key)?;
        Ok(Storage.has(&storage_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_storage_key_generation() {
        let map: Map<String, u64> = Map::new("balances");
        let key = "alice".to_string();
        let storage_key = map.storage_key(&key).unwrap();
        
        // Should be prefixed and hashed
        assert!(storage_key.starts_with("balances:"));
        assert!(storage_key.len() > "balances:".len());
    }
}
```

### 4. Context API (`context.rs`)

```rust
// src/context.rs
use crate::ffi;

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
}

/// Get the current execution context
/// 
/// This is cached for the duration of the contract call
pub fn context() -> Context {
    thread_local! {
        static CONTEXT: std::cell::RefCell<Option<Context>> = std::cell::RefCell::new(None);
    }

    CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        if ctx.is_none() {
            // Fetch context from host
            let mut sender_buf = vec![0u8; 64];
            let mut address_buf = vec![0u8; 64];

            unsafe {
                let sender_len = ffi::get_sender(sender_buf.as_mut_ptr() as i32);
                sender_buf.truncate(sender_len as usize);

                let address_len = ffi::get_contract_address(address_buf.as_mut_ptr() as i32);
                address_buf.truncate(address_len as usize);

                *ctx = Some(Context {
                    sender: String::from_utf8_unchecked(sender_buf),
                    contract_address: String::from_utf8_unchecked(address_buf),
                    block_height: ffi::get_block_height(),
                    block_timestamp: ffi::get_block_timestamp(),
                    value: ffi::get_value(),
                });
            }
        }

        ctx.clone().unwrap()
    })
}
```

### 5. Error Types (`error.rs`)

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("Storage read failed")]
    StorageReadFailed,

    #[error("Storage write failed")]
    StorageWriteFailed,

    #[error("Serialization failed")]
    SerializationFailed,

    #[error("Deserialization failed")]
    DeserializationFailed,

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Contract call failed: {0}")]
    ContractCallFailed(String),

    #[error("Transfer failed")]
    TransferFailed,

    #[error("{0}")]
    Custom(String),
}

pub type ContractResult<T> = Result<T, ContractError>;
```

### 6. Events System (`events.rs`)

```rust
// src/events.rs
use crate::ffi;
use serde::Serialize;

/// Emit an event that can be indexed by off-chain services
pub fn emit<T: Serialize>(topic: &str, data: &T) {
    if let Ok(data_bytes) = postcard::to_allocvec(data) {
        unsafe {
            ffi::emit_event(
                topic.as_ptr() as i32,
                topic.len() as i32,
                data_bytes.as_ptr() as i32,
                data_bytes.len() as i32,
            );
        }
    }
}

/// Log a debug message (only visible in development)
pub fn log(message: &str) {
    unsafe {
        ffi::log(message.as_ptr() as i32, message.len() as i32);
    }
}

#[macro_export]
macro_rules! event {
    ($topic:expr, $($field:ident: $value:expr),* $(,)?) => {
        {
            #[derive(serde::Serialize)]
            struct Event {
                $($field: _),*
            }
            let event = Event {
                $($field: $value),*
            };
            $crate::events::emit($topic, &event);
        }
    };
}
```

### 7. Contract Macro (`macros/contract.rs`)

```rust
// macros/src/lib.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Mark a function as a contract entry point
/// 
/// # Example
/// ```ignore
/// #[contract_entrypoint]
/// pub fn transfer(to: String, amount: u64) -> Result<(), ContractError> {
///     // Contract logic
/// }
/// ```
#[proc_macro_attribute]
pub fn contract_entrypoint(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;

    let expanded = quote! {
        #[no_mangle]
        pub extern "C" fn #fn_name() {
            // Entry point wrapper that handles serialization/deserialization
            fn inner(#fn_inputs) #fn_output {
                #fn_block
            }

            // Call the inner function
            match inner() {
                Ok(_) => {},
                Err(e) => {
                    silica_contract_sdk::events::log(&format!("Contract error: {}", e));
                    std::process::abort();
                }
            }
        }
    };

    TokenStream::from(expanded)
}
```

## Example Contract

```rust
// examples/simple_token.rs
use silica_contract_sdk::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct TokenState {
    total_supply: u64,
    balances: Map<String, u64>,
}

#[contract_entrypoint]
pub fn initialize(total_supply: u64) -> ContractResult<()> {
    let mut state = TokenState {
        total_supply,
        balances: Map::new("balances"),
    };

    // Mint all tokens to deployer
    let sender = context().sender();
    state.balances.set(sender, &total_supply)?;

    // Emit event
    event!("Transfer", from: "0x0", to: sender, amount: total_supply);

    Ok(())
}

#[contract_entrypoint]
pub fn transfer(to: String, amount: u64) -> ContractResult<()> {
    let sender = context().sender();
    let mut balances: Map<String, u64> = Map::new("balances");

    // Check balance
    let sender_balance = balances.get(&sender)?.unwrap_or(0);
    if sender_balance < amount {
        return Err(ContractError::InsufficientBalance {
            required: amount,
            available: sender_balance,
        });
    }

    // Update balances
    balances.set(&sender, &(sender_balance - amount))?;
    
    let recipient_balance = balances.get(&to)?.unwrap_or(0);
    balances.set(&to, &(recipient_balance + amount))?;

    // Emit event
    event!("Transfer", from: sender, to: to, amount: amount);

    Ok(())
}

#[contract_entrypoint]
pub fn balance_of(account: String) -> ContractResult<u64> {
    let balances: Map<String, u64> = Map::new("balances");
    Ok(balances.get(&account)?.unwrap_or(0))
}
```

## Build Configuration

**Build script for optimal WASM:**

```bash
#!/bin/bash
# build-contract.sh

cargo build --target wasm32-unknown-unknown --release

# Optimize WASM size
wasm-opt -Oz --enable-bulk-memory \
    target/wasm32-unknown-unknown/release/contract.wasm \
    -o contract_optimized.wasm

# Show size
ls -lh contract_optimized.wasm
```

## Testing Strategy

```rust
// tests/integration_tests.rs
use silica_contract_sdk::*;

#[cfg(test)]
mod tests {
    use super::*;

    // Mock host functions for testing
    #[test]
    fn test_token_transfer() {
        // Initialize mock environment
        let mut env = MockEnv::new();
        env.set_sender("alice");
        env.set_contract_address("token_contract");

        // Deploy contract
        initialize(1000000).unwrap();

        // Transfer tokens
        transfer("bob".to_string(), 100).unwrap();

        // Verify balances
        assert_eq!(balance_of("alice".to_string()).unwrap(), 999900);
        assert_eq!(balance_of("bob".to_string()).unwrap(), 100);
    }
}
```

## Next Steps

1. Implement the core SDK components (FFI, storage, context)
2. Create type-safe storage structures (Map, Vector, Set)
3. Build procedural macros for contract generation
4. Add comprehensive tests with mock environment
5. Document the API with rustdoc
6. Create example contracts
7. Integrate with the Chert runtime (`silica/src/wasm.rs`)

---

**For questions or contributions, see the main [SMART_CONTRACTS_ROADMAP.md](../SMART_CONTRACTS_ROADMAP.md)**
