<div align="center">
  <img src="assets/hido_logo.png" width="150" alt="HIDO Logo">
  <h1>HIDO - Hierarchical Intent-Driven Orchestration</h1>
</div>

A decentralized agent framework providing secure identity management, semantic intent communication, and immutable audit trails.

**Production Ready** | **Current Version: 1.0.0**

## Features

### UAIL - Universal Agent Identity Layer
- DID (Decentralized Identifier) generation and management
- Verifiable Credentials with issuance and revocation
- Ed25519 cryptography with SHA3-256 hashing

### ICC - Intent Communication Channel
- Semantic Intent structure with domain taxonomy
- Protocol handlers for message exchange
- LZ4 compression with dictionary encoding
- Capability-based routing with load balancing

### BAL - Blockchain Audit Layer
- Content-addressed action blocks
- Tamper-evident blockchain
- Chain integrity verification

### Intelligence Layer
- **GNN**: Graph Neural Networks for agent reasoning
- **Consensus**: pBFT-based decision making
- **Federated Learning**: Privacy-preserving model training

### Production & Enterprise
- **Kubernetes**: Native deployment and scaling
- **Multi-Region**: Active-active regions with failover
- **SLA**: Contract monitoring and penalty enforcement
- **Plugins**: Hot-swappable capability system
- **Compliance**: GDPR, SOC2, HIPAA rule engines
- **Flexible Audit**: Pluggable backends (Blockchain, PostgreSQL, Kafka+S3, Hybrid)

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
hido = "1.0.0"
```

## Project Structure

```
src/
├── lib.rs          # Library entry point
├── core/           # Common types and error handling
├── uail/           # Identity Layer
├── icc/            # Communication Layer
├── bal/            # Blockchain Audit Layer
├── audit/          # Flexible Audit Interface (Phase 5)
├── gnn/            # AI Reasoning
├── consensus/      # Agreement Protocols
├── k8s/            # Orchestration
├── monitoring/     # Observability
└── compliance/     # Regulatory Rules
```

## Community & Contributing

We welcome contributions from the community! Whether it's fixing bugs, adding new agents, or improving documentation, your help is appreciated.

### How to Contribute
1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Run tests ensuring everything passes (`cargo test`)
4. Commit your changes (`git commit -m 'Add amazing feature'`)
5. Push to the branch (`git push origin feature/amazing-feature`)
6. Open a Pull Request

## License

MIT
