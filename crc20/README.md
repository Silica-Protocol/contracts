# CRC-20 Fungible Token Standard

A standard interface for fungible tokens on Chert Coin blockchain, similar to ERC-20 on Ethereum.

## Features

- ✅ **Transfer** - Send tokens between accounts
- ✅ **Approve/TransferFrom** - Delegated transfers via allowances
- ✅ **Balance Queries** - Check account balances
- ✅ **Total Supply** - Query total token supply
- ✅ **Mint** - Create new tokens (owner only)
- ✅ **Metadata** - Token name, symbol, and decimals
- ✅ **Events** - Transfer and Approval events for indexing

## API Reference

### Initialize

```rust
fn initialize()
```

Initializes the token contract with metadata and mints initial supply to deployer.

**Events:**
- `Transfer { from: "0x0", to: deployer, amount: initial_supply }`

### Transfer

```rust
fn transfer(to: String, amount: u64)
```

Transfers tokens from sender to recipient.

**Requirements:**
- Sender must have sufficient balance
- Amount must be > 0

**Events:**
- `Transfer { from: sender, to: recipient, amount }`

### Approve

```rust
fn approve(spender: String, amount: u64)
```

Approves a spender to transfer tokens on behalf of the sender.

**Events:**
- `Approval { owner: sender, spender, amount }`

### Transfer From

```rust
fn transfer_from(from: String, to: String, amount: u64)
```

Transfers tokens from one account to another using an allowance.

**Requirements:**
- Caller must have sufficient allowance
- From account must have sufficient balance

**Events:**
- `Transfer { from, to, amount }`

### Balance Of

```rust
fn balance_of(account: String) -> u64
```

Returns the token balance of an account.

### Total Supply

```rust
fn total_supply() -> u64
```

Returns the total token supply.

### Decimals

```rust
fn decimals() -> u8
```

Returns the number of decimals (e.g., 18).

### Mint (Owner Only)

```rust
fn mint(to: String, amount: u64)
```

Mints new tokens to an address. Only callable by contract owner.

**Requirements:**
- Caller must be contract owner
- Must not cause overflow

**Events:**
- `Transfer { from: "0x0", to, amount }`

## Building

```bash
# Build optimized WASM
cargo build --target wasm32-unknown-unknown --release

# The output will be in:
# target/wasm32-unknown-unknown/release/crc20_token.wasm
```

## Deployment Example

```rust
// Deploy the contract
let wasm_code = include_bytes!("crc20_token.wasm");
let contract_address = client.deploy_contract(wasm_code).await?;

// Initialize with metadata
client.call_contract(
    contract_address,
    "initialize",
    &InitializeArgs {
        name: "My Token",
        symbol: "MTK",
        decimals: 18,
        initial_supply: 1_000_000_000_000,
    }
).await?;
```

## Usage Example

```rust
// Transfer tokens
client.call_contract(
    token_address,
    "transfer",
    &TransferArgs {
        to: "recipient_address",
        amount: 100_000_000_000,
    }
).await?;

// Approve spending
client.call_contract(
    token_address,
    "approve",
    &ApproveArgs {
        spender: "spender_address",
        amount: 50_000_000_000,
    }
).await?;

// Query balance
let balance: u64 = client.query_contract(
    token_address,
    "balance_of",
    &BalanceOfArgs {
        account: "account_address",
    }
).await?;
```

## Events

### Transfer

```rust
Transfer {
    from: String,
    to: String,
    amount: u64,
}
```

Emitted when tokens are transferred.

### Approval

```rust
Approval {
    owner: String,
    spender: String,
    amount: u64,
}
```

Emitted when an allowance is set.

## Security Considerations

- ✅ Overflow protection on all arithmetic operations
- ✅ Balance checks before transfers
- ✅ Allowance checks before delegated transfers
- ✅ Owner-only mint function
- ✅ Input validation

## License

MIT License
