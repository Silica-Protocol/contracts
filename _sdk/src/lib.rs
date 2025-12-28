//! # Silica Contract SDK
//!
//! Rust SDK for writing smart contracts on the Silica blockchain.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use silica_contract_sdk::prelude::*;
//!
//! pub extern "C" fn transfer() {
//!     let ctx = try_context().expect("context must be available");
//!     let sender = ctx.sender();
//!     
//!     // Your contract logic here
//! }
//! ```

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod context;
pub mod crypto;
pub mod error;
pub mod events;
pub mod ffi;
pub mod security;
pub mod storage;

/// Common imports for contract development
pub mod prelude {
    pub use crate::context::{Context, context, try_context};
    pub use crate::crypto;
    pub use crate::error::{ContractError, ContractResult};
    pub use crate::events::{emit, log};
    pub use crate::security::safe_math;
    pub use crate::security::validation;
    pub use crate::security::{AccessControl, ReentrancyGuard};
    pub use crate::storage::{Map, Storage, storage};

    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use serde::{Deserialize, Serialize};
}
