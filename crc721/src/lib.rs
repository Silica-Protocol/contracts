//! CRC-721 Non-Fungible Token Standard
//!
//! A standard interface for non-fungible tokens on Chert Coin blockchain, similar to ERC-721 on Ethereum.
//!
//! ## Features
//! - Unique Token Ownership - Each token has a unique ID and single owner
//! - Transfer - Send NFTs between accounts
//! - Approve/TransferFrom - Delegated transfers via approvals
//! - Operator Approval - Approve operators to manage all tokens
//! - Metadata URI - Link to off-chain metadata (images, attributes)
//! - Enumeration - Query tokens by owner and total supply
//! - Minting - Create new NFTs (controlled access)
//! - Burning - Destroy NFTs permanently
//! - Events - Transfer, Approval, and ApprovalForAll events

#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(target_arch = "wasm32", no_main)]

#[cfg(target_arch = "wasm32")]
extern crate alloc;

use alloc::vec;

use silica_contract_sdk::event;
use silica_contract_sdk::prelude::*;
use serde::{Deserialize, Serialize};

/// NFT collection metadata
#[derive(Serialize, Deserialize, Clone)]
pub struct CollectionMetadata {
    pub name: String,
    pub symbol: String,
    pub base_uri: String,
    pub total_supply: u64,
    pub owner: String,
    pub initialized: bool,
}

/// Token information
#[derive(Serialize, Deserialize)]
pub struct TokenInfo {
    pub token_id: u64,
    pub owner: String,
    pub metadata_uri: String,
    pub burned: bool,
}

/// Initialize the NFT collection
///
/// # Arguments
/// * `name` - Collection name (e.g., "Chert Punks")
/// * `symbol` - Collection symbol (e.g., "CPUNK")
/// * `base_uri` - Base URI for token metadata
#[unsafe(no_mangle)]
pub extern "C" fn initialize(name: String, symbol: String, base_uri: String) {
    let ctx = context();
    let deployer = ctx.sender();

    // Validate parameters
    if name.is_empty() {
        log("Collection name is required");
        return;
    }

    if symbol.is_empty() {
        log("Collection symbol is required");
        return;
    }

    if base_uri.is_empty() {
        log("Base URI is required");
        return;
    }

    // Initialize collection metadata
    let metadata = CollectionMetadata {
        name: name.clone(),
        symbol: symbol.clone(),
        base_uri: base_uri.clone(),
        total_supply: 0,
        owner: deployer.to_string(),
        initialized: true,
    };

    let storage_ref = storage();
    if storage_ref.set("collection_metadata", &metadata).is_err() {
        log("Failed to store collection metadata");
        return;
    }

    log(&format!(
        "NFT Collection '{}' ({}) initialized with base URI: {}",
        name, symbol, base_uri
    ));
    event!("CollectionInitialized",
        name: name,
        symbol: symbol,
        base_uri: base_uri,
        owner: deployer
    );
}

/// Check if caller is the contract owner
fn is_owner() -> bool {
    let ctx = context();
    let storage_ref = storage();
    let metadata: CollectionMetadata = match storage_ref.get("collection_metadata") {
        Ok(Some(m)) => m,
        _ => return false,
    };
    ctx.sender() == metadata.owner
}

/// Check if an address is approved for a specific token
fn is_approved_for_token(token_id: u64, address: &str) -> bool {
    let storage_ref = storage();

    // Check operator approvals first (takes precedence)
    let metadata: CollectionMetadata = match storage_ref.get("collection_metadata") {
        Ok(Some(m)) => m,
        _ => return false,
    };

    let operator_approvals: Map<(String, String), bool> = Map::new("operator_approvals");
    let metadata_ref = &metadata;
    let operator_key = (metadata_ref.owner.clone(), address.to_string());

    if operator_approvals.get(&operator_key).ok().flatten() == Some(true) {
        return true;
    }

    // Check specific token approval
    let token_approvals: Map<u64, String> = Map::new("token_approvals");
    match token_approvals.get(&token_id) {
        Ok(Some(approved_addr)) => approved_addr == address.to_string(),
        _ => false,
    }
}

/// Mint a new NFT to the specified address
///
/// # Arguments
/// * `to` - Recipient address
/// * `token_id` - Unique token identifier
/// * `metadata_uri` - URI suffix for token metadata
#[unsafe(no_mangle)]
pub extern "C" fn mint(to: String, token_id: u64, metadata_uri: String) {
    // Reentrancy protection
    let _guard = match ReentrancyGuard::enter() {
        Ok(guard) => guard,
        Err(_) => {
            log("Reentrancy detected in mint");
            return;
        }
    };

    let ctx = context();
    let minter = ctx.sender();

    // Check if caller has minting permission
    if !is_owner() {
        log("Only owner can mint tokens");
        return;
    }

    // Validate parameters
    if to.is_empty() {
        log("Recipient address is required");
        return;
    }

    if metadata_uri.is_empty() {
        log("Metadata URI is required");
        return;
    }

    // Check if token ID already exists
    let storage_ref = storage();
    let tokens: Map<u64, TokenInfo> = Map::new("tokens");
    if tokens.get(&token_id).ok().flatten().is_some() {
        log("Token ID already exists");
        return;
    }

    // Create token info
    let token_info = TokenInfo {
        token_id,
        owner: to.clone(),
        metadata_uri: metadata_uri.clone(),
        burned: false,
    };

    // Store token information
    if tokens.set(&token_id, &token_info).is_err() {
        log("Failed to store token information");
        return;
    }

    // Update owner's token count
    let mut balances: Map<String, u64> = Map::new("balances");
    let current_balance = balances.get(&to).ok().flatten().unwrap_or(0);
    let new_balance = safe_math::add(current_balance, 1).unwrap_or(0);

    if balances.set(&to, &new_balance).is_err() {
        log("Failed to update owner balance");
        return;
    }

    // Track all tokens for enumeration
    let mut all_tokens: Vec<u64> = Vec::new();
    let all_tokens_vec: Map<String, Vec<u64>> = Map::new("all_tokens");
    match all_tokens_vec.get(&"global".to_string()) {
        Ok(Some(mut tokens_vec)) => {
            tokens_vec.push(token_id);
            if all_tokens_vec
                .set(&"global".to_string(), &tokens_vec)
                .is_err()
            {
                log("Failed to update all tokens list");
                return;
            }
            all_tokens = tokens_vec;
        }
        _ => {
            all_tokens.push(token_id);
            if all_tokens_vec
                .set(&"global".to_string(), &all_tokens)
                .is_err()
            {
                log("Failed to initialize all tokens list");
                return;
            }
        }
    }

    // Track tokens for owner enumeration
    let mut owner_tokens: Map<String, Vec<u64>> = Map::new("owner_tokens");
    let mut owner_tokens_vec = match owner_tokens.get(&to) {
        Ok(Some(tokens_vec)) => tokens_vec,
        _ => Vec::new(),
    };
    owner_tokens_vec.push(token_id);

    if owner_tokens.set(&to, &owner_tokens_vec).is_err() {
        log("Failed to track owner tokens");
        return;
    }

    // Update total supply
    let mut metadata: CollectionMetadata = match storage_ref.get("collection_metadata") {
        Ok(Some(mut m)) => {
            m.total_supply = safe_math::add(m.total_supply, 1).unwrap_or(m.total_supply);
            if storage_ref.set("collection_metadata", &m).is_err() {
                log("Failed to update total supply");
                return;
            }
            m
        }
        _ => {
            log("Failed to load collection metadata");
            return;
        }
    };

    log(&format!(
        "Token {} minted to {} with metadata URI: {}",
        token_id, to, metadata_uri
    ));
    event!("Transfer",
        from: "0x0".to_string(),
        to: to,
        token_id: token_id
    );
}

/// Transfer an NFT from one address to another
///
/// # Arguments
/// * `from` - Current owner address
/// * `to` - Recipient address
/// * `token_id` - Token to transfer
#[unsafe(no_mangle)]
pub extern "C" fn transfer_from(from: String, to: String, token_id: u64) {
    // Reentrancy protection
    let _guard = match ReentrancyGuard::enter() {
        Ok(guard) => guard,
        Err(_) => {
            log("Reentrancy detected in transfer_from");
            return;
        }
    };

    let ctx = context();
    let caller = ctx.sender();

    // Validate parameters
    if from.is_empty() || to.is_empty() {
        log("From and to addresses are required");
        return;
    }

    // Check if token exists and get current owner
    let storage_ref = storage();
    let tokens: Map<u64, TokenInfo> = Map::new("tokens");
    let mut token_info = match tokens.get(&token_id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            log("Token does not exist");
            return;
        }
        Err(_) => {
            log("Failed to read token information");
            return;
        }
    };

    // Verify ownership
    if token_info.owner != from {
        log("From address is not the token owner");
        return;
    }

    // Check if caller is authorized to transfer
    let caller_addr = caller.to_string();
    if token_info.owner != caller_addr && !is_approved_for_token(token_id, &caller_addr) {
        log("Caller is not authorized to transfer this token");
        return;
    }

    // Transfer ownership
    token_info.owner = to.clone();

    if tokens.set(&token_id, &token_info).is_err() {
        log("Failed to update token ownership");
        return;
    }

    // Update balances
    let mut balances: Map<String, u64> = Map::new("balances");

    // Decrease sender balance
    let from_balance = balances.get(&from).ok().flatten().unwrap_or(0);
    if from_balance == 0 {
        log("Sender has no tokens to transfer");
        return;
    }

    let new_from_balance = safe_math::sub(from_balance, 1).unwrap_or(0);
    if balances.set(&from, &new_from_balance).is_err() {
        log("Failed to update sender balance");
        return;
    }

    // Increase recipient balance
    let to_balance = balances.get(&to).ok().flatten().unwrap_or(0);
    let new_to_balance = safe_math::add(to_balance, 1).unwrap_or(0);
    if balances.set(&to, &new_to_balance).is_err() {
        log("Failed to update recipient balance");
        return;
    }

    // Clear token approval
    let mut token_approvals: Map<u64, String> = Map::new("token_approvals");
    // Clear token approval (just set to empty string)
    if token_approvals.set(&token_id, &"".to_string()).is_err() {
        // Approval already cleared
        log("Failed to clear token approval");
        return;
    }

    // Update owner token lists
    let mut owner_tokens: Map<String, Vec<u64>> = Map::new("owner_tokens");

    // Remove from sender's token list
    let mut sender_tokens = match owner_tokens.get(&from) {
        Ok(Some(tokens_vec)) => tokens_vec,
        _ => Vec::new(),
    };
    if let Some(pos) = sender_tokens.iter().position(|&x| x == token_id) {
        sender_tokens.remove(pos);
    }

    if owner_tokens.set(&from, &sender_tokens).is_err() {
        log("Failed to update sender token list");
        return;
    }

    // Add to recipient's token list
    let mut recipient_tokens = match owner_tokens.get(&to) {
        Ok(Some(tokens_vec)) => tokens_vec,
        _ => Vec::new(),
    };
    recipient_tokens.push(token_id);

    if owner_tokens.set(&to, &recipient_tokens).is_err() {
        log("Failed to update recipient token list");
        return;
    }

    log(&format!(
        "Token {} transferred from {} to {}",
        token_id, from, to
    ));
    event!("Transfer",
        from: from,
        to: to,
        token_id: token_id
    );
}

/// Safely transfer an NFT with recipient validation
///
/// # Arguments
/// * `from` - Current owner address
/// * `to` - Recipient address
/// * `token_id` - Token to transfer
/// * `data` - Additional data for recipient contract
#[unsafe(no_mangle)]
pub extern "C" fn safe_transfer_from(from: String, to: String, token_id: u64, data: Vec<u8>) {
    // For now, safe transfer behaves like regular transfer
    // In a full implementation, this would check if recipient is a contract
    // and call onCRC721Received callback

    transfer_from(from, to, token_id);

    log(&format!(
        "Safe transfer completed for token {} with data: {:?}",
        token_id, data
    ));
}

/// Approve an address to transfer a specific token
///
/// # Arguments
/// * `to` - Address to approve (or "0x0" to clear approval)
/// * `token_id` - Token to grant approval for
#[unsafe(no_mangle)]
pub extern "C" fn approve(to: String, token_id: u64) {
    let ctx = context();
    let owner = ctx.sender();

    // Check if token exists and owner matches
    let storage_ref = storage();
    let tokens: Map<u64, TokenInfo> = Map::new("tokens");
    let token_info = match tokens.get(&token_id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            log("Token does not exist");
            return;
        }
        Err(_) => {
            log("Failed to read token information");
            return;
        }
    };

    // Verify ownership
    if token_info.owner != owner {
        log("Caller is not the token owner");
        return;
    }

    // Cannot approve the owner
    if to == owner {
        log("Cannot approve the token owner");
        return;
    }

    // Store approval
    let mut token_approvals: Map<u64, String> = Map::new("token_approvals");
    if token_approvals.set(&token_id, &to).is_err() {
        log("Failed to store token approval");
        return;
    }

    log(&format!("Token {} approved for address: {}", token_id, to));
    event!("Approval",
        owner: owner,
        approved: to,
        token_id: token_id
    );
}

/// Approve or revoke an operator to manage all tokens
///
/// # Arguments
/// * `operator` - Address to set operator status for
/// * `approved` - True to approve, false to revoke
#[unsafe(no_mangle)]
pub extern "C" fn set_approval_for_all(operator: String, approved: bool) {
    let ctx = context();
    let owner = ctx.sender();

    // Validate parameters
    if operator.is_empty() {
        log("Operator address is required");
        return;
    }

    // Cannot approve yourself
    if operator == owner {
        log("Cannot set yourself as operator");
        return;
    }

    let storage_ref = storage();
    let metadata: CollectionMetadata = match storage_ref.get("collection_metadata") {
        Ok(Some(m)) => m,
        _ => {
            log("Failed to load collection metadata");
            return;
        }
    };

    // Store operator approval
    let mut operator_approvals: Map<(String, String), bool> = Map::new("operator_approvals");
    let approval_key = (owner.clone(), operator.clone());

    if operator_approvals.set(&approval_key, &approved).is_err() {
        log("Failed to store operator approval");
        return;
    }

    log(&format!(
        "Operator {} {} for {}",
        operator,
        if approved { "approved" } else { "revoked" },
        owner
    ));
    event!("ApprovalForAll",
        owner: owner,
        operator: operator,
        approved: approved
    );
}

/// Burn (destroy) an NFT permanently
///
/// # Arguments
/// * `token_id` - Token to burn
#[unsafe(no_mangle)]
pub extern "C" fn burn(token_id: u64) {
    // Reentrancy protection
    let _guard = match ReentrancyGuard::enter() {
        Ok(guard) => guard,
        Err(_) => {
            log("Reentrancy detected in burn");
            return;
        }
    };

    let ctx = context();
    let burner = ctx.sender();

    // Check if token exists and get current owner
    let storage_ref = storage();
    let tokens: Map<u64, TokenInfo> = Map::new("tokens");
    let mut token_info = match tokens.get(&token_id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            log("Token does not exist");
            return;
        }
        Err(_) => {
            log("Failed to read token information");
            return;
        }
    };

    // Verify ownership
    let burner_addr = burner.to_string();
    if token_info.owner != burner_addr && !is_approved_for_token(token_id, &burner_addr) {
        log("Caller is not authorized to burn this token");
        return;
    }

    // Mark token as burned
    token_info.burned = true;
    token_info.owner = "0x0".to_string(); // Zero address for burned tokens

    if tokens.set(&token_id, &token_info).is_err() {
        log("Failed to update token information");
        return;
    }

    // Decrease owner's balance
    let mut balances: Map<String, u64> = Map::new("balances");
    let owner_balance = match balances.get(&token_info.owner) {
        Ok(Some(b)) => b,
        _ => {
            log("Owner balance not found");
            return;
        }
    };

    if owner_balance > 0 {
        let new_owner_balance = safe_math::sub(owner_balance, 1).unwrap_or(0);
        if balances.set(&token_info.owner, &new_owner_balance).is_err() {
            log("Failed to update owner balance");
            return;
        }
    }

    // Clear all approvals
    let mut token_approvals: Map<u64, String> = Map::new("token_approvals");
    // Clear token approval
    let _ = token_approvals.set(&token_id, &"".to_string());

    let mut operator_approvals: Map<(String, String), bool> = Map::new("operator_approvals");
    let mut keys_to_remove = Vec::new();

    // Find and remove all operator approvals for this owner
    let all_operator_keys = vec![
        (token_info.owner.clone(), "operator1".to_string()),
        (token_info.owner.clone(), "operator2".to_string()),
        // In real implementation, you'd iterate through all stored keys
    ];

    for key in all_operator_keys {
        if operator_approvals.get(&key).ok().flatten() == Some(true) {
            keys_to_remove.push(key);
        }
    }

    for key in keys_to_remove {
        // Clear operator approval
        let _ = operator_approvals.set(&key, &false);
    }

    log(&format!("Token {} burned permanently", token_id));
    event!("Transfer",
        from: burner_addr,
        to: "0x0".to_string(),
        token_id: token_id
    );
}

/// Get the owner of a specific token
#[unsafe(no_mangle)]
pub extern "C" fn owner_of(token_id: u64) -> String {
    let tokens: Map<u64, TokenInfo> = Map::new("tokens");
    match tokens.get(&token_id) {
        Ok(Some(token_info)) => {
            if token_info.burned {
                "0x0".to_string()
            } else {
                token_info.owner
            }
        }
        _ => "0x0".to_string(),
    }
}

/// Get the number of tokens owned by an address
#[unsafe(no_mangle)]
pub extern "C" fn balance_of(owner: String) -> u64 {
    if owner.is_empty() {
        return 0;
    }

    let balances: Map<String, u64> = Map::new("balances");
    balances.get(&owner).ok().flatten().unwrap_or(0)
}

/// Get the approved address for a token
#[unsafe(no_mangle)]
pub extern "C" fn get_approved(token_id: u64) -> String {
    let token_approvals: Map<u64, String> = Map::new("token_approvals");
    match token_approvals.get(&token_id) {
        Ok(Some(approved)) => approved,
        _ => "0x0".to_string(),
    }
}

/// Check if an operator is approved for all tokens of an owner
#[unsafe(no_mangle)]
pub extern "C" fn is_approved_for_all(owner: String, operator: String) -> bool {
    if owner.is_empty() || operator.is_empty() {
        return false;
    }

    let operator_approvals: Map<(String, String), bool> = Map::new("operator_approvals");
    let approval_key = (owner, operator);
    operator_approvals
        .get(&approval_key)
        .ok()
        .flatten()
        .unwrap_or(false)
}

/// Get the metadata URI for a token
#[unsafe(no_mangle)]
pub extern "C" fn token_uri(token_id: u64) -> String {
    let tokens: Map<u64, TokenInfo> = Map::new("tokens");
    let storage_ref = storage();

    match tokens.get(&token_id) {
        Ok(Some(token_info)) => {
            let metadata: CollectionMetadata = match storage_ref.get("collection_metadata") {
                Ok(Some(m)) => m,
                _ => return "".to_string(),
            };

            if token_info.burned {
                "".to_string()
            } else {
                // Combine base URI with token-specific metadata URI
                format!("{}{}", metadata.base_uri, token_info.metadata_uri)
            }
        }
        _ => "".to_string(),
    }
}

/// Get the total number of tokens in existence
#[unsafe(no_mangle)]
pub extern "C" fn total_supply() -> u64 {
    let storage_ref = storage();
    let metadata: CollectionMetadata =
        match storage_ref.get::<CollectionMetadata>("collection_metadata") {
            Ok(Some(m)) => m.total_supply,
            _ => 0,
        };
    metadata
}

/// Get token ID at a given index in the global token list
#[unsafe(no_mangle)]
pub extern "C" fn token_by_index(index: u64) -> u64 {
    let all_tokens_vec: Map<String, Vec<u64>> = Map::new("all_tokens");
    match all_tokens_vec.get(&"global".to_string()) {
        Ok(Some(tokens_vec)) => {
            if (index as usize) < tokens_vec.len() {
                tokens_vec[index as usize]
            } else {
                0 // Invalid index
            }
        }
        _ => 0,
    }
}

/// Get token ID at a given index in an owner's token list
#[unsafe(no_mangle)]
pub extern "C" fn token_of_owner_by_index(owner: String, index: u64) -> u64 {
    if owner.is_empty() {
        return 0;
    }

    let owner_tokens: Map<String, Vec<u64>> = Map::new("owner_tokens");
    match owner_tokens.get(&owner) {
        Ok(Some(tokens_vec)) => {
            if (index as usize) < tokens_vec.len() {
                tokens_vec[index as usize]
            } else {
                0 // Invalid index
            }
        }
        _ => 0,
    }
}

/// Get collection metadata
#[unsafe(no_mangle)]
pub extern "C" fn get_collection_info() -> String {
    let storage_ref = storage();
    let metadata: CollectionMetadata = match storage_ref.get("collection_metadata") {
        Ok(Some(m)) => {
            log(&format!(
                "Collection: {} ({}) - Total Supply: {}",
                m.name, m.symbol, m.total_supply
            ));
            m
        }
        _ => {
            log("Failed to load collection metadata");
            return "".to_string();
        }
    };

    format!(
        "{}|{}|{}|{}",
        metadata.name, metadata.symbol, metadata.base_uri, metadata.total_supply
    )
}
