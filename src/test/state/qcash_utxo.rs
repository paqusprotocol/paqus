use crate::block::Height;
use crate::consensus::supply::{Amount, XPQ};
use crate::crypto::{Address, BlockHash, TransactionHash};
use crate::ledger::QCASH_WITHDRAW_MATURITY;
use crate::qcash::{
    CashCoinFile, CashDenomination, DepositCashMetadata, QCashError, QCashMetadata,
    WithdrawCashMetadata, cash_coin_commitment, decode_cash_coin_file, encode_cash_coin_file,
};
use crate::state::{CashCoinId, QCashUtxoError, QCashUtxoSet, QCashUtxoStatus};

fn withdraw_metadata(amount: u64, seed: u8) -> WithdrawCashMetadata {
    let runs = crate::qcash::format_cash_coins(Amount(amount * XPQ)).unwrap();
    let count = runs.iter().map(|run| run.count as usize).sum();
    let commitments: Vec<[u8; 32]> = (0..count)
        .map(|index| [seed.wrapping_add(index as u8); 32])
        .collect();
    WithdrawCashMetadata::new(Amount(amount * XPQ), &commitments).unwrap()
}

#[test]
fn withdraw_issues_individually_tracked_coins() {
    let mut state = QCashUtxoSet::new();
    let metadata = withdraw_metadata(252, 1);
    let ids = state
        .apply_withdraw(
            Address([1; 20]),
            TransactionHash([1; 32]),
            &metadata,
            Height(0),
        )
        .unwrap();

    assert_eq!(ids.len(), 4);
    assert_eq!(state.coins().count(), 4);
    assert_eq!(state.spendable_balance(), Ok(Amount(0)));
    assert_eq!(state.total_value(), Ok(Amount(252 * XPQ)));
    state.finalize_at(Height(100));
    assert_eq!(state.spendable_balance(), Ok(Amount(252 * XPQ)));
    assert!(
        state
            .coins()
            .all(|coin| coin.status == QCashUtxoStatus::Spendable)
    );
}

#[test]
fn deposit_consumes_utxos_and_prevents_double_spend() {
    let mut state = QCashUtxoSet::new();
    let empty_root = state.consensus_root();
    let ids = state
        .apply_withdraw(
            Address([2; 20]),
            TransactionHash([2; 32]),
            &withdraw_metadata(52, 2),
            Height(0),
        )
        .unwrap();
    state.finalize_at(Height(100));
    assert_ne!(state.consensus_root(), empty_root);
    let deposit = QCashMetadata::deposit(Amount(52 * XPQ)).unwrap();

    assert_eq!(state.apply_deposit(&deposit, &ids), Ok(()));
    assert_eq!(state.spendable_balance(), Ok(Amount(0)));
    assert_eq!(state.coins().count(), 0);
    assert_eq!(state.consensus_root(), empty_root);
    assert_eq!(
        state.apply_deposit(&deposit, &ids),
        Err(QCashUtxoError::UnknownCoin)
    );
}

#[test]
fn deposit_is_atomic_when_a_coin_is_unknown() {
    let mut state = QCashUtxoSet::new();
    let ids = state
        .apply_withdraw(
            Address([3; 20]),
            TransactionHash([3; 32]),
            &withdraw_metadata(2, 3),
            Height(0),
        )
        .unwrap();
    state.finalize_at(Height(100));
    let mut invalid_ids = ids.clone();
    invalid_ids[0].0[0] ^= 0xff;

    assert_eq!(
        state.apply_deposit(
            &QCashMetadata::deposit(Amount(2 * XPQ)).unwrap(),
            &invalid_ids,
        ),
        Err(QCashUtxoError::UnknownCoin)
    );
    assert_eq!(
        state.coin(ids[0]).unwrap().status,
        QCashUtxoStatus::Spendable
    );
}

#[test]
fn deposit_requires_matching_denominations() {
    let mut state = QCashUtxoSet::new();
    let ids = state
        .apply_withdraw(
            Address([4; 20]),
            TransactionHash([4; 32]),
            &withdraw_metadata(10, 4),
            Height(0),
        )
        .unwrap();
    state.finalize_at(Height(100));

    assert_eq!(
        state.apply_deposit(&QCashMetadata::deposit(Amount(5 * XPQ)).unwrap(), &ids),
        Err(QCashUtxoError::DenominationMismatch)
    );
}

#[test]
fn coin_id_is_deterministic_and_formats_file_name() {
    let metadata = withdraw_metadata(50, 5);
    let output = &metadata.outputs[0];
    let tx_hash = TransactionHash([5; 32]);
    let id = CashCoinId::derive(tx_hash, output);

    assert_eq!(id, CashCoinId::derive(tx_hash, output));
    assert_ne!(id, CashCoinId::derive(TransactionHash([6; 32]), output));
    assert_eq!(id.short_id().len(), 9);
    assert_eq!(
        id.file_name(crate::qcash::CashDenomination::Fifty).len(),
        16
    );
    assert!(
        id.file_name(crate::qcash::CashDenomination::Fifty)
            .starts_with("50+")
    );
    assert!(
        id.file_name(crate::qcash::CashDenomination::Fifty)
            .ends_with(".XPQ")
    );
}

#[test]
fn repeated_withdraw_context_is_rejected_as_collision() {
    let mut state = QCashUtxoSet::new();
    let metadata = withdraw_metadata(1, 6);
    let withdrawer = Address([6; 20]);
    let tx_hash = TransactionHash([6; 32]);
    state
        .apply_withdraw(withdrawer, tx_hash, &metadata, Height(0))
        .unwrap();

    assert_eq!(
        state.apply_withdraw(withdrawer, tx_hash, &metadata, Height(0)),
        Err(QCashUtxoError::CoinIdCollision)
    );
}

#[test]
fn explicit_outputs_preserve_withdraw_origin_and_indexes() {
    let denominations = [CashDenomination::OneHundred; 10];
    let commitments: Vec<[u8; 32]> = (10..20).map(|seed| [seed; 32]).collect();
    let metadata =
        WithdrawCashMetadata::with_denominations(Amount(1_000 * XPQ), &denominations, &commitments)
            .unwrap();
    let withdrawer = Address([9; 20]);
    let tx_hash = TransactionHash([8; 32]);
    let mut state = QCashUtxoSet::new();
    let ids = state
        .apply_withdraw(withdrawer, tx_hash, &metadata, Height(0))
        .unwrap();

    assert_eq!(ids.len(), 10);
    for (index, id) in ids.iter().enumerate() {
        let coin = state.coin(*id).unwrap();
        assert_eq!(coin.withdrawer, withdrawer);
        assert_eq!(coin.outpoint.transaction_hash, tx_hash);
        assert_eq!(coin.outpoint.output_index, index as u32);
        assert_eq!(coin.denomination, denominations[index]);
        assert_eq!(coin.commitment, commitments[index]);
    }
}

#[test]
fn deposit_proof_redeems_files_with_valid_opening_secrets() {
    let secrets = [[21; 32], [22; 32]];
    let commitments = secrets.map(|secret| cash_coin_commitment(&secret));
    let metadata = WithdrawCashMetadata::with_denominations(
        Amount(150 * XPQ),
        &[CashDenomination::OneHundred, CashDenomination::Fifty],
        &commitments,
    )
    .unwrap();
    let tx_hash = TransactionHash([30; 32]);
    let mut state = QCashUtxoSet::new();
    state
        .apply_withdraw(Address([31; 20]), tx_hash, &metadata, Height(0))
        .unwrap();

    let files = [
        CashCoinFile::new(tx_hash, &metadata.outputs[0], secrets[0]).unwrap(),
        CashCoinFile::new(tx_hash, &metadata.outputs[1], secrets[1]).unwrap(),
    ];
    let recipient = Address([32; 20]);
    let deposit = DepositCashMetadata::new(&files, recipient).unwrap();
    let before_maturity = QCASH_WITHDRAW_MATURITY as u64 - 1;
    state.finalize_at(Height(before_maturity));
    assert_eq!(
        state.apply_deposit_proof(&deposit, recipient, Height(before_maturity)),
        Err(QCashUtxoError::CoinNotMature)
    );
    state.finalize_at(Height(QCASH_WITHDRAW_MATURITY as u64));
    assert_eq!(
        state.apply_deposit_proof(
            &deposit,
            recipient,
            Height(QCASH_WITHDRAW_MATURITY as u64 + 1),
        ),
        Ok(Amount(150 * XPQ))
    );
    assert_eq!(state.spendable_balance(), Ok(Amount(0)));
    assert_eq!(state.total_value(), Ok(Amount(0)));
    assert_eq!(
        state.apply_deposit_proof(&deposit, recipient, Height(202)),
        Err(QCashUtxoError::UnknownCoin)
    );
}

#[test]
fn cash_file_rejects_wrong_opening_secret() {
    let secret = [40; 32];
    let metadata = WithdrawCashMetadata::with_denominations(
        Amount(XPQ),
        &[CashDenomination::One],
        &[cash_coin_commitment(&secret)],
    )
    .unwrap();

    assert_eq!(
        CashCoinFile::new(TransactionHash([41; 32]), &metadata.outputs[0], [42; 32]),
        Err(QCashError::InvalidCommitment)
    );
}

#[test]
fn cash_file_codec_roundtrips_and_rejects_corruption() {
    let secret = [50; 32];
    let metadata = WithdrawCashMetadata::with_denominations(
        Amount(100 * XPQ),
        &[CashDenomination::OneHundred],
        &[cash_coin_commitment(&secret)],
    )
    .unwrap();
    let file = CashCoinFile::new(TransactionHash([51; 32]), &metadata.outputs[0], secret).unwrap();
    let encoded = encode_cash_coin_file(&file).unwrap();

    assert_eq!(decode_cash_coin_file(&encoded), Ok(file));
    assert!(!encoded.windows(32).any(|window| window == [51; 32]));
    let recipient = Address([52; 20]);
    let input = file.deposit_input(recipient).unwrap();
    assert_ne!(
        crate::codec::canonical_bytes(&input),
        crate::codec::canonical_bytes(&file)
    );
    assert_eq!(
        DepositCashMetadata::new(&[file], recipient).unwrap().inputs,
        vec![input]
    );

    let mut corrupted = encoded.clone();
    corrupted[20] ^= 0xff;
    assert_eq!(
        decode_cash_coin_file(&corrupted),
        Err(QCashError::InvalidCashFile)
    );
    assert_eq!(
        decode_cash_coin_file(b"random file renamed to coin.XPQ"),
        Err(QCashError::InvalidCashFile)
    );
}

#[test]
fn deposit_authorization_is_bound_to_recipient() {
    let secret = [53; 32];
    let output = crate::qcash::QCashOutput {
        coin_index: 0,
        denomination: CashDenomination::One,
        commitment: cash_coin_commitment(&secret),
    };
    let file = CashCoinFile::new(TransactionHash([54; 32]), &output, secret).unwrap();
    let recipient = Address([55; 20]);
    let attacker = Address([56; 20]);
    let metadata = DepositCashMetadata::new(&[file], recipient).unwrap();

    assert_eq!(metadata.validate_authorizations(recipient), Ok(()));
    assert_eq!(
        metadata.validate_authorizations(attacker),
        Err(QCashError::InvalidDepositAuthorization)
    );
}

#[test]
fn block_journal_removes_withdraw_coins_on_reorg() {
    let mut state = QCashUtxoSet::new();
    let before = state.clone();
    let block_hash = BlockHash([60; 32]);
    let ids = state
        .apply_withdraw_in_block(
            block_hash,
            Height(7),
            Address([61; 20]),
            TransactionHash([62; 32]),
            &withdraw_metadata(10, 63),
        )
        .unwrap();
    assert!(state.journal(block_hash).is_some());

    state.rollback_block(block_hash).unwrap();
    assert!(state.coin(ids[0]).is_none());
    assert_eq!(state.total_value(), Ok(Amount(0)));
    assert!(state.journal(block_hash).is_none());
    assert_eq!(state, before);
}

#[test]
fn block_journal_restores_spent_utxo_when_deposit_is_reorged() {
    let secret = [70; 32];
    let withdraw = WithdrawCashMetadata::with_denominations(
        Amount(XPQ),
        &[CashDenomination::One],
        &[cash_coin_commitment(&secret)],
    )
    .unwrap();
    let tx_hash = TransactionHash([71; 32]);
    let mut state = QCashUtxoSet::new();
    state
        .apply_withdraw(Address([72; 20]), tx_hash, &withdraw, Height(0))
        .unwrap();
    state.finalize_at(Height(100));
    let id = CashCoinId::derive(tx_hash, &withdraw.outputs[0]);
    let file = CashCoinFile::new(tx_hash, &withdraw.outputs[0], secret).unwrap();
    let recipient = Address([74; 20]);
    let deposit = DepositCashMetadata::new(&[file], recipient).unwrap();
    let block_hash = BlockHash([73; 32]);

    state
        .apply_deposit_in_block(block_hash, Height(101), &deposit, recipient)
        .unwrap();
    assert!(state.coin(id).is_none());
    state.rollback_block(block_hash).unwrap();
    assert_eq!(state.coin(id).unwrap().status, QCashUtxoStatus::Spendable);
}
