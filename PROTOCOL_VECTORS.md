# Paqus Protocol Vectors

Frozen canonical compatibility vectors for the current protocol and encoding
profile `paqus-borsh-le`. The executable source of truth is
`src/test/protocol_vectors.rs`. These values must not be regenerated silently;
any intentional byte change requires an explicit protocol decision and new vectors.

All integers use Borsh little-endian encoding. Keys and ML-DSA-87 signatures are
derived deterministically from 32-byte seeds: transfer `0x01` and eCash `0x02`.

`Envelope digest` is `hash_bytes(SignedProtocolTransaction bytes)`. `txid`
commits to the payload and intentionally excludes the witness. `wtxid` uses the
domain-separated unified envelope, including family, public key, and signature.

| Family | Bytes | Envelope digest | txid | wtxid |
|---|---:|---|---|---|
| Transfer | 7309 | `68054e012f0830bddf44502219f7617be3a1382501277d8db0a4c6fc1110d257` | `d80da8dcced3cbc6eeddce84d9e6420390e20a260fb8f4573e7890cb32b6f964` | `ad96ece725f117a9eb5e99523c4f011b9746b134c7f365db72ead8e100ca7456` |
| eCash | 7332 | `5b8fe8c217ea846185a4257824c7da1a0c82335b746373ef8d1b6714cf13f041` | `78bf83e96dc4448a42437951ea3d0b2259b5da5f0516808a763dca69f0f5db21` | `76329a85c265ec9e5c7e84ecd0d5af45a3b15ec4c7ac1a0b0ef9639495254f58` |

## Mixed-family SegWit block v1

The vector block contains the two envelopes above at height 42, previous hash
`55` repeated 32 times, miner address `99` repeated 20 times, difficulty 7,
timestamp `1700000042`, nonce 9, and state root `77` repeated 32 times.

| Field | Value |
|---|---|
| Canonical byte length | `14873` |
| Canonical byte digest | `c681ad3b2e7aef1a1972a66ff9a1f98bd795d9024f886603c3c5d908cc6f89cf` |
| Block hash | `00645a305af69f71da9b6d7990ec909a661a7b3b47a86edd3cb70b2fa06f801b` |
| Merkle root | `c05c0046170398f3cac3ce6e5f0834e204757b13bc2f4fcb1afe214c67c778db` |
| Witness root | `b7af85f1dac8c0effa474fb63200185d0dcd45c99951227f3b609a0bd1fc44c5` |

## Protocol event

The event is the transfer receipt at index 0 in the vector block.

- Event ID: `a9c0c299f92bdb02e7ed3fe64ec03511d2d062380f11cb89a38d1e99c6569438`
- Canonical bytes:

```text
012a0000000000000000645a305af69f71da9b6d7990ec909a661a7b3b47a86edd3cb70b2fa06f801b01d80da8dcced3cbc6eeddce84d9e6420390e20a260fb8f4573e7890cb32b6f9640000000000589a5fa09aa6e8f47096c82a566389a6d725f983212121212121212121212121212121212121212165000000000000000200000000000000
```

Changing any canonical field, enum ordering, hash domain, ML-DSA encoding, or
Borsh layout must change these vectors through an explicit protocol upgrade.
