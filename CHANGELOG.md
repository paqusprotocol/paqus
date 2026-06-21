# Changelog

All notable Paqus core devnet changes are summarized here.

## 0.1.5 - Devnet

- Changed the default P2P bind address to `[::]:30333` so node applications can bind on IPv6 by default while still allowing IPv4 override.
- Kept the default RPC bind address at `127.0.0.1:9933` so local RPC remains private by default.
- Added parameter tests for default RPC and P2P socket addresses.

## 0.1.4 - Devnet

- Changed proof-of-work difficulty semantics from leading zero bytes to leading zero bits.
- Changed difficulty retargeting to use a 10-block adjustment window.
- Reduced devnet transaction finality depth to 1 block.
- Reduced devnet mining reward maturity to 20 blocks.
- Added automatic receiver account creation for incoming transfers.
- Added mempool byte accounting and `MAX_MEMPOOL_BYTES`.
- Added same-sender, same-nonce replacement-by-fee policy.
- Added transaction hash indexing in storage.
- Added address transaction history indexing in storage.
- Added storage APIs for transaction lookup and address transaction locations.
- Added network protocol handshake messages: `Version`, `VerAck`, and `Reject`.
- Added version compatibility checks for chain id, chain name, protocol stage, network magic, and protocol version.

## 0.1.3 - Devnet

- Fixed canonical genesis configuration.
- Preserved canonical genesis hash validation during node initialization.
- Cleaned up stored-data integrity error handling.

## 0.1.2 - Devnet

- Added canonical default node ports to core parameters:
  - RPC: `9933`
  - P2P: `30333`
- Added default bind address constants for node applications:
  - RPC: `127.0.0.1:9933`
  - P2P: `0.0.0.0:30333`
- Updated devnet target block time from 2 minutes to 5 minutes.
- Clarified that RPC and P2P runtime services live in node applications, while the core crate stays focused on protocol primitives.
- Removed the experimental async networking crate from the core direction.

## 0.1.1 - Devnet

- Updated devnet consensus and mining behavior.
- Improved mempool validation against ledger state.
- Added mining attempt budgeting and candidate block handling improvements.
- Expanded node balance reporting for confirmed, available, and pending balances.
- Added maturity tracking for mining rewards and received transaction credits.
- Added storage and node tests around devnet behavior.

## 0.1.0 - Devnet

- Initial Paqus core crate.
- Added block and transaction primitives.
- Added SHA3-512 hashing for block and transaction data.
- Added Argon2 proof-of-work hashing.
- Added ML-DSA-87 wallet keys, signatures, and transaction signing.
- Added address derivation and address formatting.
- Added ledger state, account credits, state roots, and state proofs.
- Added genesis block and genesis ledger helpers.
- Added consensus validation, reward schedule, and difficulty retarget primitives.
- Added mempool, miner helpers, fork choice, reorg handling, and node wrapper.
- Added sled-backed storage for blocks, accounts, state snapshots, and chain tip.
- Added basic network messages, peer wrapper, framed transport, and network handler.
