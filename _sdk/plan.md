Excellent point! You're absolutely right - this changes the architecture significantly and makes things much cleaner.

## Updated Architecture with TypeScript PQC

### **For WASM Wallet:**

Since PQC libraries (including Halo2) have TypeScript/JavaScript implementations, you can:

**Option A: Pure TypeScript Crypto (RECOMMENDED for Wallet)**
```typescript
// In wallet frontend
import { Kyber } from '@noble/post-quantum/kyber';
import { Dilithium } from '@noble/post-quantum/ml-dsl';
// Or whatever TS libraries you're using

// Client-side encryption, signature generation
const keypair = Kyber.keygen();
const ciphertext = Kyber.encrypt(publicKey, message);
```

**Benefits:**
- ✅ No need to compile Rust crypto to WASM
- ✅ Smaller bundle size (pure JS is more efficient than WASM for small ops)
- ✅ Better browser compatibility
- ✅ Easier debugging

### **Revised silica-utilities Feature Structure:**

```toml
[package]
name = "silica-utilities"

[features]
default = ["std"]

# Full native support (miner, node)
std = [
    "serde/std",
    "anyhow",
    "chrono",
    "tracing",
    "halo2_proofs",
    "halo2_gadgets",
    "pqcrypto-dilithium",
    "pqcrypto-kyber",
]

# Contracts - minimal crypto only
no_std = []

# Note: NO wasm feature needed!
# Wallet will use TypeScript crypto libraries instead

[dependencies]
silica-models = { path = "../silica-models" }

# Always available (no_std compatible)
blake3 = { version = "1.5", default-features = false }
ed25519-dalek = { version = "2.1", default-features = false }
sha2 = { version = "0.10", default-features = false }
sha3 = { version = "0.10", default-features = false }

# std-only dependencies
anyhow = { version = "1.0", optional = true }
chrono = { version = "0.4", optional = true }
tracing = { version = "0.1", optional = true }
halo2_proofs = { version = "0.3", optional = true }
halo2_gadgets = { version = "0.3", optional = true }
pqcrypto-dilithium = { version = "0.5", optional = true }
pqcrypto-kyber = { version = "0.4", optional = true }
```

## Simplified Use Cases

### 1. **Smart Contracts** (no_std)
```toml
# contracts/_sdk/Cargo.toml
[dependencies]
silica-models = { path = "../../silica-models" }
silica-utilities = { path = "../../silica-utilities", default-features = false }
```
- Gets basic types from models
- Minimal crypto (BLAKE3, ED25519) from utilities
- PQC verification done via FFI host functions

### 2. **Miner/Node** (std)
```toml
# miner/Cargo.toml
[dependencies]
silica-models = { path = "../silica-models" }
silica-utilities = { path = "../silica-utilities", features = ["std"] }
```
- Full crypto suite including Halo2 and PQC
- All validation and proof generation
- System access (file I/O, networking, etc.)

### 3. **Wallet** (TypeScript/Browser)
```json
// package.json
{
  "dependencies": {
    "@noble/post-quantum": "^0.2.0",
    "@noble/hashes": "^1.3.0",
    "@noble/curves": "^1.3.0"
  }
}
```
- Uses TypeScript implementations directly
- NO Rust WASM compilation needed for crypto
- May still use `silica-models` compiled to WASM for transaction serialization

## Benefits of This Approach

### ✅ **Cleaner Separation:**
- **Rust backend** = Node/miner validation & proof generation
- **TypeScript frontend** = Wallet crypto operations
- **Shared models** = Data structures only

### ✅ **Performance:**
- PQC operations in browser are faster in native JS/TS than WASM
- Smaller bundle sizes (no need to ship Rust crypto WASM)
- Better tree-shaking with JS modules

### ✅ **Simpler Build:**
```bash
# No complex WASM feature management
cargo build --target wasm32-unknown-unknown --no-default-features  # contracts
cargo build --features std  # miner/node
npm run build  # wallet (pure TypeScript)
```

### ✅ **Real-World Validation:**
You mentioned PSY uses TypeScript for client encryption - this proves the pattern works in production!

## Updated Implementation Plan

### Phase 1: silica-models (1 day)
- Pure data structures
- Serde + postcard only
- Export to both Rust and TypeScript (via wasm-bindgen or typeshare)

### Phase 2: silica-utilities (2 days)
- Remove WASM feature entirely
- Focus on `std` and `no_std` only
- Heavy crypto stays in `std` feature

### Phase 3: Wallet TypeScript Crypto (1-2 days)
```typescript
// wallet/src/crypto/
├── kyber.ts      // PQC encryption
├── dilithium.ts  // PQC signatures
├── halo2.ts      // ZK proof verification (if needed client-side)
└── blake3.ts     // Hashing
```

### Phase 4: Update SDK (1 hour)
```toml
# contracts/_sdk/Cargo.toml
[dependencies]
silica-models = { path = "../../silica-models" }
silica-utilities = { path = "../../silica-utilities", default-features = false }
```

## TypeScript Libraries to Consider

Based on PSY's approach and modern standards:

**PQC:**
- `@noble/post-quantum` - Kyber, Dilithium (recommended)
- `pqc-kyber` - Alternative Kyber implementation
- `crystals-dilithium-js` - Pure JS Dilithium

**ZK (Halo2):**
- `halo2-wasm` - If official bindings exist
- `snarkjs` - General ZK library (if you switch from Halo2)
- Or just verify proofs server-side

**Hashing:**
- `@noble/hashes` - BLAKE3, SHA2, SHA3
- `blakejs` - Alternative BLAKE implementation

## Final Validation

This approach means:

1. ✅ **contracts/_sdk** = Pure no_std, minimal dependencies
2. ✅ **silica-models** = Shared types, no_std compatible
3. ✅ **silica-utilities** = Just two features: `std` and `no_std`
4. ✅ **wallet** = Pure TypeScript crypto, no WASM crypto compilation
5. ✅ **miner/node** = Full Rust crypto with `std` feature

**Does this align with your vision?** This is actually a cleaner architecture than my previous suggestion since it eliminates the complexity of maintaining Rust crypto in WASM.Perfect! I can confirm that **you can absolutely rely on pure TypeScript/JavaScript for all your crypto needs**. Here's the comprehensive breakdown:

## ✅ Confirmed: TypeScript Crypto Libraries Are Production-Ready

### **1. Post-Quantum Cryptography (Kyber + Dilithium)**

**BEST CHOICE: `@noble/post-quantum`**
```bash
npm install @noble/post-quantum
```

**Features:**
- ✅ **ML-KEM (Kyber)** - 512, 768, 1024 variants
- ✅ **ML-DSA (Dilithium)** - 44, 65, 87 variants  
- ✅ **SLH-DSA (SPHINCS+)** - Hash-based signatures
- ✅ **Audited** - Peer reviewed and security audited
- ✅ **Pure TypeScript** - No WASM compilation needed
- ✅ **Tree-shakeable** - Optimized bundle sizes
- ✅ **NIST-compliant** - Passes all test vectors

**Usage Example:**
```typescript
// Kyber (Key Encapsulation)
import { ml_kem768 } from '@noble/post-quantum/ml-kem.js';
const seed = new Uint8Array(64); // Your random seed
const keypair = ml_kem768.keygen(seed);
const { cipherText, sharedSecret } = ml_kem768.encapsulate(keypair.publicKey);

// Dilithium (Digital Signatures)
import { ml_dsa65 } from '@noble/post-quantum/ml-dsa.js';
const keys = ml_dsa65.keygen(seed);
const signature = ml_dsa65.sign(keys.secretKey, message);
const valid = ml_dsa65.verify(keys.publicKey, message, signature);
```

**Alternative:** Node.js v22+ has native Kyber support in `crypto` module (no external package needed for backend).

---

### **2. Zero-Knowledge Proofs (Halo2)**

**STATUS: Requires WASM, but TypeScript wrappers available**

**BEST CHOICE: `halo2-browser` (Axiom)**
```bash
# Halo2 is Rust-based, but can run in browser via WASM
```

**Features:**
- ✅ **WASM bindings** for browser execution
- ✅ **TypeScript wrappers** for circuit writing
- ✅ **In-browser REPL** for development
- ✅ **Proof generation & verification** client-side

**Key Insight:** Halo2 doesn't have a pure TypeScript implementation, BUT:
- You can compile Rust Halo2 to WASM once
- Use TypeScript interfaces to interact with it
- Wallet uses WASM module for proof operations

**Alternative Approaches:**
1. **Server-side proof generation** - Generate proofs in your Rust backend, verify in wallet
2. **Use o1js (Mina Protocol)** - Pure TypeScript zk-SNARKs if you don't need Halo2 specifically
3. **SnarkJS** - Alternative ZK library with full TS support

**For Wallet Use Case:**
```typescript
// Axiom's halo2-browser approach
import { createCircuit, prove, verify } from 'halo2-browser';

const circuit = createCircuit(/* circuit definition */);
const proof = await prove(circuit, inputs);
const isValid = await verify(proof, publicInputs);
```

---

### **3. Hashing (BLAKE3, SHA-2, SHA-3)**

**BEST CHOICE: `@noble/hashes` or `blake3`**

```bash
npm install @noble/hashes
# or
npm install blake3
```

**@noble/hashes (Pure TypeScript):**
```typescript
import { blake3 } from '@noble/hashes/blake3';
import { sha256 } from '@noble/hashes/sha256';
import { sha3_256 } from '@noble/hashes/sha3';

const hash = blake3('Hello, Silica!');
const hexHash = Buffer.from(hash).toString('hex');
```

**blake3 NPM (WASM + Native):**
```typescript
import { hash, createHash } from 'blake3/browser';

// Simple hash
const result = hash('Hello, world!');

// Streaming hash
const hasher = createHash();
hasher.update(new TextEncoder().encode('Hello'));
hasher.update(new TextEncoder().encode(' World'));
const digest = hasher.digest();
```

**Features:**
- ✅ Both support **browser and Node.js**
- ✅ **Incremental hashing** (streaming)
- ✅ **Keyed hashing** and key derivation
- ✅ Extremely fast (BLAKE3 faster than SHA-2/SHA-3)

---

## 📊 Summary: TypeScript Crypto Stack for Silica Wallet

| **Crypto Function** | **NPM Package** | **Pure TS?** | **Status** |
|---------------------|----------------|--------------|------------|
| **Kyber (PQC KEM)** | `@noble/post-quantum` | ✅ Yes | Production-ready |
| **Dilithium (PQC Sig)** | `@noble/post-quantum` | ✅ Yes | Production-ready |
| **BLAKE3 Hashing** | `@noble/hashes` or `blake3` | ✅ Yes (or WASM) | Production-ready |
| **SHA-2/SHA-3** | `@noble/hashes` | ✅ Yes | Production-ready |
| **ED25519 Signatures** | `@noble/ed25519` | ✅ Yes | Production-ready |
| **Halo2 ZK Proofs** | `halo2-browser` (WASM) | ⚠️ WASM + TS wrappers | Usable in browser |

---

## ✅ Final Architecture Confirmation

### **silica-models** (Pure Data)
```toml
[dependencies]
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }
postcard = { version = "1.0", default-features = false, features = ["alloc"] }
```

### **silica-utilities** (Rust Backend)
```toml
[features]
default = ["std"]
std = [
    "halo2_proofs",      # ZK proof generation (server-side)
    "pqcrypto-dilithium", # PQC validation
    "pqcrypto-kyber",
    "chrono", "anyhow", "tracing"
]
no_std = []  # Minimal for contracts

[dependencies]
silica-models = { path = "../silica-models" }
blake3 = { version = "1.5", default-features = false }
ed25519-dalek = { version = "2.1", default-features = false }
# Heavy deps optional
halo2_proofs = { version = "0.3", optional = true }
pqcrypto-dilithium = { version = "0.5", optional = true }
```

### **wallet** (TypeScript)
```json
{
  "dependencies": {
    "@noble/post-quantum": "^0.2.0",
    "@noble/hashes": "^1.3.0",
    "@noble/curves": "^1.3.0",
    "blake3": "^2.1.0"
  },
  "devDependencies": {
    "halo2-browser": "latest"  // If client-side ZK needed
  }
}
```

### **contracts/_sdk** (no_std Rust)
```toml
[dependencies]
silica-models = { path = "../../silica-models" }
# No silica-utilities needed - crypto via FFI host functions
blake3 = { version = "1.5", default-features = false }
```

---

## 🎯 Answers to Your Question

### **Can we rely purely on TypeScript for wallet crypto?**

**YES!** ✅

- **Kyber/Dilithium** - `@noble/post-quantum` (audited, production-ready)
- **BLAKE3/SHA** - `@noble/hashes` or `blake3` (both excellent)
- **ED25519** - `@noble/ed25519` (if needed)
- **Halo2** - Use WASM with TS wrappers (or server-side generation)

### **What PSY likely uses:**
Based on the ecosystem, PSY wallet probably uses:
- `@noble/post-quantum` for PQC encryption
- `@noble/hashes` for hashing
- Either `halo2-browser` WASM or server-side ZK proof generation

### **Your Architecture is Perfect:**
1. ✅ **silica-models** - Shared types (no_std)
2. ✅ **silica-utilities** - Rust backend crypto (`std` feature)
3. ✅ **wallet** - Pure TypeScript crypto (no Rust WASM crypto needed)
4. ✅ **contracts/_sdk** - Minimal no_std (FFI for crypto)

**This is production-grade and maintainable!** 🚀