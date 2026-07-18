use criterion::{Criterion, criterion_group, criterion_main};
use paqus::block::{Block, Height, Nonce};
use paqus::consensus::Consensus;
use paqus::crypto::{Address, HASH_SIZE, Hash};
use std::hint::black_box;

fn sha3_512_proof_of_work(c: &mut Criterion) {
    let consensus = Consensus::with_default_config();
    let block = Block::new(
        Height(1),
        Hash([7; HASH_SIZE]),
        Address([9; 20]),
        1_700_000_001,
        Nonce(42),
        vec![],
    );
    let mut group = c.benchmark_group("proof_of_work");
    group.bench_function("sha3_512_header_hash", |b| {
        b.iter(|| {
            black_box(
                black_box(&consensus)
                    .proof_of_work_hash(black_box(&block))
                    .unwrap(),
            )
        })
    });
    group.finish();
}

criterion_group!(pow_benches, sha3_512_proof_of_work);
criterion_main!(pow_benches);
