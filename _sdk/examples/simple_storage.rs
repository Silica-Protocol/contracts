//! Simple key-value storage contract
//!
//! Allows storing and retrieving string values

#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(target_arch = "wasm32", no_main)]

#[cfg(target_arch = "wasm32")]
extern crate alloc;

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

use silica_contract_sdk::event;
use silica_contract_sdk::prelude::*;

/// Store a key-value pair
#[unsafe(no_mangle)]
pub extern "C" fn store() {
    // In a real contract, we'd parse arguments from input
    // For now, this is a placeholder showing the pattern

    let ctx = match try_context() {
        Ok(ctx) => ctx,
        Err(_) => {
            log("Failed to acquire execution context");
            return;
        }
    };
    let mut map: Map<String, String> = Map::new("data");

    // Example: store a value
    let key = "example_key".to_string();
    let value = "example_value".to_string();

    if map.set(&key, &value).is_ok() {
        log(&format!("Stored: {} = {}", key, value));
        event!("Stored", key: key, value: value, by: ctx.sender());
    }
}

/// Retrieve a value by key
#[unsafe(no_mangle)]
pub extern "C" fn retrieve() {
    let map: Map<String, String> = Map::new("data");
    let key = "example_key".to_string();

    match map.get(&key) {
        Ok(Some(value)) => {
            log(&format!("Retrieved: {} = {}", key, value));
        }
        Ok(None) => {
            log(&format!("Key not found: {}", key));
        }
        Err(_) => {
            log("Error retrieving value");
        }
    }
}

/// Delete a key-value pair
#[unsafe(no_mangle)]
pub extern "C" fn delete() {
    let ctx = match try_context() {
        Ok(ctx) => ctx,
        Err(_) => {
            log("Failed to acquire execution context");
            return;
        }
    };
    let mut map: Map<String, String> = Map::new("data");
    let key = "example_key".to_string();

    if map.remove(&key).is_ok() {
        log(&format!("Deleted key: {}", key));
        event!("Deleted", key: key, by: ctx.sender());
    }
}
