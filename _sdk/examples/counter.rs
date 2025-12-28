//! Simple counter contract example
//!
//! Demonstrates basic storage operations and state management

#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(target_arch = "wasm32", no_main)]

#[cfg(target_arch = "wasm32")]
extern crate alloc;

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

use silica_contract_sdk::event;
use silica_contract_sdk::prelude::*;

/// Initialize the counter with a starting value
#[unsafe(no_mangle)]
pub extern "C" fn initialize() {
    let mut storage = storage();

    // Set initial counter value to 0
    if storage.set("counter", &0u64).is_ok() {
        log("Counter initialized to 0");
        event!("Initialized", value: 0u64);
    }
}

/// Increment the counter by 1
#[unsafe(no_mangle)]
pub extern "C" fn increment() {
    let mut storage = storage();

    // Get current value
    let current: u64 = storage.get("counter").ok().flatten().unwrap_or(0);

    // Increment
    let new_value = current + 1;

    if storage.set("counter", &new_value).is_ok() {
        log(&format!("Counter incremented to {}", new_value));
        event!("Incremented", old_value: current, new_value: new_value);
    }
}

/// Get the current counter value
#[unsafe(no_mangle)]
pub extern "C" fn get_count() -> u64 {
    let storage = storage();

    storage.get("counter").ok().flatten().unwrap_or(0)
}

/// Reset counter to zero (only owner can do this)
#[unsafe(no_mangle)]
pub extern "C" fn reset() {
    let mut storage = storage();
    let ctx = match try_context() {
        Ok(ctx) => ctx,
        Err(_) => {
            log("Failed to acquire execution context");
            return;
        }
    };

    // In a real contract, check if sender is owner
    log(&format!("Counter reset by {}", ctx.sender()));

    if storage.set("counter", &0u64).is_ok() {
        event!("Reset", by: ctx.sender());
    }
}
