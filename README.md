# Chert Coin Smart Contracts

Rust/WASM smart contracts and contract templates for the Chert Coin platform.

## 📋 Quick Start

**Status:** 🚧 In Development

### For Contract Developers

The smart contract system is currently being built. See our roadmap below.

**When ready, you'll be able to:**
```rust
use silica_contract_sdk::prelude::*;

#[contract_entrypoint]
pub fn transfer(to: String, amount: u64) -> ContractResult<()> {
    // Your contract logic here
    Ok(())
}
```

**Build and deploy:**
```bash
cargo build --target wasm32-unknown-unknown --release
chert-cli contract deploy contract.wasm
```

### For Core Contributors

We're building the infrastructure to support smart contracts. See the development plan below.

## 📚 Documentation

- **[Smart Contracts Roadmap](../SMART_CONTRACTS_ROADMAP.md)** - Complete 5-6 month implementation plan
- **[Q&A Summary](../SMART_CONTRACTS_QA.md)** - Quick answers to common questions
- **[Rust SDK Specification](RUST_SDK_SPEC.md)** - Detailed SDK design and architecture

## 🏗️ Current Architecture

**The VM is already integrated into Silica!**

### Existing Infrastructure ✅

Located in `silica/src/`:
- ✅ **WASM Runtime** (`wasm.rs`) - Wasmtime v20.0 with security-focused configuration
- ✅ **Execution Engine** (`execution.rs`) - Parallel lane execution for conflict-free processing
- ✅ **Compute Unit Metering** - Gas-like accounting for contract execution
- ✅ **Host Functions** - Basic storage access (state_read, state_write, log)
- ✅ **Storage Integration** - Arc<StorageManager> for persistent state

### What We're Building 🚧

**Phase 1: Contract SDK** (3-4 weeks)
- Rust SDK for writing WASM contracts (`contracts/sdk/`)
- Storage abstractions (Map, Vector, Set)
- Execution context API (sender, block info)
- Event emission system

**Phase 2: Enhanced Runtime** (2-3 weeks)
- Additional host functions (events, transfers, crypto)
- Cross-contract calls with reentrancy protection
- Async storage access

**Phase 3: Deployment System** (2-3 weeks)
- DeployContract transaction type
- Contract metadata and ABI
- Deployment validation and security

**Phase 4: Contract API** (2 weeks)
- RPC endpoints for deployment and calls
- TypeScript SDK integration
- CLI tools

**Phase 5: Standard Templates** (6-8 weeks)
- CRC-20 fungible tokens
- CRC-721 NFTs
- DEX/AMM
- Multisig wallets
- Timelock contracts
- Oracle integration
- Privacy tokens
- Cross-shard bridges
- Staking pools
- DAO governance

## 🎯 Standard Contracts

We're building 10 essential contract templates beyond system contracts:

| Contract | Purpose | Status |
|----------|---------|--------|
| **CRC-20** | Fungible token standard (like ERC-20) | 📋 Planned |
| **CRC-721** | Non-fungible token (NFT) standard | 📋 Planned |
| **DEX/AMM** | Decentralized exchange with liquidity pools | 📋 Planned |
| **Multisig** | M-of-N signature wallet | 📋 Planned |
| **Timelock** | Delayed execution for security | 📋 Planned |
| **Oracle** | Off-chain data bridge | 📋 Planned |
| **Privacy Token** | Confidential transactions | 📋 Planned |
| **Cross-Shard** | Inter-shard communication | 📋 Planned |
| **Staking Pool** | Liquid staking delegation | 📋 Planned |
| **DAO** | On-chain governance | 📋 Planned |

## 🔧 Repository Structure

```
contracts/
├── README.md                   # This file
├── RUST_SDK_SPEC.md            # Detailed SDK specification
├── sdk/                        # Contract SDK (to be created)
│   ├── src/
│   │   ├── storage/            # Storage abstractions
│   │   ├── context.rs          # Execution context
│   │   ├── events.rs           # Event system
│   │   └── ffi.rs              # Host function bindings
│   └── examples/               # Example contracts
├── crc20/                      # Fungible token standard
├── crc721/                     # NFT standard
├── dex/                        # Decentralized exchange
└── ...
```

## ⚙️ VM Location

**Important:** The `/vm` folder is **NOT NEEDED** and can be removed.

All VM functionality is already integrated into the `silica/` folder:
- `silica/src/wasm.rs` - WASM runtime with Wasmtime
- `silica/src/execution.rs` - Execution engine with parallel lanes
- `silica/src/config.rs` - ExecutionConfig and WasmConfig

## 🚀 Getting Involved

### Priority Tasks

1. **Build the Contract SDK** - Create `contracts/sdk/` crate
2. **Extend Host Functions** - Add more capabilities to the runtime
3. **Implement Deployment** - Add contract deployment transactions
4. **Create Templates** - Build standard contract implementations

### How to Contribute

```bash
# Start with the SDK
cd contracts
mkdir sdk
cd sdk
cargo init --lib

# Follow the specification
# See: contracts/RUST_SDK_SPEC.md
```

## 📖 Additional Resources

- **Main Roadmap:** [SMART_CONTRACTS_ROADMAP.md](../SMART_CONTRACTS_ROADMAP.md)
- **Q&A:** [SMART_CONTRACTS_QA.md](../SMART_CONTRACTS_QA.md)
- **SDK Spec:** [RUST_SDK_SPEC.md](RUST_SDK_SPEC.md)
- **Security Guidelines:** [../SECURITY_GUIDELINES.md](../SECURITY_GUIDELINES.md)
- **Client SDKs:** [../sdk/README.md](../sdk/README.md)

## 🎯 Timeline

**Total Estimated Effort:** 20-25 weeks (5-6 months)

**Critical Path:**
1. Contract SDK (3-4 weeks)
2. Runtime Enhancement (2-3 weeks)
3. Deployment System (2-3 weeks)
4. Contract API (2 weeks)
5. Standard Templates (6-8 weeks, parallelizable)
6. Testing & Security (continuous)
7. Documentation (continuous)

**See the full roadmap for detailed specifications and milestones.**

---

**Questions?** Check [SMART_CONTRACTS_QA.md](../SMART_CONTRACTS_QA.md) for quick answers!
