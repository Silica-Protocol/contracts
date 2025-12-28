//! Foreign Function Interface for host functions
//!
//! These functions are provided by the Chert runtime and allow
//! contracts to interact with the blockchain state. When running
//! tests on the native host, we provide a deterministic mock runtime
//! so contracts can be exercised without a full blockchain node.

use crate::error::{ContractError, ContractResult};
use alloc::string::String;
#[cfg(target_arch = "wasm32")]
use alloc::vec;
use alloc::vec::Vec;

#[cfg(target_arch = "wasm32")]
mod host {
    use super::{ContractError, ContractResult, String, Vec, vec};

    // Host function imports from the runtime
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn state_read(
            account_ptr: i32,
            account_len: i32,
            key_ptr: i32,
            key_len: i32,
            value_ptr: i32,
            value_len_ptr: i32,
        ) -> i32;

        pub fn state_write(
            account_ptr: i32,
            account_len: i32,
            key_ptr: i32,
            key_len: i32,
            value_ptr: i32,
            value_len: i32,
        ) -> i32;

        pub fn log(msg_ptr: i32, msg_len: i32);

        pub fn emit_event(topic_ptr: i32, topic_len: i32, data_ptr: i32, data_len: i32);

        pub fn transfer(to_ptr: i32, to_len: i32, amount: u64) -> i32;

        pub fn get_block_height() -> u64;
        pub fn get_block_timestamp() -> u64;
        pub fn get_sender(buffer_ptr: i32) -> i32;
        pub fn get_contract_address(buffer_ptr: i32) -> i32;
        pub fn get_value() -> u64;

        pub fn get_call_data_length() -> i32;
        pub fn read_call_data(buffer_ptr: i32, buffer_len: i32) -> i32;
        pub fn write_return_data(buffer_ptr: i32, buffer_len: i32) -> i32;

    }

    pub fn read_storage(account: &str, key: &str) -> ContractResult<Vec<u8>> {
        const MAX_VALUE_SIZE: usize = 65_536;
        let mut value = vec![0_u8; MAX_VALUE_SIZE];
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

    pub fn write_storage(account: &str, key: &str, value: &[u8]) -> ContractResult<()> {
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

    pub fn log_message(message: &str) {
        unsafe {
            log(message.as_ptr() as i32, message.len() as i32);
        }
    }

    pub fn emit_event_internal(topic: &str, data: &[u8]) {
        unsafe {
            emit_event(
                topic.as_ptr() as i32,
                topic.len() as i32,
                data.as_ptr() as i32,
                data.len() as i32,
            );
        }
    }

    pub fn transfer_tokens(to: &str, amount: u64) -> ContractResult<()> {
        let result = unsafe { transfer(to.as_ptr() as i32, to.len() as i32, amount) };
        if result == 0 {
            Ok(())
        } else {
            Err(ContractError::TransferFailed)
        }
    }

    pub fn block_height() -> u64 {
        unsafe { get_block_height() }
    }

    pub fn block_timestamp() -> u64 {
        unsafe { get_block_timestamp() }
    }

    pub fn sender() -> String {
        let mut buffer = vec![0_u8; 128];
        let len = unsafe { get_sender(buffer.as_mut_ptr() as i32) };
        buffer.truncate(len as usize);
        String::from_utf8_lossy(&buffer).into_owned()
    }

    pub fn contract_address() -> String {
        let mut buffer = vec![0_u8; 128];
        let len = unsafe { get_contract_address(buffer.as_mut_ptr() as i32) };
        buffer.truncate(len as usize);
        String::from_utf8_lossy(&buffer).into_owned()
    }

    pub fn value() -> u64 {
        unsafe { get_value() }
    }

    pub fn read_call_data_internal() -> ContractResult<Vec<u8>> {
        let len = unsafe { get_call_data_length() };
        if len < 0 {
            return Err(ContractError::CallDataUnavailable);
        }
        if len == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = vec![0_u8; len as usize];
        let result = unsafe { read_call_data(buffer.as_mut_ptr() as i32, len) };
        if result == 0 {
            Ok(buffer)
        } else {
            Err(ContractError::CallDataUnavailable)
        }
    }

    pub fn write_return_data_internal(data: &[u8]) -> ContractResult<()> {
        let result = unsafe { write_return_data(data.as_ptr() as i32, data.len() as i32) };
        if result == 0 {
            Ok(())
        } else {
            Err(ContractError::ReturnDataWriteFailed)
        }
    }

    fn hash_blake3_bytes(data: &[u8]) -> [u8; 32] {
        let digest = blake3::hash(data);
        *digest.as_bytes()
    }

    fn verify_signature_slice(
        pubkey: &[u8; 32],
        message: &[u8],
        signature: &[u8; 64],
    ) -> ContractResult<bool> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let verifying_key = VerifyingKey::from_bytes(pubkey)
            .map_err(|_| ContractError::InvalidSignature)?;
        let signature = Signature::from_bytes(signature);

        match verifying_key.verify(message, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn hash_blake3_internal(data_ptr: i32, data_len: i32, output_ptr: i32) {
        unsafe {
            let data = core::slice::from_raw_parts(data_ptr as *const u8, data_len as usize);
            let digest = hash_blake3_bytes(data);
            let output_slice = core::slice::from_raw_parts_mut(output_ptr as *mut u8, 32);
            output_slice.copy_from_slice(&digest);
        }
    }

    pub fn verify_signature_internal(
        pubkey_ptr: i32,
        message_ptr: i32,
        message_len: i32,
        signature_ptr: i32,
    ) -> i32 {
        unsafe {
            let pubkey_slice = core::slice::from_raw_parts(pubkey_ptr as *const u8, 32);
            let message =
                core::slice::from_raw_parts(message_ptr as *const u8, message_len as usize);
            let signature_slice = core::slice::from_raw_parts(signature_ptr as *const u8, 64);

            let mut pubkey = [0_u8; 32];
            pubkey.copy_from_slice(pubkey_slice);
            let mut signature = [0_u8; 64];
            signature.copy_from_slice(signature_slice);

            match verify_signature_slice(&pubkey, message, &signature) {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(_) => -1,
            }
        }
    }

    pub fn batch_hash_blake3(inputs: &[&[u8]]) -> ContractResult<Vec<[u8; 32]>> {
        let mut outputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            outputs.push(hash_blake3_bytes(input));
        }
        Ok(outputs)
    }

    pub fn batch_verify_signatures(
        pubkeys: &[&[u8; 32]],
        messages: &[&[u8]],
        signatures: &[&[u8; 64]],
    ) -> ContractResult<Vec<bool>> {
        if pubkeys.len() != messages.len() || pubkeys.len() != signatures.len() {
            return Err(ContractError::InvalidArgument(String::from(
                "Batch input length mismatch",
            )));
        }

        let mut results = Vec::with_capacity(pubkeys.len());
        for idx in 0..pubkeys.len() {
            results.push(verify_signature_slice(
                pubkeys[idx],
                messages[idx],
                signatures[idx],
            )?);
        }
        Ok(results)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod host {
    use super::{ContractError, ContractResult, String, Vec};
    use alloc::string::ToString;
    use spin::Mutex;

    #[derive(Clone, Debug)]
    pub struct EventRecord {
        pub topic: String,
        pub data: Vec<u8>,
    }

    #[derive(Default)]
    pub struct MockRuntime {
        storage: alloc::collections::BTreeMap<(String, String), Vec<u8>>,
        sender: String,
        contract_address: String,
        block_height: u64,
        block_timestamp: u64,
        value: u64,
        call_data: Vec<u8>,
        return_data: Vec<u8>,
        events: Vec<EventRecord>,
        logs: Vec<String>,
    }

    impl MockRuntime {
        fn reset(&mut self) {
            self.storage.clear();
            self.events.clear();
            self.logs.clear();
            self.call_data.clear();
            self.return_data.clear();
            self.block_height = 0;
            self.block_timestamp = 0;
            self.value = 0;
        }

        fn storage_key(account: &str, key: &str) -> (String, String) {
            (account.to_string(), key.to_string())
        }

        fn read_storage(&self, account: &str, key: &str) -> ContractResult<Vec<u8>> {
            let lookup = Self::storage_key(account, key);
            Ok(self.storage.get(&lookup).cloned().unwrap_or_else(Vec::new))
        }

        fn write_storage(&mut self, account: &str, key: &str, value: &[u8]) -> ContractResult<()> {
            let lookup = Self::storage_key(account, key);
            if value.is_empty() {
                self.storage.remove(&lookup);
            } else {
                self.storage.insert(lookup, value.to_vec());
            }
            Ok(())
        }

        fn log(&mut self, message: &str) {
            self.logs.push(message.to_string());
        }

        fn emit_event_internal(&mut self, topic: &str, data: &[u8]) {
            self.events.push(EventRecord {
                topic: topic.to_string(),
                data: data.to_vec(),
            });
        }
    }

    static MOCK_RUNTIME: Mutex<Option<MockRuntime>> = Mutex::new(None);

    pub fn with_runtime<F, R>(f: F) -> R
    where
        F: FnOnce(&mut MockRuntime) -> R,
    {
        let mut guard = MOCK_RUNTIME.lock();
        if guard.is_none() {
            *guard = Some(MockRuntime::default());
        }

        // SAFETY: guard is initialized above and held until the closure completes.
        let runtime = guard.as_mut().expect("mock runtime initialized");
        f(runtime)
    }

    pub fn read_storage(account: &str, key: &str) -> ContractResult<Vec<u8>> {
        with_runtime(|rt| rt.read_storage(account, key))
    }

    pub fn write_storage(account: &str, key: &str, value: &[u8]) -> ContractResult<()> {
        with_runtime(|rt| rt.write_storage(account, key, value))
    }

    pub fn log_message(message: &str) {
        with_runtime(|rt| rt.log(message));
    }

    pub fn emit_event_internal(topic: &str, data: &[u8]) {
        with_runtime(|rt| rt.emit_event_internal(topic, data));
    }

    pub fn transfer_tokens(_to: &str, _amount: u64) -> ContractResult<()> {
        // Value transfers are no-ops in the mock runtime.
        Ok(())
    }

    pub fn block_height() -> u64 {
        with_runtime(|rt| rt.block_height)
    }

    pub fn block_timestamp() -> u64 {
        with_runtime(|rt| rt.block_timestamp)
    }

    pub fn sender() -> String {
        with_runtime(|rt| rt.sender.clone())
    }

    pub fn contract_address() -> String {
        with_runtime(|rt| rt.contract_address.clone())
    }

    pub fn value() -> u64 {
        with_runtime(|rt| rt.value)
    }

    pub fn read_call_data_internal() -> ContractResult<Vec<u8>> {
        with_runtime(|rt| Ok(rt.call_data.clone()))
    }

    pub fn write_return_data_internal(data: &[u8]) -> ContractResult<()> {
        with_runtime(|rt| {
            rt.return_data = data.to_vec();
            Ok(())
        })
    }

    fn hash_blake3_bytes(data: &[u8]) -> [u8; 32] {
        let digest = blake3::hash(data);
        *digest.as_bytes()
    }

    fn verify_signature_slice(
        pubkey: &[u8; 32],
        message: &[u8],
        signature: &[u8; 64],
    ) -> ContractResult<bool> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let verifying_key = VerifyingKey::from_bytes(pubkey)
            .map_err(|_| ContractError::InvalidSignature)?;
        let signature = Signature::from_bytes(signature);

        match verifying_key.verify(message, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn hash_blake3_internal(data_ptr: i32, data_len: i32, output_ptr: i32) {
        unsafe {
            let data = core::slice::from_raw_parts(data_ptr as *const u8, data_len as usize);
            let digest = hash_blake3_bytes(data);
            let output_slice = core::slice::from_raw_parts_mut(output_ptr as *mut u8, 32);
            output_slice.copy_from_slice(&digest);
        }
    }

    pub fn verify_signature_internal(
        pubkey_ptr: i32,
        message_ptr: i32,
        message_len: i32,
        signature_ptr: i32,
    ) -> i32 {
        unsafe {
            let pubkey_slice = core::slice::from_raw_parts(pubkey_ptr as *const u8, 32);
            let message =
                core::slice::from_raw_parts(message_ptr as *const u8, message_len as usize);
            let signature_slice = core::slice::from_raw_parts(signature_ptr as *const u8, 64);

            let mut pubkey = [0_u8; 32];
            pubkey.copy_from_slice(pubkey_slice);
            let mut signature = [0_u8; 64];
            signature.copy_from_slice(signature_slice);

            match verify_signature_slice(&pubkey, message, &signature) {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(_) => -1,
            }
        }
    }

    pub fn batch_hash_blake3(inputs: &[&[u8]]) -> ContractResult<Vec<[u8; 32]>> {
        let mut outputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            outputs.push(hash_blake3_bytes(input));
        }
        Ok(outputs)
    }

    pub fn batch_verify_signatures(
        pubkeys: &[&[u8; 32]],
        messages: &[&[u8]],
        signatures: &[&[u8; 64]],
    ) -> ContractResult<Vec<bool>> {
        if pubkeys.len() != messages.len() || pubkeys.len() != signatures.len() {
            return Err(ContractError::InvalidArgument(String::from(
                "Batch input length mismatch",
            )));
        }

        let mut results = Vec::with_capacity(pubkeys.len());
        for idx in 0..pubkeys.len() {
            results.push(verify_signature_slice(
                pubkeys[idx],
                messages[idx],
                signatures[idx],
            )?);
        }
        Ok(results)
    }

    pub fn reset() {
        with_runtime(|rt| rt.reset());
    }

    pub fn set_sender(sender: &str) {
        with_runtime(|rt| rt.sender = sender.to_string());
    }

    pub fn set_contract_address(addr: &str) {
        with_runtime(|rt| rt.contract_address = addr.to_string());
    }

    pub fn set_block_height(height: u64) {
        with_runtime(|rt| rt.block_height = height);
    }

    pub fn set_block_timestamp(timestamp: u64) {
        with_runtime(|rt| rt.block_timestamp = timestamp);
    }

    pub fn set_value(amount: u64) {
        with_runtime(|rt| rt.value = amount);
    }

    pub fn set_call_data(data: &[u8]) {
        with_runtime(|rt| rt.call_data = data.to_vec());
    }

    pub fn take_events() -> Vec<EventRecord> {
        with_runtime(|rt| {
            let mut drained = Vec::new();
            core::mem::swap(&mut drained, &mut rt.events);
            drained
        })
    }

    pub fn take_logs() -> Vec<String> {
        with_runtime(|rt| {
            let mut drained = Vec::new();
            core::mem::swap(&mut drained, &mut rt.logs);
            drained
        })
    }

    pub fn take_return_data() -> Vec<u8> {
        with_runtime(|rt| {
            let mut drained = Vec::new();
            core::mem::swap(&mut drained, &mut rt.return_data);
            drained
        })
    }

    pub fn inspect_storage(account: &str, key: &str) -> Vec<u8> {
        with_runtime(|rt| {
            let lookup = MockRuntime::storage_key(account, key);
            rt.storage.get(&lookup).cloned().unwrap_or_default()
        })
    }

    pub use EventRecord as MockEventRecord;
}

pub(crate) fn read_storage(account: &str, key: &str) -> ContractResult<Vec<u8>> {
    host::read_storage(account, key)
}

pub(crate) fn write_storage(account: &str, key: &str, value: &[u8]) -> ContractResult<()> {
    host::write_storage(account, key, value)
}

pub(crate) fn log_message(message: &str) {
    host::log_message(message);
}

pub(crate) fn emit_event_internal(topic: &str, data: &[u8]) {
    host::emit_event_internal(topic, data);
}

pub fn transfer_tokens(to: &str, amount: u64) -> ContractResult<()> {
    host::transfer_tokens(to, amount)
}

pub(crate) fn get_block_height() -> u64 {
    host::block_height()
}

pub(crate) fn get_block_timestamp() -> u64 {
    host::block_timestamp()
}

pub(crate) fn get_sender_address() -> String {
    host::sender()
}

pub(crate) fn get_contract_addr() -> String {
    host::contract_address()
}

pub(crate) fn get_value() -> u64 {
    host::value()
}

pub(crate) fn read_call_data() -> ContractResult<Vec<u8>> {
    host::read_call_data_internal()
}

pub(crate) fn write_return_data(data: &[u8]) -> ContractResult<()> {
    host::write_return_data_internal(data)
}

/// Hash data with BLAKE3 (public wrapper for crypto module)
pub fn call_hash_blake3(data: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    host::hash_blake3_internal(
        data.as_ptr() as i32,
        data.len() as i32,
        output.as_mut_ptr() as i32,
    );
    output
}

/// Verify signature (public wrapper for crypto module)
pub fn call_verify_signature(
    pubkey: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> ContractResult<bool> {
    let result = host::verify_signature_internal(
        pubkey.as_ptr() as i32,
        message.as_ptr() as i32,
        message.len() as i32,
        signature.as_ptr() as i32,
    );

    match result {
        1 => Ok(true),
        0 => Ok(false),
        _ => Err(ContractError::InvalidSignature),
    }
}

/// Host-accelerated batch hashing helper
pub fn batch_hash_blake3(inputs: &[&[u8]]) -> ContractResult<Vec<[u8; 32]>> {
    host::batch_hash_blake3(inputs)
}

/// Host-accelerated batch signature verification helper
pub fn batch_verify_signatures(
    pubkeys: &[&[u8; 32]],
    messages: &[&[u8]],
    signatures: &[&[u8; 64]],
) -> ContractResult<Vec<bool>> {
    host::batch_verify_signatures(pubkeys, messages, signatures)
}

#[cfg(not(target_arch = "wasm32"))]
pub mod mock {
    use super::host;
    use alloc::string::String;
    use alloc::vec::Vec;

    pub use host::MockEventRecord as EventRecord;

    pub fn reset() {
        host::reset();
    }

    pub fn set_sender(sender: &str) {
        host::set_sender(sender);
    }

    pub fn set_contract_address(addr: &str) {
        host::set_contract_address(addr);
    }

    pub fn set_block_height(height: u64) {
        host::set_block_height(height);
    }

    pub fn set_block_timestamp(timestamp: u64) {
        host::set_block_timestamp(timestamp);
    }

    pub fn set_value(amount: u64) {
        host::set_value(amount);
    }

    pub fn set_call_data(data: &[u8]) {
        host::set_call_data(data);
    }

    pub fn take_events() -> Vec<EventRecord> {
        host::take_events()
    }

    pub fn take_logs() -> Vec<String> {
        host::take_logs()
    }

    pub fn take_return_data() -> Vec<u8> {
        host::take_return_data()
    }

    pub fn inspect_storage(account: &str, key: &str) -> Vec<u8> {
        host::inspect_storage(account, key)
    }
}
