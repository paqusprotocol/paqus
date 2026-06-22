# Paqus Core Spec

This document defines the consensus-facing core rules implemented by the
`paqus` crate.

## Units And Supply

- The smallest unit is `nyx`.
- `1 XPQ = 100 nyx`.
- Total supply is capped at `u32::MAX` nyx.
- Genesis premine is `95 nyx`.
- New subsidy minting must never push total supply above `u32::MAX`.

## Canonical Encoding

- Borsh is the canonical byte encoding for core objects.
- `codec.rs` is the only public boundary for canonical encode/decode helpers.
- Supported canonical byte helpers:
  - `transaction_bytes`
  - `signed_transaction_bytes`
  - `block_header_bytes`
  - `block_bytes`
  - `state_root_bytes`
- Decode helpers must validate decoded objects before returning them:
  - `decode_transaction`
  - `decode_signed_transaction`
  - `decode_block`

## Hash Domains

Core hashes are domain-separated. A hash from one domain must not be treated as
equivalent to another domain.

Hash domains:

- `Transaction`
- `SignedTransaction`
- `BlockHeader`
- `GenesisAllocation`
- `Coinbase`
- `MerkleNode`
- `AccountState`
- `StateNode`
- `Raw`

The type system distinguishes:

- `BlockHash`
- `TransactionHash`
- `MerkleHash`
- `StateRoot`
- `PreviousHash`

## Transactions

A transaction is valid when:

- transaction version is supported at the active height policy;
- amount is non-zero;
- fee is at least `MIN_FEE`;
- sender and recipient are different;
- sender account exists;
- sender nonce matches the transaction nonce;
- sender can spend `amount + fee` at the current block height.

Transaction outputs become spendable at:

```text
block_height + FINALITY_DEPTH
```

Current value:

```text
FINALITY_DEPTH = 2
```

So an output created in block `12` is spendable in block `14`.

## Blocks

A block is valid when:

- block version is supported at its height;
- genesis blocks have no coinbase and no transactions;
- non-genesis blocks have coinbase and no genesis allocations;
- transaction count and serialized size are within limits;
- total transaction fees do not overflow `u32`;
- coinbase total does not overflow `u32`;
- timestamp is not too far in the future;
- transaction formats are valid;
- merkle root matches block contents.

Chain linkage requires:

- first block height is `0`;
- first block previous hash is zero;
- next block height is active tip height + 1;
- next block previous hash equals active tip hash;
- next block timestamp is not earlier than active tip timestamp.

## Coinbase, Fees, And Rewards

Coinbase must be exact.

The miner address in coinbase must equal the block miner address.

Coinbase fees must equal the checked sum of all transaction fees in the block.

Coinbase subsidy must equal:

```text
min(block_reward(height), MAX_UNIT_SUPPLY - supply_after_fees_are_credited)
```

Fees are miner revenue but mature like normal transaction outputs:

```text
fee spendable height = block_height + FINALITY_DEPTH
```

Subsidy matures at:

```text
subsidy spendable height = block_height + BLOCK_REWARD_MATURITY
```

Current values:

```text
FINALITY_DEPTH = 2
BLOCK_REWARD_MATURITY = 20
```

## State Root

State root is calculated from accounts ordered by `BTreeMap` key order.

Account leaf hashes and state parent hashes use separate hash domains.

State transition must be atomic:

- if block application fails, ledger state and chain tip must not change.

## Genesis

Genesis is deterministic.

The canonical genesis hash is:

```text
32ac01d654c1fe57d12506456bb7237f4baf214a3573b11fcdb128974d95864f4031856cae53a859c5adc5d2880670739571057b71b2575642e5cce6d16efe1d
```

## Fork Choice And Reorg Planning

Fork choice selects the tip with the highest cumulative work.

Tie-breaker:

```text
lowest block hash wins
```

Core provides reorg planning:

- `common_ancestor`
- `plan_reorg`

Runtime code is responsible for storing competing branches and applying a reorg
plan to rebuild active state.

## Invariants

Core invariants include:

- total supply must never exceed `u32::MAX`;
- account map key must match account address;
- sum of account credits must equal account balance;
- failed block application must not mutate state;
- state root must be deterministic for the same state;
- domain-separated hashes must not be mixed.
