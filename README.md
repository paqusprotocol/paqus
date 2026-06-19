# paqus

Devnet core crate for the Paqus proof-of-work blockchain.

This crate contains the core building blocks used by a Paqus full node:

- SHA3-512 block and transaction hashing
- Argon2 proof-of-work hashing
- ML-DSA-87 transaction signatures
- ledger state, block validation, fork choice, and reorg handling
- mempool, mining helpers, sled storage, and basic node/network primitives

Devnet transactions support simple fee tiers: slow 1 unit, normal 2 units, fast
3 units, and aggressive 5 units. Mining rewards and received transaction credits
are tracked with explicit spendable heights, so account views can separate
spendable balance from unspendable confirmed funds.

## Status

This is a devnet release. APIs, consensus parameters, storage format, and network
messages can still change before a stable mainnet release.

## Community

Join the Paqus Matrix room for discussion, questions, and devnet coordination:
https://matrix.to/#/#paqus:matrix.org

## Example

```rust
use paqus::consensus::Consensus;
use paqus::genesis::{GenesisConfig, GENESIS_PREMINE_ADDRESS};
use paqus::node::Node;
use paqus::types::Address;

let node = Node::init_or_load(
    "./paqus-devnet-db",
    GenesisConfig {
        premine_address: GENESIS_PREMINE_ADDRESS,
        miner_address: Address([9; 20]),
        timestamp: 1_700_000_000,
    },
    Consensus::with_default_config(),
)?;

println!("tip: {:?}", node.tip_height());
# Ok::<(), Box<dyn std::error::Error>>(())
```
