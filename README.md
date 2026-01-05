# HIDO - Hierarchical Intent-Driven Orchestration

A decentralized agent framework built in Rust providing secure identity management, semantic intent communication, and immutable audit trails.

## Features

**UAIL - Universal Agent Identity Layer**
- DID (Decentralized Identifier) generation and management
- Verifiable Credentials with issuance and revocation
- Ed25519 cryptography with SHA3-256 hashing

**ICC - Intent Communication Channel**
- Semantic Intent structure with domain taxonomy
- Protocol handlers for message exchange
- LZ4 compression with dictionary encoding
- Capability-based routing with load balancing

**BAL - Blockchain Audit Layer**
- Content-addressed action blocks
- Tamper-evident blockchain
- Chain integrity verification

## Quick Start

```rust
use hido::uail::{DIDManager, DIDConfig};

#[tokio::main]
async fn main() {
    // Create a new agent identity
    let mut manager = DIDManager::new(DIDConfig::default());
    let did = manager.generate().await.unwrap();
    println!("Agent DID: {}", did.id);
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hido = "0.1.0"
```

## Testing

```bash
cargo test
```

## Project Structure

```
src/
├── lib.rs          # Library entry point
├── core/           # Common types and error handling
│   ├── error.rs
│   ├── types.rs
│   └── mod.rs
├── uail/           # Universal Agent Identity Layer
│   ├── did.rs      # DID implementation
│   ├── credential.rs
│   ├── crypto.rs
│   └── mod.rs
├── icc/            # Intent Communication Channel
│   ├── intent.rs   # Semantic intents
│   ├── protocol.rs
│   ├── compression.rs
│   ├── router.rs
│   └── mod.rs
└── bal/            # Blockchain Audit Layer
    ├── block.rs    # Action blocks
    ├── chain.rs    # Blockchain
    └── mod.rs
```

## Roadmap

- [x] Phase 1: Foundation (UAIL, ICC, BAL)
- [ ] Phase 2: Intelligence (GNN, Consensus, Federated Learning)
- [ ] Phase 3: Production (Kubernetes, Multi-region)
- [ ] Phase 4: Enterprise (SLA, Plugins, Compliance)

## License

MIT
