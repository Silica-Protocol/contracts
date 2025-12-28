//! Cryptographic utilities for smart contracts (optimized for WASM with SIMD support)

use crate::error::{ContractError, ContractResult};
use crate::ffi;
use alloc::string::String;
use alloc::vec::Vec;

/// SIMD-enabled cryptographic operations for high-performance batch processing
pub mod simd {
    use super::*;

    /// SIMD-accelerated batch signature verification
    /// Dispatches to the host runtime for vectorized execution
    pub fn batch_verify_signatures_simd(
        pubkeys: &[&[u8; 32]],
        messages: &[&[u8]],
        signatures: &[&[u8; 64]],
    ) -> ContractResult<Vec<bool>> {
        ffi::batch_verify_signatures(pubkeys, messages, signatures)
    }

    /// SIMD-optimized batch hashing for multiple inputs
    pub fn batch_hash_blake3(inputs: &[&[u8]]) -> ContractResult<Vec<[u8; 32]>> {
        ffi::batch_hash_blake3(inputs)
    }

    /// Vectorized XOR operation for cryptographic mixing
    #[inline(always)]
    pub fn xor_bytes(a: &mut [u8], b: &[u8]) {
        assert_eq!(a.len(), b.len(), "XOR inputs must be same length");

        // Use SIMD when available, fallback to scalar
        #[cfg(target_feature = "simd128")]
        unsafe {
            // WebAssembly SIMD XOR - this would be implemented in host
            // For now, use scalar fallback
            xor_bytes_scalar(a, b);
        }

        #[cfg(not(target_feature = "simd128"))]
        xor_bytes_scalar(a, b);
    }

    #[inline(always)]
    fn xor_bytes_scalar(a: &mut [u8], b: &[u8]) {
        for (a_byte, &b_byte) in a.iter_mut().zip(b.iter()) {
            *a_byte ^= b_byte;
        }
    }
}

/// Hash data with BLAKE3 (optimized - direct FFI call)
#[inline(always)]
pub fn hash_blake3(data: &[u8]) -> [u8; 32] {
    ffi::call_hash_blake3(data)
}

/// Verify an Ed25519 signature (optimized - batched verification)
#[inline(always)]
pub fn verify_signature(
    pubkey: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> ContractResult<bool> {
    ffi::call_verify_signature(pubkey, message, signature)
}

/// Batch verify multiple signatures (for efficiency in multi-sig operations)
pub fn batch_verify_signatures(
    pubkeys: &[&[u8; 32]],
    messages: &[&[u8]],
    signatures: &[&[u8; 64]],
) -> ContractResult<Vec<bool>> {
    if pubkeys.len() != messages.len() || pubkeys.len() != signatures.len() {
        return Err(ContractError::InvalidArgument(String::from(
            "Mismatched batch sizes",
        )));
    }

    // Use SIMD acceleration when available
    simd::batch_verify_signatures_simd(pubkeys, messages, signatures)
}

/// Batch verify multiple signatures with fallback implementation
pub fn batch_verify_signatures_fallback(
    pubkeys: &[&[u8; 32]],
    messages: &[&[u8]],
    signatures: &[&[u8; 64]],
) -> ContractResult<Vec<bool>> {
    let mut results = Vec::with_capacity(pubkeys.len());
    for i in 0..pubkeys.len() {
        results.push(verify_signature(pubkeys[i], messages[i], signatures[i])?);
    }
    Ok(results)
}

/// Generate multiple key pairs efficiently (for testing/benchmarking)
pub fn generate_keypairs(count: usize) -> ContractResult<Vec<([u8; 32], [u8; 32])>> {
    use blake3::Hasher;
    use ed25519_dalek::SigningKey;

    const MAX_BATCH: usize = 1_024;
    if count > MAX_BATCH {
        return Err(ContractError::InvalidArgument(String::from(
            "Keypair batch exceeds static bound",
        )));
    }

    let mut pairs = Vec::with_capacity(count);
    for index in 0..count {
        let mut hasher = Hasher::new();
    hasher.update(b"silica-contract-sdk-keygen");
        hasher.update(&(index as u64).to_le_bytes());
        let digest = hasher.finalize();

        let mut seed = [0u8; 32];
        seed.copy_from_slice(&digest.as_bytes()[..32]);

        let signing_key = SigningKey::from_bytes(&seed);
        let public = signing_key.verifying_key().to_bytes();
        let secret = signing_key.to_bytes();
        pairs.push((public, secret));
    }

    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blake3::hash;
    use ed25519_dalek::{Signer, SigningKey};
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn blake3_hash_matches_reference() {
        let data = b"chert-hash";
        let expected = hash(data);
        let actual = hash_blake3(data);
        assert_eq!(actual, *expected.as_bytes());
    }

    #[wasm_bindgen_test]
    fn ed25519_signature_roundtrip() {
        let key_material = generate_keypairs(1).expect("key generation");
        let signing_key = SigningKey::from_bytes(&key_material[0].1);
        let message = b"context integrity";
        let signature = signing_key.sign(message);
        let verified = verify_signature(
            &signing_key.verifying_key().to_bytes(),
            message,
            &signature.to_bytes(),
        )
        .expect("verification should succeed");
        assert!(verified);
    }

    #[wasm_bindgen_test]
    fn batch_verification_consistency() {
        let key_material = generate_keypairs(1).expect("key generation");
        let signing_key = SigningKey::from_bytes(&key_material[0].1);
        let message = b"batch message";
        let signature = signing_key.sign(message);

        let pubkeys = vec![signing_key.verifying_key().to_bytes()];
        let signatures = vec![signature.to_bytes()];
        let messages: Vec<&[u8]> = vec![message.as_ref()];

        let results = batch_verify_signatures(&[&pubkeys[0]], &messages, &[&signatures[0]])
            .expect("batch verification");

        assert_eq!(results.len(), 1);
        assert!(results[0]);
    }

    #[wasm_bindgen_test]
    fn simd_batch_hash_blake3_matches_scalar() {
        let inputs: Vec<&[u8]> = vec![b"alpha".as_ref(), b"beta".as_ref(), b"gamma".as_ref()];
        let simd_hashes = simd::batch_hash_blake3(&inputs).expect("SIMD hashes");

        assert_eq!(simd_hashes.len(), inputs.len());
        for (idx, input) in inputs.iter().enumerate() {
            assert_eq!(simd_hashes[idx], hash_blake3(input));
        }
    }

    #[wasm_bindgen_test]
    fn simd_batch_verify_matches_fallback() {
        let key_material = generate_keypairs(3).expect("keypairs");
        let signing_keys: Vec<SigningKey> = key_material
            .iter()
            .map(|(_, secret)| SigningKey::from_bytes(secret))
            .collect();

        let messages: Vec<Vec<u8>> = vec![b"msg-0".to_vec(), b"msg-1".to_vec(), b"msg-2".to_vec()];

        let mut pubkeys = Vec::with_capacity(signing_keys.len());
        let mut signatures = Vec::with_capacity(signing_keys.len());
        let mut message_refs: Vec<&[u8]> = Vec::with_capacity(signing_keys.len());

        for (idx, signer) in signing_keys.iter().enumerate() {
            pubkeys.push(signer.verifying_key().to_bytes());
            let signature = signer.sign(&messages[idx]);
            signatures.push(signature.to_bytes());
            message_refs.push(messages[idx].as_slice());
        }

        let pubkey_refs: Vec<&[u8; 32]> = pubkeys.iter().collect();
        let signature_refs: Vec<&[u8; 64]> = signatures.iter().collect();

        let simd_results =
            simd::batch_verify_signatures_simd(&pubkey_refs, &message_refs, &signature_refs)
                .expect("SIMD verification");

        let fallback_results =
            batch_verify_signatures_fallback(&pubkey_refs, &message_refs, &signature_refs)
                .expect("fallback verification");

        assert_eq!(simd_results, fallback_results);
    }

    #[wasm_bindgen_test]
    fn generate_keypairs_produces_requested_amount() {
        let pairs = generate_keypairs(4).expect("keypairs");
        assert_eq!(pairs.len(), 4);
        for (public, secret) in pairs {
            assert_eq!(public.len(), 32);
            assert_eq!(secret.len(), 32);
        }
    }
}
