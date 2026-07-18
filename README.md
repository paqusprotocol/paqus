# Paqus Core

Core consensus primitives for the Paqus proof-of-work blockchain.

Canonical cross-implementation fixtures are published in
[`PROTOCOL_VECTORS.md`](./PROTOCOL_VECTORS.md).

## Decoder Fuzzing

Consensus decoders reject oversized input before Borsh deserialization and
reject malformed length prefixes or trailing bytes. Three isolated libFuzzer
targets cover transaction-family decoders, unified protocol envelopes/events,
and blocks:

```bash
cargo install cargo-fuzz
cd core
cargo fuzz run decode
cargo fuzz run decode_protocol
cargo fuzz run decode_block
```

The fuzz crate is an independent workspace under `core/fuzz`, so libFuzzer is
never linked into production builds. Deterministic length-bomb, trailing-byte,
and oversized-input regressions also run under the normal `cargo test` suite.

## Consensus Benchmarks

Criterion baselines cover canonical block encoding and validated decoding,
bounded rejection of malformed, trailing, and oversized decoder inputs,
transaction `txid`/`wtxid` identity and ML-DSA-87 operations,
eCash bearer-file and deposit-authorization operations, unified
transaction-envelope costs per family,
protocol-event encoding and lookup, signed block validation at
several sizes, payload and witness Merkle commitments, snapshot commitment and
finality rules, state-root and account-proof
operations for up to 1,000 accounts, a separate 10,000-account large-state
group, multi-transaction ledger transitions, and one- or two-block
competing-branch reorg planning and execution. A mixed-family block covers
transfer and eCash encoding,
validation, application, and rollback. Late-failure scenarios measure rejection
costs for an invalid final signature, invalid final nonce, and incorrect
post-execution state root.

Fork-choice scaling measures linear-chain insertion and tip-to-genesis ancestor
traversal at 100 and 1,000 blocks.

Fast and medium scenarios use 20 samples with a five-second measurement window.
Large-state and end-to-end ledger scenarios use 10 samples with a ten-second
window and a longer warm-up, keeping expensive measurements stable without
inflating every microbenchmark.

```bash
cargo bench -p paqus --bench consensus
```

Use smoke mode to compile and execute every scenario once without collecting a
statistical baseline:

```bash
cargo bench -p paqus --bench consensus -- --test
```

The SHA3-512 proof-of-work hash has a dedicated benchmark target:

```bash
cargo bench -p paqus --bench pow
cargo bench -p paqus --bench pow -- --test
```

Criterion writes local reports under `target/criterion`. Compare measurements
on the same machine, toolchain, CPU governor, and build configuration; benchmark
numbers from different hosts are not directly comparable. CI runs smoke mode to
prevent benchmark rot but does not enforce timing thresholds on shared runners.

Use the workspace helper to save and compare a named local baseline. An optional
third argument filters benchmark names, which is useful when iterating on one
subsystem:

```bash
./core/scripts/benchmark-baseline.sh save before-change
./core/scripts/benchmark-baseline.sh compare before-change
./core/scripts/benchmark-baseline.sh compare-lenient before-change protocol_envelope
./core/scripts/benchmark-baseline.sh list
```

Baselines remain local under `target/criterion`; do not compare results produced
on different machines or under different power and CPU-governor settings.

Paqus core is intentionally deterministic. It contains protocol data types,
canonical encoding, hashing, transaction and block validation, ledger state
transition, fork choice, reorg planning, rewards, supply limits, genesis
construction, and protocol invariants. Node networking, local storage, mempool
policy, mining loops, wallet UX, and runtime services live outside this crate.

## Status

The current chain identity uses protocol version 1, SHA3-512 proof of work, and
per-block ASERT difficulty adjustment. Any change to these rules, their
canonical encoding, or the frozen genesis identity is a consensus-breaking
change and requires an explicit protocol upgrade. Runtime policy, networking,
storage, and wallet behavior remain outside this crate.

## Disclaimer

Paqus is an independent, non-profit blockchain research and development project.
It is not affiliated with, endorsed by, sponsored by, or officially connected to
any other project, company, foundation, token, protocol, or organization that may
use a similar name, mark, or terminology.

## Main Modules

- `codec`: canonical Borsh encoding, decode validation, and domain-separated SHA3-256 hashing.
- `crypto`: ML-DSA-87 keys, signatures, verification, Bech32 addresses, and hashing.
- `ecash`: canonical deposit/withdraw metadata, cash denominations, and amount formatting helpers.
- `transaction`: transaction payloads, witnesses, signed transactions, and transaction validation.
- `block`: block headers, blocks, coinbase, merkle roots, and block validation.
- `consensus`: SHA3-512 proof-of-work validation, supply definitions, block reward schedule, and per-block ASERT difficulty adjustment.
- `ledger`: account state, state roots, state proofs, fork choice, reorg planning, rewards, and invariants.
- `genesis`: canonical genesis block and empty-ledger helpers.
- `snapshot`: snapshot root helpers.

The older broad modules `params`, `types`, `version`, `checkpoint`, and `core`
have been removed. Constants and wrapper types now live beside the logic that
uses them:

- address types and constants live in `crypto::address`;
- hash types and hash functions live in `crypto::hash`;
- key, signature, and key-size constants live in `crypto::keygen`;
- amount, balance, fee, reward, and supply definitions live in `consensus::supply`;
- block height and nonce types live in `block`;
- account nonce lives in `transaction`;
- maturity and finality depths live in `ledger`;
- chain identity lives in `genesis::ChainParams`.

## Units And Supply

- Smallest unit: `paqus`.
- `1 XPQ = 100_000 paqus`.
- Amounts, balances, and fees use `u64` units.
- Genesis has no premine allocation.
- The scheduled block subsidy is `10 XPQ`, changing to a perpetual `0.2 XPQ`
  tail emission after five years.

## eCash Metadata

The `ecash` module provides deterministic metadata for deposits and withdrawals.
Supported whole-XPQ denominations are `1`, `2`, `5`, `10`, `20`, `50`, and
`100`. `format_cash_coins` converts an `Amount` to the fewest supported coins,
stored as canonical descending denomination/count pairs. Zero and fractional-XPQ
amounts are rejected.

`EcashMetadata::deposit` and `EcashMetadata::withdraw` construct and validate
metadata while `EcashMetadata::amount` reconstructs its value in paqus. The
metadata is intentionally independent of the version-1 transaction layout; a
future consensus upgrade can embed it without changing existing transaction
hashes implicitly.

`WithdrawCashMetadata::plan_automatic` cashes only the whole-XPQ portion of a
requested amount. For example, a request for `1_000.1 XPQ` produces ten
`100 XPQ` outputs and returns `0.1 XPQ` as an on-chain remainder. An amount below
`1 XPQ` has no cashable portion and is rejected. The wallet creates one unique
commitment per planned output before calling `from_automatic_plan`.

`state::OffchainCoinState` is a separate local state for the eCash lifecycle.
Applying withdraw metadata creates one `OffchainCashCoin` per physical coin and
assigns a deterministic 32-byte `CashCoinId`. `WithdrawCashMetadata` contains
the consensus-visible output index, denomination, and wallet commitment for each
coin. The ID is a domain-separated SHA3-256 hash of the withdraw transaction
hash and its canonical output. Applying deposit metadata requires those
IDs, verifies their denominations, and changes their status from `Issued` to
`Redeemed`. Redeemed IDs remain recorded, preventing the same coin from being
deposited twice. This state is Borsh/Serde serializable but is not part of the
account state root or consensus transition.

eCash uses finality maturity rather than normal transfer confirmation maturity.
Withdraw outputs remain `PendingIssue` for `ECASH_WITHDRAW_MATURITY` (currently
100 blocks) and cannot be deposited or exported as active cash before becoming
`Issued`. A deposit changes an issued coin to `PendingRedeem`; after
`ECASH_DEPOSIT_MATURITY` (also 100 blocks) it becomes `Redeemed`. Deposit account
credits use the same 100-block spendable height. Pending issue and redeem states
remain included in `economic_supply`.

Block-scoped transitions use `EcashBlockJournal`. Withdraw journals record new
coin IDs, while deposit journals retain complete prior coin records.
`rollback_block` removes outputs created by a disconnected withdraw, restores
coins from a disconnected deposit to their earlier status (normally `Issued`),
and restores the deterministic event sequence. This makes the same transaction
safe to apply again on a competing canonical branch. `Ledger::apply_signed_ecash_transaction_in_block`
binds journal entries to the containing block hash. `EcashAccountJournal`
snapshots each touched signer and recipient once per block.
`Ledger::rollback_ecash_block` atomically restores balances, credit maturity
entries, nonces, newly created recipient accounts, and the associated offchain
coin journal.

`CashCoinId::file_name` formats a portable display name as
`<denomination>+<first 9 hash hex characters>.XPQ`, for example
`50+51D7AB830.XPQ`. The short hash is only a file label; validation always uses
the complete 32-byte ID stored in the coin data.

Wallet bearer files use `CashCoinFile`. The file stores the withdraw transaction
hash, output index, denomination, and a 32-byte opening secret. Withdraw outputs
publish only `cash_coin_commitment(opening_secret)`. `DepositCashMetadata`
reveals the secret when redeeming; `OffchainCoinState::apply_deposit_proof`
reconstructs the output and coin ID, verifies it against issued state, then marks
all inputs redeemed atomically. Anyone who obtains an unredeemed `.XPQ` file can
redeem it, so wallets must protect these files like physical cash.

The `.XPQ` wire format is defined by `encode_cash_coin_file` and
`decode_cash_coin_file`: `XPQCASH1` magic bytes, a little-endian payload length,
canonical Borsh `CashCoinFile` payload, and a 32-byte domain-separated checksum.
Files are limited to 1 KiB. Wallets and nodes must use the strict decoder rather
than trusting the extension or file name; malformed, truncated, oversized, and
checksum-mismatched files are rejected before deposit proof validation.

The explicit withdraw metadata types are kept separate from transaction version
1 so transaction-family boundaries remain explicit. A protocol-version activation
is still required before these outputs can be embedded in blocks as consensus
transaction metadata.

`transaction::EcashTransaction` defines the version-1 eCash transaction
envelope alongside version-1 transfer bytes. Its kinds are
`WithdrawCash`, carrying an amount and explicit outputs, and `DepositCash`,
carrying a recipient and bearer proofs. `SignedEcashTransaction` uses a separate
`PAQUSCORE_ECASH_TX_V1` signature domain and validates the signer, signature,
size, metadata, withdraw output total, and deposit fee bounds. Network activation
policy is still required before nodes accept blocks on a live chain.

`Ledger::apply_signed_ecash_transaction` executes version-1 eCash transitions
atomically. Withdraw debits `cash amount + fee`, increments the signer nonce,
and issues every explicit output under the transaction hash. Deposit verifies
and redeems all bearer proofs, increments the signer nonce, and credits
`deposit amount - fee` to the recipient with normal confirmation maturity. A
failure rolls back both account and offchain coin state. `economic_supply`
counts account balances plus issued, unredeemed cash; `total_supply` remains the
account-only view.

`block::Block` is the SegWit version-1 block envelope. Its canonical wire format
serializes transfer and eCash payload sections first, followed by two parallel
witness sections. Decode rejects any
payload/witness length mismatch. The header commits to the payload `merkle_root`
and the separate `witness_root`, so two valid witness variants keep the same
`txid` but have different `wtxid`, witness root, and block hash.

`stripped_size`, `witness_size`, and `weight` expose consensus accounting with a
witness scale factor of four. The physical 5 MiB block limit remains enforced
alongside the weight limit. `block_bytes` and `decode_block` are the only
canonical wire codec and always include both sections.

Signed transactions and unified envelopes also expose `virtual_size()`, rounded
up from weight. Node relay fees and miner ordering use virtual size, while
in-memory pool limits continue to use physical serialized bytes.

`Ledger::state_root_after_block` stages every transaction family and combined
coinbase effects for mining. `apply_block` validates and atomically executes the
complete version-1 block, while `rollback_block` restores the complete protocol
state snapshot and eCash journals. `Chain` stores all blocks in one map.

## Transaction Validity Windows

Every transaction family carries a signed `ValidityWindow` with inclusive
`valid_from` and `valid_until` block heights. Transactions are rejected before
the lower bound and after the upper bound; both boundary heights are valid. The
default constructors use `ValidityWindow::UNBOUNDED` for compatibility, while
`with_validity_window` attaches explicit bounds before signing.

Canonical decoding validates the shape of the window without rejecting a
not-yet-valid transaction. Block validation and ledger execution enforce it at
the candidate block height. The full-node mempool may retain future-valid
transactions, but miner selection excludes transactions outside the candidate
height's window.

## Protocol Events

Every successfully applied block produces canonical version-1 `ProtocolEvent`
receipts in the same order as its state transitions. Events cover transfers,
eCash,
genesis allocations, and coinbase payments. Each event is bound to the block
height and hash, optional transaction hash, and a block-local event index; its
domain-separated `EventId` is deterministic.

Events are derived receipts and are deliberately excluded from
`protocol_state_root`. A failed block emits nothing, and `rollback_block`
removes the disconnected block's events. Consumers can use
`Ledger::events_for_block`, `Ledger::event`, and the validated codec helpers to
build an external persistent index.

## Addresses

Wallet addresses use standard Bech32 with the human-readable prefix `PX`.
Addresses encode 20 bytes and are formatted uppercase. The Bech32 checksum is
the standard 6-character checksum.

## Current Consensus Parameters

```text
CHAIN_NAME = Paqus
CHAIN_ID = 747
COIN_NAME = XPQ
UNIT_NAME = paqus
PROTOCOL_STAGE = Mainnet
PROTOCOL_VERSION = 1
NETWORK_MAGIC = 58505101
BLOCK_TIME = 1 minute
BLOCKS_PER_DAY = 1_440
DIFFICULTY_ADJUSTMENT_INTERVAL = 1 block (ASERT)
ASERT_HALF_LIFE = 172800 seconds (2 days)
DIFFICULTY_START = 1 leading-zero bit
CONFIRMATION_DEPTH = 10 blocks
BLOCK_REWARD_MATURITY = 120 blocks
FINALITY_DEPTH = 100 blocks
DECIMALS = 5
1 XPQ = 100_000 paqus
BLOCK_REWARD = 1_000_000 paqus (10 XPQ)
TAIL_EMISSION = 20_000 paqus (0.2 XPQ)
TAIL_EMISSION_START_HEIGHT = 2_628_000
SNAPSHOT_INTERVAL = 50_000 blocks
POW_ALGORITHM = SHA3-512
DIFFICULTY_ALGORITHM = ASERT
POW_DIGEST_SIZE = 64 bytes (512 bits)
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

Difficulty has a minimum of `1` for normal consensus validation. Core does not
define a protocol maximum difficulty. If a requested difficulty exceeds what a
64-byte proof-of-work hash can satisfy, proof-of-work validation simply fails as
insufficient work rather than rejecting the difficulty value as out of range.
Proof of work is SHA3-512 over the canonical Borsh-encoded block header, and
difficulty counts required leading zero bits in that 512-bit digest.
Difficulty is recalculated for every block with ASERT, anchored to the first
mined block after the frozen genesis. The target schedule uses the one-minute
block interval and a two-day half-life. The first mined block uses
`DIFFICULTY_START`; genesis is a trusted frozen anchor and is not competitively
mined.

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

The type system distinguishes `BlockHash`, `TransactionHash`,
`WitnessTransactionHash`, `MerkleHash`, `WitnessMerkleHash`, `StateRoot`,
`PreviousHash`, and other hash wrappers.

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

Application layers can inspect witness identities uniformly through
`SignedProtocolTransaction::witness_public_keys()` and
`SignedProtocolTransaction::witness_addresses()`. Single-signature envelopes
contain one key. These accessors do not verify signatures, so decoded
transactions must still pass normal validation before an application trusts the
returned identity.

Every signed family exposes two identities: `hash()`/`txid()` identifies the
signed payload independently of its witness, while `wtxid()` commits to the
unified family tag and complete witness. Mempool and state-transition duplicate
logic may use `txid`; block witness integrity and byte-level identity use
`wtxid`.

## Blocks

A block is valid when:

- genesis blocks have no coinbase and no transactions;
- non-genesis blocks have coinbase;
- transaction count and serialized size are within limits;
- transaction fees and coinbase totals do not overflow `u64`;
- timestamp is not too far in the future;
- transaction signatures and sender addresses are valid;
- merkle root matches block contents;
- proof-of-work satisfies the expected difficulty.

An ordinary non-genesis block may contain zero user transactions. It still
contains a coinbase transaction and must satisfy the same linkage, state-root,
difficulty, timestamp, and reward rules. Mining therefore does not depend on a
non-empty mempool.

Chain linkage requires:

- first block height is `0`;
- first block previous hash is zero;
- next block height is active tip height + 1;
- next block previous hash equals active tip hash;
- next block timestamp is not earlier than active tip timestamp.

Coinbase subsidy must equal:

```text
block_reward(height)
```

Coinbase fees must equal the checked sum of all transaction fees in the block.

## State And Reorgs

The canonical account state root is a 160-level sparse Merkle root keyed by the
address bits. Account leaf hashes and state parent hashes use separate hash
domains.

`Ledger` keeps this tree as derived, incremental state. Account changes update
only the affected leaf and its ancestor path, while `state_root()` is a cached
read. Ledger staging and rollback snapshots share it copy-on-write, and loading a
persisted account snapshot rebuilds the cache once. The cache is not serialized
or included as a second consensus representation; its root is required to equal
a full canonical rebuild by ledger invariant checks.

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

Canonical encoding is frozen under the profile `paqus-borsh-le`. Consensus data
remains canonical little-endian Borsh and
must only be encoded through `codec.rs`. Field order, enum order, integer width,
collection layout, hash domains, transaction family tags, and witness section
layout are part of this frozen format.

Genesis is deterministic. The frozen genesis hash is:

```text
2f636a6ecec93619e436f66f42ba977f89a3126421e9db149819576a51e320b8
```

Genesis creates no initial account allocations and no premine supply.

The full node validates the computed genesis block against this literal identity
before opening or initializing chain state. Canonical compatibility vectors in
`PROTOCOL_VECTORS.md` are executable regression fixtures. A change to any frozen
byte or hash must be treated as an explicit protocol and chain-identity change;
the frozen expected values must never be rewritten accidentally.

## Invariants

Core invariants include:

- subsidy selection must follow the 10 XPQ schedule and 0.2 XPQ tail emission;
- account map key must match account address;
- sum of account credits must equal account balance;
- failed block application must not mutate state;
- state root must be deterministic for the same state;
- domain-separated hashes must not be mixed.

## Example

```rust
use paqus::block::{Block, Height, Nonce};
use paqus::codec::block_header_hash;
use paqus::crypto::{Address, Hash};
use paqus::transaction::SignedTransaction;

fn inspect(block: &Block, tx: &SignedTransaction) {
    let block_hash = block_header_hash(&block.header);
    let tx_hash = tx.hash();

    println!("block header hash: {:?}", block_hash);
    println!("transaction hash: {:?}", tx_hash);
}

let _genesis_like = Block::new(
    Height(0),
    Hash([0; paqus::crypto::HASH_SIZE]),
    Address([0; 20]),
    1_700_000_000,
    Nonce(0),
    vec![],
);
```

## Changelog

### Current protocol version 1

- Replaced Argon2id proof of work with a 64-byte SHA3-512 digest over the
  canonical Borsh block header.
- Expanded proof-of-work and cumulative-work handling to the 512-bit hash
  domain.
- Added per-block ASERT adjustment with a one-minute target, two-day half-life,
  and an anchor at the first mined block after frozen genesis.
- Set initial difficulty to 1 leading-zero bit.
- Explicitly allow valid coinbase-only blocks, so mining continues when the
  mempool is empty.
- Kept chain protocol version 1; the full-node runtime storage format was reset
  separately to storage version 1.
- Set the current monetary units to five decimals, 10 XPQ block subsidy, and a
  perpetual 0.2 XPQ tail emission beginning at height 2,628,000.

- Added an incremental sparse account-state tree with cached root/proof reads,
  canonical full-rebuild invariant checks, storage-load reconstruction, and
  rollback/reorg cache restoration.
- Added mixed-family atomic reorg coverage for transfer, eCash, coinbase,
  events, and protocol state roots.
- Fixed eCash withdrawal rollback to remove disconnected outputs and restore its
  event sequence, allowing deterministic reapplication after a reorg.

### 0.2.1 - Mainnet

- Reorganized the public API around ownership boundaries and removed the broad
  `params`, `types`, `version`, `checkpoint`, and `core` modules.
- Moved hash wrappers and hash helpers into `crypto::hash`.
- Moved coin units, amount/balance/fee wrappers, and reward
  calculation into `consensus::supply`.
- Moved key, signature, and key-size definitions into `crypto::keygen`.
- Changed wallet addresses to uppercase Bech32 with HRP `PX`, 20-byte payloads,
  and standard 6-character checksums.
- Changed amount, balance, fee, and supply storage from `u32` to `u64`.
- Consolidated XPQ precision and emission parameters in `consensus::supply`.
- Removed genesis premine allocations.
- Removed protocol version constants from core.
- Removed checkpoint validation from core.
- Removed the protocol maximum difficulty bound. Difficulty still has minimum
  `1`; unsatisfiable high difficulty values fail proof-of-work naturally.
- Added `zeroize` for secret-key material and static protocol size assertions.

### 0.2.0 - Mainnet

- Tightened block decoding and validation so signed transactions inside blocks must verify their sender address and signature.
- Clarified canonical deserialization semantics and added `canonical_deserialize`.
- Required non-genesis blocks to carry the canonical state root in core ledger validation.
- Restricted raw unsigned transaction application helpers to crate/test internals.
- Prevented standalone signed transaction application from bypassing locked credit maturity.
- Added explicit trusted account-state import naming with `Account::trusted_with_nonce`.
- Added difficulty-range checks to fork choice insertion.
- Simplified chain parameters to one Paqus protocol identity instead of separate mainnet/testnet/devnet profiles.

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
- Added domain-separated SHA3 hashing foundations.
- Added ML-DSA-87 wallet keys, signatures, and transaction signing.
- Added address derivation, ledger state, genesis helpers, consensus validation, fork choice, reorg handling, and state proofs.

## Community

Join the Paqus Matrix room for discussion, questions, and protocol development:
https://matrix.to/#/#paqus:matrix.org
