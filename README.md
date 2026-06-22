# paqus

Core primitives for the Paqus proof-of-work blockchain.

## Disclaimer

Paqus is an independent, non-profit blockchain research and development project.
It is not affiliated with, endorsed by, sponsored by, or officially connected to
any other project, company, foundation, token, protocol, or organization that may
use a similar name, mark, or terminology. Any resemblance in naming is
coincidental and should not be interpreted as a partnership, endorsement, or
shared ownership.

This crate contains deterministic blockchain logic and protocol primitives:

- canonical Borsh encoding and domain-separated SHA3-512 hashing
- typed block, transaction, state, and Merkle hashes
- Argon2 proof-of-work hashing
- ML-DSA-87 transaction signatures
- account state transition and transaction validation
- block validation, state root checks, fork choice, and reorg planning
- reward, fee, supply cap, and maturity rules
- genesis construction and core invariants

The crate intentionally stays focused on core logic. Node networking, local
storage, transaction pool policy, wallet UX, and runtime services belong outside
this layer and can call these primitives.

Transactions support simple fee tiers: slow 1 unit, normal 2 units, fast 3
units, and aggressive 5 units. Transaction outputs become spendable after the
core finality depth, while block subsidy rewards mature separately.

## Status

The core API and consensus parameters are still experimental and may change
before a stable protocol release.

## Community

Join the Paqus Matrix room for discussion, questions, and protocol development:
https://matrix.to/#/#paqus:matrix.org

## Example

```rust
use paqus::core::{Block, SignedTransaction};
use paqus::codec::{hash_block, hash_signed_transaction};

fn inspect(block: &Block, tx: &SignedTransaction) {
    let block_hash = hash_block(block);
    let tx_hash = hash_signed_transaction(tx);

    println!("block hash: {:?}", block_hash);
    println!("tx hash: {:?}", tx_hash);
}
```
