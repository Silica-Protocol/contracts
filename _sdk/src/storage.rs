//! Storage abstractions for smart contracts (memory pool optimized)

use crate::context::try_context;
use crate::error::{ContractError, ContractResult};
use crate::ffi;
use alloc::string::{String, ToString};
use core::marker::PhantomData;
use core::str;
use itoa::Buffer;
use serde::{Deserialize, Serialize};

/// Low-level storage access
pub struct Storage;

impl Storage {
    /// Get a value from storage (optimized - single context call)
    pub fn get<T>(&self, key: &str) -> ContractResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let ctx = try_context()?;
        match ffi::read_storage(ctx.contract_address(), key) {
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

    /// Set a value in storage (optimized - single context call)
    pub fn set<T>(&mut self, key: &str, value: &T) -> ContractResult<()>
    where
        T: Serialize,
    {
        let ctx = try_context()?;
        let data = postcard::to_allocvec(value).map_err(|_| ContractError::SerializationFailed)?;

        ffi::write_storage(ctx.contract_address(), key, &data)
    }

    /// Remove a value from storage (optimized - single context call)
    pub fn remove(&mut self, key: &str) -> ContractResult<()> {
        let ctx = try_context()?;
        ffi::write_storage(ctx.contract_address(), key, &[])
    }

    /// Check if a key exists (optimized - single context call)
    pub fn has(&self, key: &str) -> bool {
        let ctx = match try_context() {
            Ok(ctx) => ctx,
            Err(_) => return false,
        };
        match ffi::read_storage(ctx.contract_address(), key) {
            Ok(data) => !data.is_empty(),
            Err(_) => false,
        }
    }
}

/// Global storage instance
pub fn storage() -> Storage {
    Storage
}

/// Type-safe key-value map in storage (optimized for minimal allocations)
#[derive(Clone)]
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

    /// Generate storage key for a map entry (memory pool optimized)
    fn storage_key(&self, key: &K) -> ContractResult<String> {
        let key_bytes =
            postcard::to_allocvec(key).map_err(|_| ContractError::SerializationFailed)?;
        let key_hash = blake3::hash(&key_bytes);

        let mut storage_key = String::with_capacity(self.prefix.len() + 1 + 64);
        storage_key.push_str(&self.prefix);
        storage_key.push(':');

        let mut hex_buf = [0u8; 64];
        hex::encode_to_slice(key_hash.as_bytes(), &mut hex_buf)
            .map_err(|_| ContractError::SerializationFailed)?;
        let hex_str = str::from_utf8(&hex_buf).map_err(|_| ContractError::SerializationFailed)?;
        storage_key.push_str(hex_str);

        Ok(storage_key)
    }

    /// Get a value from the map (memory pool optimized)
    pub fn get(&self, key: &K) -> ContractResult<Option<V>> {
        let storage_key = self.storage_key(key)?;
        storage().get(&storage_key)
    }

    /// Set a value in the map (memory pool optimized)
    pub fn set(&mut self, key: &K, value: &V) -> ContractResult<()> {
        let storage_key = self.storage_key(key)?;
        storage().set(&storage_key, value)
    }

    /// Remove a value from the map (memory pool optimized)
    pub fn remove(&mut self, key: &K) -> ContractResult<()> {
        let storage_key = self.storage_key(key)?;
        storage().remove(&storage_key)
    }

    /// Check if a key exists (memory pool optimized)
    pub fn contains_key(&self, key: &K) -> ContractResult<bool> {
        let storage_key = self.storage_key(key)?;
        Ok(storage().has(&storage_key))
    }
}

/// Type-safe vector in storage (optimized for sequential access)
#[derive(Clone)]
pub struct Vector<T> {
    prefix: String,
    _phantom: PhantomData<T>,
}

impl<T> Vector<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Create a new vector with a unique prefix
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            _phantom: PhantomData,
        }
    }

    /// Get the length of the vector (cached for performance)
    pub fn len(&self) -> ContractResult<u64> {
        let len_key = self.len_key();
        Ok(storage().get::<u64>(&len_key)?.unwrap_or(0))
    }

    /// Check if vector is empty (optimized inline)
    pub fn is_empty(&self) -> ContractResult<bool> {
        let len_key = self.len_key();
        Ok(storage().get::<u64>(&len_key)?.unwrap_or(0) == 0)
    }

    /// Get an element at index (bounds checked)
    pub fn get(&self, index: u64) -> ContractResult<Option<T>> {
        let len = self.len()?;
        if index >= len {
            return Ok(None);
        }

        let item_key = self.item_key(index);
        storage().get(&item_key)
    }

    /// Set an element at index (bounds checked)
    pub fn set(&mut self, index: u64, value: &T) -> ContractResult<()> {
        let len = self.len()?;
        if index >= len {
            return Err(ContractError::InvalidArgument(
                "Index out of bounds".to_string(),
            ));
        }

        let item_key = self.item_key(index);
        storage().set(&item_key, value)
    }

    /// Push an element to the end (optimized - single length read)
    pub fn push(&mut self, value: &T) -> ContractResult<()> {
        let len = self.len()?;
        let item_key = self.item_key(len);
        storage().set(&item_key, value)?;

        let len_key = self.len_key();
        storage().set(&len_key, &(len + 1))
    }

    /// Pop an element from the end (optimized - single length read)
    pub fn pop(&mut self) -> ContractResult<Option<T>> {
        let len = self.len()?;
        if len == 0 {
            return Ok(None);
        }

        let item_key = self.item_key(len - 1);
        let value = storage().get(&item_key)?;
        storage().remove(&item_key)?;

        let len_key = self.len_key();
        storage().set(&len_key, &(len - 1))?;

        Ok(value)
    }

    fn len_key(&self) -> String {
        let mut key = String::with_capacity(self.prefix.len() + 5);
        key.push_str(&self.prefix);
        key.push_str("::len");
        key
    }

    fn item_key(&self, index: u64) -> String {
        let mut buffer = Buffer::new();
        let index_str = buffer.format(index);

        let mut key = String::with_capacity(self.prefix.len() + 7 + index_str.len());
        key.push_str(&self.prefix);
        key.push_str("::item::");
        key.push_str(index_str);
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn test_map_storage_key_generation() {
        let map: Map<String, u64> = Map::new("balances");
        let key = "alice".to_string();
        let storage_key = map.storage_key(&key).unwrap();

        // Should be prefixed and hashed
        assert!(storage_key.starts_with("balances:"));
        assert!(storage_key.len() > "balances:".len());
    }
}
