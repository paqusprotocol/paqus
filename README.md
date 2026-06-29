# Paqus Core

Core consensus primitives for the Paqus proof-of-work blockchain.

Paqus core is intentionally deterministic. It contains protocol data types,
canonical encoding, hashing, transaction and block validation, ledger state
transition, fork choice, reorg planning, rewards, supply limits, genesis
construction, and protocol invariants. Node networking, local storage, mempool
policy, mining loops, wallet UX, and runtime services live outside this crate.

## Status

The 0.2.x line freezes the current core consensus parameter set unless a future
protocol upgrade explicitly introduces an activation height. Runtime policy,
networking, storage, and wallet behavior remain outside this crate.

## Disclaimer

Paqus is an independent, non-profit blockchain research and development project.
It is not affiliated with, endorsed by, sponsored by, or officially connected to
any other project, company, foundation, token, protocol, or organization that may
use a similar name, mark, or terminology.

## Main Modules

- `codec`: canonical Borsh encoding, decode validation, and domain-separated SHA3-512 hashing.
- `crypto`: ML-DSA-87 keys, signatures, verification, and address derivation.
- `transaction`: transaction payloads, witnesses, signed transactions, and transaction validation.
- `block`: block headers, blocks, genesis allocations, coinbase, merkle roots, and block validation.
- `consensus`: proof-of-work validation, block reward schedule, difficulty retargeting, and version checks.
- `ledger`: account state, state roots, state proofs, fork choice, reorg planning, rewards, and invariants.
- `genesis`: canonical genesis block and ledger helpers.
- `checkpoint` and `snapshot`: checkpoint lookup and snapshot root helpers.
- `params`, `types`, and `version`: protocol constants, typed primitives, and version policy.
- `core`: convenience re-export surface for applications.

## Units And Supply

- Smallest unit: `paqus`.
- `1 XPQ = 100 paqus`.
- Total supply cap: `u32::MAX` paqus.
- Genesis premine: `95 paqus`.
- New subsidy minting must never push total supply above the cap.

## Current Consensus Parameters

```text
CHAIN_NAME = Paqus
CHAIN_ID = 747
COIN_NAME = XPQ
UNIT_NAME = paqus
PROTOCOL_STAGE = Mainnet
PROTOCOL_VERSION = 3
NETWORK_MAGIC = 58505101
BLOCK_TIME = 5 minutes
BLOCKS_PER_DAY = 288
DIFFICULTY_ADJUSTMENT_INTERVAL = 2016 blocks
CONFIRMATION_DEPTH = 10 blocks
BLOCK_REWARD_MATURITY = 120 blocks
FINALITY_DEPTH = 100 blocks
BLOCK_REWARD = 5_000 paqus
TAIL_EMISSION = 100 paqus
TAIL_EMISSION_START_HEIGHT = 420_480
SNAPSHOT_INTERVAL = 50_000 blocks
CHECKPOINT_INTERVAL = 50_000 blocks
ARGON2_POW_MEMORY = 512 MiB
ARGON2_POW_PARALLELISM = 2 lanes
```

Transaction outputs and miner fees become spendable at:

```text
block_height + CONFIRMATION_DEPTH
```

Block subsidy rewards become spendable at:

```text
block_height + BLOCK_REWARD_MATURITY
```

Fee price selection is node policy, not consensus. Core still accounts for the
fee carried by each transaction, checks that a sender can pay `amount + fee`,
and requires the block coinbase fee total to match the included transactions.
Minimum relay fee, market fee, and pending transaction expiry are enforced by
node mempool policy.

Forks may only reorganize non-final blocks. A block is final once the active tip
height is at least:

```text
block_height + FINALITY_DEPTH
```

## Canonical Encoding

Borsh is the canonical byte encoding for consensus objects. Use `codec` helpers
instead of calling serialization internals directly.

Supported byte helpers:

- `transaction_bytes`
- `signed_transaction_bytes`
- `block_header_bytes`
- `block_bytes`
- `state_root_bytes`

Decode helpers validate decoded objects before returning them:

- `decode_transaction`
- `decode_signed_transaction`
- `decode_block`

`canonical_deserialize` and its compatibility alias `canonical_decode` only
deserialize bytes. They do not imply domain or consensus validity.

`decode_signed_transaction` verifies the sender address and signature.
`decode_block` validates block-local rules such as transaction signatures,
merkle root, size, and timestamp bounds. It does not validate proof of work,
parent linkage, ledger state root, fork choice, or coinbase subsidy against a
ledger.

## Hash Domains

Core hashes are domain-separated. A hash from one domain must not be treated as
equivalent to another domain.

Hash domains include:

- `Transaction`
- `SignedTransaction`
- `BlockHeader`
- `GenesisAllocation`
- `Coinbase`
- `MerkleNode`
- `AccountState`
- `StateNode`
- `SnapshotRoot`
- `Raw`

The type system distinguishes `BlockHash`, `TransactionHash`, `MerkleHash`,
`StateRoot`, `PreviousHash`, and other hash wrappers.

## Transactions

A transaction is valid when:

- the transaction version is supported;
- amount is non-zero;
- fee fits in the transaction amount type;
- sender and recipient are different;
- sender account exists;
- sender nonce matches the transaction nonce;
- sender can spend `amount + fee` at the current block height.

Transaction timestamps are signed metadata. Core does not reject an otherwise
valid transaction only because the timestamp is old or ahead of local wall
clock. Relay age and future-time checks belong to node mempool policy.

`Witness` stores the public key and signature. `SignedTransaction` stores the
transaction payload plus its witness.

## Blocks

A block is valid when:

- block version is supported at its height;
- genesis blocks have no coinbase and no transactions;
- non-genesis blocks have coinbase and no genesis allocations;
- transaction count and serialized size are within limits;
- transaction fees and coinbase totals do not overflow `u32`;
- timestamp is not too far in the future;
- transaction signatures and sender addresses are valid;
- merkle root matches block contents;
- proof-of-work satisfies the expected difficulty.

Chain linkage requires:

- first block height is `0`;
- first block previous hash is zero;
- next block height is active tip height + 1;
- next block previous hash equals active tip hash;
- next block timestamp is not earlier than active tip timestamp.

Coinbase subsidy must equal:

```text
min(block_reward(height), MAX_UNIT_SUPPLY - supply_after_fees_are_credited)
```

Coinbase fees must equal the checked sum of all transaction fees in the block.

## State And Reorgs

State root is calculated from accounts ordered by `BTreeMap` key order. Account
leaf hashes and state parent hashes use separate hash domains.

State transition is atomic: if block application fails, ledger state and chain
tip must not change.

Non-genesis blocks must carry the canonical state root produced by their ledger
transition. Placeholder zero state roots are rejected by core ledger validation.

Fork choice selects the valid tip with the highest cumulative work. Ties are
resolved by the lowest block hash. Reorg planning is exposed through:

- `common_ancestor`
- `plan_reorg`

Runtime code is responsible for storing competing branches and applying reorg
plans to rebuild active state.

The core chain identity is intentionally singular. Test and development
environments should use runtime options such as local storage, temporary peers,
or test difficulty controls rather than alternative core consensus parameter
profiles.

## Genesis

Genesis is deterministic. The canonical genesis hash is:

```text
32ac01d654c1fe57d12506456bb7237f4baf214a3573b11fcdb128974d95864f4031856cae53a859c5adc5d2880670739571057b71b2575642e5cce6d16efe1d
```

## Invariants

Core invariants include:

- total supply must never exceed `u32::MAX`;
- account map key must match account address;
- sum of account credits must equal account balance;
- failed block application must not mutate state;
- state root must be deterministic for the same state;
- domain-separated hashes must not be mixed.

## Example

```rust
use paqus::core::{Block, Hash, Height, Nonce, SignedTransaction, block_header_hash};

fn inspect(block: &Block, tx: &SignedTransaction) {
    let block_hash = block_header_hash(&block.header);
    let tx_hash = tx.hash();

    println!("block header hash: {:?}", block_hash);
    println!("transaction hash: {:?}", tx_hash);
}

let _genesis_like = Block::new(
    Height(0),
    Hash([0; 64]),
    paqus::core::Address([0; 20]),
    1_700_000_000,
    Nonce(0),
    vec![],
);
```

## Changelog

### 0.2.0 - Mainnet

- Tightened block decoding and validation so signed transactions inside blocks must verify their sender address and signature.
- Clarified canonical deserialization semantics and added `canonical_deserialize`.
- Required non-genesis blocks to carry the canonical state root in core ledger validation.
- Clamped candidate and ledger coinbase subsidy through `mintable_subsidy`, including zero subsidy after mined supply is exhausted.
- Restricted raw unsigned transaction application helpers to crate/test internals.
- Prevented standalone signed transaction application from bypassing locked credit maturity.
- Added explicit trusted account-state import naming with `Account::trusted_with_nonce`.
- Added difficulty-range checks to fork choice insertion.
- Simplified chain parameters to one Paqus protocol identity instead of separate mainnet/testnet/devnet profiles.
- Matched checkpoint cadence to snapshot cadence at 50,000 blocks.
- Set Argon2 proof-of-work parameters to 512 MiB memory, time cost 1, parallelism 2, output 32 bytes.

### 0.1.9 - Mainnet

- Moved transaction age/future-time filtering out of core consensus and into node mempool policy.
- Kept transaction fees as sender-chosen transaction fields while removing core fee-price policy.
- Split coinbase accounting so miner fees mature at confirmation depth and block subsidy matures at reward maturity.
- Raised block future-time tolerance to two minutes and kept it as block consensus validation.

### 0.1.7 - Devnet

- Cleaned up snapshot-height detection to satisfy current Clippy lints.
- Split transaction output confirmation from hard chain finality.
- Set confirmation depth to 2 blocks, block reward maturity to 15 blocks, and hard finality depth to 50 blocks.
- Reject reorg plans that would replace finalized blocks.

### 0.1.6 - Testnet

- Updated protocol stage metadata from devnet to testnet.
- Changed the base unit name to `paqus`.
- Updated testnet economics: block reward, reward maturity, finality depth, minimum fee, difficulty retarget interval, and future timestamp limit.
- Added mined-supply tracking helpers for capped subsidy calculation.
- Simplified transaction fee validation to require the fixed minimum fee.

### 0.1.5 - Devnet

- Changed the default P2P bind address to `[::]:30333`.
- Kept the default RPC bind address at `127.0.0.1:9933`.
- Added parameter tests for default RPC and P2P socket addresses.

### 0.1.4 - Devnet

- Changed proof-of-work difficulty semantics from leading zero bytes to leading zero bits.
- Changed difficulty retargeting to use a 10-block adjustment window.
- Reduced devnet transaction finality depth to 1 block.
- Reduced devnet mining reward maturity to 20 blocks.
- Added automatic receiver account creation for incoming transfers.
- Added mempool byte accounting and transaction replacement policy.
- Added transaction hash and address transaction history indexing.
- Added network protocol handshake messages and version compatibility checks.

### 0.1.3 - Devnet

- Fixed canonical genesis configuration.
- Preserved canonical genesis hash validation during node initialization.
- Cleaned up stored-data integrity error handling.

### 0.1.2 - Devnet

- Added canonical default node ports and bind addresses.
- Updated devnet target block time from 2 minutes to 5 minutes.
- Clarified that runtime services live outside the core crate.
- Removed the experimental async networking crate from the core direction.

### 0.1.1 - Devnet

- Updated devnet consensus and mining behavior.
- Improved mempool validation against ledger state.
- Added mining attempt budgeting and candidate block handling improvements.
- Expanded node balance reporting.
- Added maturity tracking for mining rewards and transaction credits.
- Added storage and node tests around devnet behavior.

### 0.1.0 - Devnet

- Initial Paqus core crate.
- Added block and transaction primitives.
- Added SHA3-512 hashing and Argon2 proof-of-work hashing.
- Added ML-DSA-87 wallet keys, signatures, and transaction signing.
- Added address derivation, ledger state, genesis helpers, consensus validation, fork choice, reorg handling, and state proofs.

## Community

Join the Paqus Matrix room for discussion, questions, and protocol development:
https://matrix.to/#/#paqus:matrix.org
