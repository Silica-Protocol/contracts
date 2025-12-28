# Silica Contract SDK

Rust SDK for writing smart contracts on the Silica network.

## Features

- ✅ **Type-Safe Storage** - Map, Vector, and Set abstractions
- ✅ **Context API** - Access sender, block info, and transaction data
- ✅ **Event System** - Emit events for off-chain indexing
- ✅ **Cryptographic Utilities** - BLAKE3 hashing and signature verification
- ✅ **No-Std Compatible** - Works in WASM environment without std library

## Quick Start

### 1. Add Dependency

```toml
[dependencies]
silica-contract-sdk = { path = "path/to/contracts/sdk" }

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "z"
lto = true
```

### 2. Write Your Contract

```rust
#![no_std]
#![no_main]

extern crate alloc;
use silica_contract_sdk::prelude::*;

#[no_mangle]
pub extern "C" fn transfer() {
    let ctx = try_context().expect("execution context available");
    let sender = ctx.sender();
    let mut balances: Map<String, u64> = Map::new("balances");
    
    // Your contract logic here
    log(&format!("Transfer from {}", sender));
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

### 3. Build to WASM

```bash
cargo build --target wasm32-unknown-unknown --release
```

## API Reference

### Storage

```rust
use silica_contract_sdk::prelude::*;

// Low-level storage
let mut storage = storage();
storage.set("key", &value)?;
let value: T = storage.get("key")?.unwrap();

// Type-safe Map
let mut balances: Map<String, u64> = Map::new("balances");
balances.set(&"alice".to_string(), &100)?;
let balance = balances.get(&"alice".to_string())?;

// Type-safe Vector
let mut items: Vector<String> = Vector::new("items");
items.push(&"item1".to_string())?;
let item = items.get(0)?;
```

### Context

```rust
use silica_contract_sdk::prelude::*;

let ctx = try_context()?;
let sender = ctx.sender();              // Transaction sender
let contract = ctx.contract_address();  // This contract's address
let height = ctx.block_height();        // Current block height
let timestamp = ctx.block_timestamp();  // Unix timestamp
let value = ctx.value();                // Tokens sent with transaction
```

### Events

```rust
use silica_contract_sdk::prelude::*;

// Simple log
log("Contract executed successfully");

// Structured event
event!("Transfer", from: sender, to: recipient, amount: 100);
```

### Cryptography

```rust
use silica_contract_sdk::crypto;

// Hash data
let hash = crypto::hash_blake3(data);

// Verify signature
let is_valid = crypto::verify_signature(&pubkey, &message, &signature)?;
```

## Examples

See the `examples/` directory:
- **counter.rs** - Simple counter with increment/decrement
- **simple_storage.rs** - Key-value storage contract

## Error Handling

All operations return `ContractResult<T>`:

```rust
use silica_contract_sdk::prelude::*;

fn my_function() -> ContractResult<()> {
    let ctx = context();
    
    if ctx.value() < 100 {
        return Err(ContractError::InsufficientBalance {
            required: 100,
            available: ctx.value(),
        });
    }
    
    Ok(())
}
```

## Testing

Contracts can be tested using cargo test with mock environments:

```bash
cargo test
```

## Building for Production

```bash
# Build optimized WASM
cargo build --target wasm32-unknown-unknown --release

# Optional: Further optimize with wasm-opt
wasm-opt -Oz input.wasm -o output.wasm
```

## Security Considerations

- Always validate inputs
- Use checked arithmetic for overflow protection
- Implement access control for sensitive operations
- Avoid reentrancy vulnerabilities
- Test thoroughly before deployment

## License

MIT License - see LICENSE file for details
