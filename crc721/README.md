# CRC-721 Non-Fungible Token (NFT) Standard

A standard interface for non-fungible tokens on Chert Coin blockchain, similar to ERC-721 on Ethereum.

## Features

- ✅ **Unique Token Ownership** - Each token has a unique ID and single owner
- ✅ **Transfer** - Send NFTs between accounts
- ✅ **Approve/TransferFrom** - Delegated transfers via approvals
- ✅ **Operator Approval** - Approve operators to manage all tokens
- ✅ **Metadata URI** - Link to off-chain metadata (images, attributes)
- ✅ **Enumeration** - Query tokens by owner and total supply
- ✅ **Minting** - Create new NFTs (controlled access)
- ✅ **Burning** - Destroy NFTs permanently
- ✅ **Events** - Transfer, Approval, and ApprovalForAll events

## Use Cases

- 🎨 **Digital Art & Collectibles** - Unique artwork, trading cards
- 🎮 **Gaming Assets** - In-game items, characters, weapons
- 🏠 **Real Estate** - Property deeds, land parcels
- 🎫 **Tickets & Memberships** - Event tickets, club memberships
- 📜 **Certificates** - Diplomas, licenses, achievements
- 🌐 **Domain Names** - Blockchain-based domain ownership

## API Reference

### Initialize

```rust
fn initialize(name: String, symbol: String, base_uri: String)
```

Initializes the NFT collection with metadata.

**Parameters:**
- `name` - Collection name (e.g., "Chert Punks")
- `symbol` - Collection symbol (e.g., "CPUNK")
- `base_uri` - Base URI for token metadata (e.g., "https://api.example.com/metadata/")

**Requirements:**
- Can only be called once during deployment
- Caller becomes the contract owner

### Mint

```rust
fn mint(to: String, token_id: u64, metadata_uri: String)
```

Mints a new NFT to the specified address.

**Parameters:**
- `to` - Recipient address
- `token_id` - Unique token identifier
- `metadata_uri` - URI suffix for token metadata (appended to base_uri)

**Requirements:**
- Only owner or approved minter can call
- Token ID must not already exist
- Recipient address must be valid

**Events:**
- `Transfer { from: "0x0", to: recipient, token_id }`

### Transfer

```rust
fn transfer_from(from: String, to: String, token_id: u64)
```

Transfers an NFT from one address to another.

**Parameters:**
- `from` - Current owner address
- `to` - Recipient address
- `token_id` - Token to transfer

**Requirements:**
- Caller must be owner, approved address, or approved operator
- Token must exist
- Recipient must not be zero address

**Events:**
- `Transfer { from: sender, to: recipient, token_id }`
- Clears any existing approvals for the token

### Safe Transfer

```rust
fn safe_transfer_from(from: String, to: String, token_id: u64, data: Vec<u8>)
```

Safely transfers an NFT with additional data and recipient validation.

**Parameters:**
- `from` - Current owner address
- `to` - Recipient address
- `token_id` - Token to transfer
- `data` - Additional data for recipient contract

**Requirements:**
- Same as `transfer_from`
- If recipient is a contract, it must implement `onCRC721Received` callback
- Recipient contract must return acceptance magic value

### Approve

```rust
fn approve(to: String, token_id: u64)
```

Approves an address to transfer a specific token.

**Parameters:**
- `to` - Address to approve (or "0x0" to clear approval)
- `token_id` - Token to grant approval for

**Requirements:**
- Caller must be token owner or approved operator
- Cannot approve current owner

**Events:**
- `Approval { owner, approved: to, token_id }`

### Set Approval For All

```rust
fn set_approval_for_all(operator: String, approved: bool)
```

Approves or revokes an operator to manage all of caller's tokens.

**Parameters:**
- `operator` - Address to set operator status for
- `approved` - True to approve, false to revoke

**Requirements:**
- Operator cannot be caller
- Operator address must be valid

**Events:**
- `ApprovalForAll { owner: caller, operator, approved }`

### Burn

```rust
fn burn(token_id: u64)
```

Permanently destroys an NFT.

**Parameters:**
- `token_id` - Token to burn

**Requirements:**
- Caller must be owner or approved
- Token must exist

**Events:**
- `Transfer { from: owner, to: "0x0", token_id }`
- Clears all approvals

## Query Functions

### Owner Of

```rust
fn owner_of(token_id: u64) -> String
```

Returns the owner of a specific token.

**Returns:** Owner address or error if token doesn't exist

### Balance Of

```rust
fn balance_of(owner: String) -> u64
```

Returns the number of tokens owned by an address.

**Returns:** Token count

### Get Approved

```rust
fn get_approved(token_id: u64) -> Option<String>
```

Returns the approved address for a token.

**Returns:** Approved address or None

### Is Approved For All

```rust
fn is_approved_for_all(owner: String, operator: String) -> bool
```

Returns whether an operator is approved for all tokens of an owner.

**Returns:** True if approved, false otherwise

### Token URI

```rust
fn token_uri(token_id: u64) -> String
```

Returns the metadata URI for a token.

**Returns:** Full metadata URI (base_uri + token's metadata_uri)

### Total Supply

```rust
fn total_supply() -> u64
```

Returns the total number of tokens in existence.

**Returns:** Total supply count

### Token By Index

```rust
fn token_by_index(index: u64) -> Option<u64>
```

Returns token ID at a given index in all tokens list.

**Returns:** Token ID or None if index out of bounds

### Token Of Owner By Index

```rust
fn token_of_owner_by_index(owner: String, index: u64) -> Option<u64>
```

Returns token ID at a given index in owner's token list.

**Returns:** Token ID or None if index out of bounds

## Events

```rust
// Emitted when token is transferred
event Transfer {
    from: String,
    to: String,
    token_id: u64,
}

// Emitted when token approval is set
event Approval {
    owner: String,
    approved: String,
    token_id: u64,
}

// Emitted when operator approval is set
event ApprovalForAll {
    owner: String,
    operator: String,
    approved: bool,
}
```

## Storage Layout

```rust
// Token ownership: token_id -> owner
Map<u64, String>: "owners"

// Token balances: owner -> count
Map<String, u64>: "balances"

// Token approvals: token_id -> approved_address
Map<u64, String>: "token_approvals"

// Operator approvals: (owner, operator) -> bool
Map<(String, String), bool>: "operator_approvals"

// Token metadata URIs: token_id -> uri
Map<u64, String>: "token_uris"

// All tokens enumeration: index -> token_id
Vector<u64>: "all_tokens"

// Owner tokens enumeration: owner -> [token_ids]
Map<String, Vector<u64>>: "owner_tokens"

// Collection metadata
String: "name"
String: "symbol"
String: "base_uri"
u64: "total_supply"
String: "owner"
```

## Security Considerations

### Reentrancy Protection
- All state changes occur before external calls
- Safe transfer callbacks execute after ownership transfer

### Access Control
- Strict ownership and approval checks
- Owner-only functions for minting and admin operations
- Operator approval is per-owner, not global

### Input Validation
- Token ID existence checks
- Address validation (non-zero for recipients)
- Duplicate token ID prevention on mint

### Integer Safety
- Balance overflow protection
- Token ID uniqueness enforcement
- Index bounds checking for enumeration

## Metadata Standard

Tokens should link to JSON metadata following this schema:

```json
{
  "name": "Token Name #123",
  "description": "Description of the NFT",
  "image": "https://example.com/image.png",
  "external_url": "https://example.com/token/123",
  "attributes": [
    {
      "trait_type": "Rarity",
      "value": "Legendary"
    },
    {
      "trait_type": "Power",
      "value": 95,
      "max_value": 100
    }
  ]
}
```

## Extensions

### CRC-721 Metadata Extension
- `name()` - Collection name
- `symbol()` - Collection symbol
- `token_uri(token_id)` - Token metadata URI

### CRC-721 Enumerable Extension
- `total_supply()` - Total tokens minted
- `token_by_index(index)` - Token ID by global index
- `token_of_owner_by_index(owner, index)` - Token ID by owner index

### CRC-721 Burnable Extension
- `burn(token_id)` - Destroy token permanently

## Example Usage

### Deploying an NFT Collection

```rust
// Deploy contract
let contract = deploy_crc721();

// Initialize collection
contract.initialize(
    "Chert Punks".to_string(),
    "CPUNK".to_string(),
    "https://api.chertpunks.io/metadata/".to_string()
);
```

### Minting NFTs

```rust
// Mint token #1 to Alice
contract.mint(
    "chert_1alice...".to_string(),
    1,
    "1.json".to_string()  // Full URI: base_uri + "1.json"
);

// Token URI: https://api.chertpunks.io/metadata/1.json
```

### Transferring NFTs

```rust
// Transfer token from Alice to Bob
contract.transfer_from(
    "chert_1alice...".to_string(),
    "chert_1bob...".to_string(),
    1
);
```

### Approving Operators

```rust
// Alice approves marketplace as operator
contract.set_approval_for_all(
    "chert_1marketplace...".to_string(),
    true
);

// Marketplace can now transfer any of Alice's tokens
```

## Integration with Marketplaces

This standard is designed for seamless integration with NFT marketplaces:

1. **Listing** - Owner approves marketplace as operator
2. **Sale** - Marketplace calls `transfer_from` when sold
3. **Royalties** - Implement CRC-2981 (Royalty Standard) extension
4. **Metadata** - Marketplace fetches from `token_uri()`

## Differences from ERC-721

- ✅ **Native Integration** - Works with Chert's sharding and consensus
- ✅ **Lower Gas Costs** - Optimized storage layout and compute units
- ✅ **Post-Quantum Ready** - Compatible with Dilithium signatures
- ✅ **Built-in Enumeration** - No separate extension deployment needed

## Testing Checklist

- [ ] Mint tokens with unique IDs
- [ ] Transfer tokens between accounts
- [ ] Approve specific addresses for tokens
- [ ] Approve operators for all tokens
- [ ] Safe transfer with recipient validation
- [ ] Burn tokens permanently
- [ ] Query ownership and balances
- [ ] Enumerate tokens by owner and globally
- [ ] Metadata URI resolution
- [ ] Access control enforcement
- [ ] Reentrancy attack resistance
- [ ] Integer overflow protection

## License

MIT License - See LICENSE file for details

## References

- [ERC-721 Specification](https://eips.ethereum.org/EIPS/eip-721)
- [OpenZeppelin ERC-721](https://docs.openzeppelin.com/contracts/erc721)
- [NFT Metadata Standard](https://docs.opensea.io/docs/metadata-standards)

## Status

🚧 **In Development** - Implementation in progress

**Estimated Completion:** Q1 2026
