use crate::block::{Block, Height, Nonce};
use crate::consensus::supply::{Amount, XPQ};
use crate::crypto::{Address, BlockHash, address_from_public_key, generate_keypair, sign};
use crate::ledger::{Ledger, LedgerError, QCASH_DEPOSIT_MATURITY, QCASH_WITHDRAW_MATURITY};
use crate::qcash::{
    CashCoinFile, CashDenomination, DepositCashMetadata, WithdrawCashMetadata, cash_coin_commitment,
};
use crate::state::QCashUtxoError;
use crate::transaction::{QCashTransaction, SignedQCashTransaction};

fn sign_qcash(
    transaction: QCashTransaction,
    keypair: &crate::crypto::KeyPair,
) -> SignedQCashTransaction {
    let signature = sign(&keypair.secret_key, &transaction.signing_bytes());
    SignedQCashTransaction::new(transaction, keypair.public_key, signature)
}

#[test]
fn ledger_executes_withdraw_and_deposit_atomically() {
    let keypair = generate_keypair();
    let signer = address_from_public_key(&keypair.public_key);
    let recipient = Address([70; 20]);
    let initial = Amount(200 * XPQ);
    let fee = Amount(2);
    let secret = [71; 32];
    let withdraw_metadata = WithdrawCashMetadata::with_denominations(
        Amount(100 * XPQ),
        &[CashDenomination::OneHundred],
        &[cash_coin_commitment(&secret)],
    )
    .unwrap();
    let withdraw = QCashTransaction::withdraw(
        signer,
        Amount(100 * XPQ),
        fee,
        Nonce(0),
        withdraw_metadata.clone(),
    );
    let withdraw_hash = withdraw.hash();
    let signed_withdraw = sign_qcash(withdraw, &keypair);
    let mut ledger = Ledger::new();
    ledger.create_account(signer, initial).unwrap();

    ledger
        .apply_signed_qcash_transaction(&signed_withdraw, Height(0))
        .unwrap();
    assert_eq!(ledger.balance(&signer), Some(Amount(100 * XPQ - fee.0)));
    assert_eq!(ledger.qcash_utxos.spendable_balance(), Ok(Amount(0)));
    assert_eq!(ledger.qcash_utxos.total_value(), Ok(Amount(100 * XPQ)));
    assert_eq!(ledger.account(&signer).unwrap().nonce, Nonce(1));
    ledger.finalize_qcash_at(Height(QCASH_WITHDRAW_MATURITY as u64));
    assert_eq!(
        ledger.qcash_utxos.spendable_balance(),
        Ok(Amount(100 * XPQ))
    );

    let file = CashCoinFile::new(withdraw_hash, &withdraw_metadata.outputs[0], secret).unwrap();
    let deposit_metadata = DepositCashMetadata::new(&[file], recipient).unwrap();
    let deposit = QCashTransaction::deposit(signer, recipient, fee, Nonce(1), deposit_metadata);
    let signed_deposit = sign_qcash(deposit, &keypair);
    ledger
        .apply_signed_qcash_transaction(&signed_deposit, Height(101))
        .unwrap();

    assert_eq!(ledger.balance(&recipient), Some(Amount(100 * XPQ - fee.0)));
    assert_eq!(ledger.qcash_utxos.spendable_balance(), Ok(Amount(0)));
    assert_eq!(ledger.qcash_utxos.total_value(), Ok(Amount(0)));
    assert_eq!(ledger.account(&signer).unwrap().nonce, Nonce(2));
    assert_eq!(
        ledger
            .account(&recipient)
            .unwrap()
            .available_balance_at(Height(101 + QCASH_DEPOSIT_MATURITY as u64 - 1)),
        Amount(0)
    );
    ledger.finalize_qcash_at(Height(101 + QCASH_DEPOSIT_MATURITY as u64));
    assert_eq!(ledger.qcash_utxos.total_value(), Ok(Amount(0)));
    assert_eq!(
        ledger
            .account(&recipient)
            .unwrap()
            .available_balance_at(Height(101 + QCASH_DEPOSIT_MATURITY as u64)),
        Amount(100 * XPQ - fee.0)
    );
}

#[test]
fn failed_deposit_does_not_mutate_accounts_or_coin_state() {
    let keypair = generate_keypair();
    let signer = address_from_public_key(&keypair.public_key);
    let secret = [80; 32];
    let metadata = WithdrawCashMetadata::with_denominations(
        Amount(XPQ),
        &[CashDenomination::One],
        &[cash_coin_commitment(&secret)],
    )
    .unwrap();
    let withdraw = QCashTransaction::withdraw(signer, Amount(XPQ), Amount(0), Nonce(0), metadata);
    let mut ledger = Ledger::new();
    ledger.create_account(signer, Amount(2 * XPQ)).unwrap();
    ledger
        .apply_signed_qcash_transaction(&sign_qcash(withdraw.clone(), &keypair), Height(0))
        .unwrap();
    ledger.finalize_qcash_at(Height(100));
    let before = ledger.clone();

    let wrong_secret = [81; 32];
    let wrong_output = crate::qcash::QCashOutput {
        coin_index: 0,
        denomination: CashDenomination::One,
        commitment: cash_coin_commitment(&wrong_secret),
    };
    let wrong_file = CashCoinFile::new(withdraw.hash(), &wrong_output, wrong_secret).unwrap();
    let invalid = DepositCashMetadata::new(&[wrong_file], signer).unwrap();
    let deposit = QCashTransaction::deposit(signer, signer, Amount(0), Nonce(1), invalid);
    assert_eq!(
        ledger.apply_signed_qcash_transaction(&sign_qcash(deposit, &keypair), Height(1)),
        Err(LedgerError::InvalidQCashUtxo(QCashUtxoError::UnknownCoin))
    );
    assert_eq!(ledger, before);
}

#[test]
fn block_rollback_restores_account_balance_nonce_and_deposit_credit() {
    let keypair = generate_keypair();
    let signer = address_from_public_key(&keypair.public_key);
    let recipient = Address([90; 20]);
    let secret = [91; 32];
    let withdraw_metadata = WithdrawCashMetadata::with_denominations(
        Amount(10 * XPQ),
        &[CashDenomination::Ten],
        &[cash_coin_commitment(&secret)],
    )
    .unwrap();
    let withdraw = QCashTransaction::withdraw(
        signer,
        Amount(10 * XPQ),
        Amount(3),
        Nonce(0),
        withdraw_metadata.clone(),
    );
    let withdraw_block = BlockHash([92; 32]);
    let mut ledger = Ledger::new();
    ledger.create_account(signer, Amount(20 * XPQ)).unwrap();
    let account_before_withdraw = ledger.account(&signer).unwrap().clone();

    ledger
        .apply_signed_qcash_transaction_in_block(
            &sign_qcash(withdraw.clone(), &keypair),
            Height(5),
            withdraw_block,
        )
        .unwrap();
    assert_ne!(ledger.account(&signer).unwrap(), &account_before_withdraw);
    ledger.rollback_qcash_block(withdraw_block).unwrap();
    assert_eq!(ledger.account(&signer).unwrap(), &account_before_withdraw);

    // Re-issue on the active chain, mature it, then test a deposit reorg.
    let active_withdraw = withdraw.with_timestamp(1);
    let active_withdraw_hash = active_withdraw.hash();
    ledger
        .apply_signed_qcash_transaction(&sign_qcash(active_withdraw, &keypair), Height(5))
        .unwrap();
    ledger.finalize_qcash_at(Height(105));
    let file =
        CashCoinFile::new(active_withdraw_hash, &withdraw_metadata.outputs[0], secret).unwrap();
    let deposit = QCashTransaction::deposit(
        signer,
        recipient,
        Amount(2),
        Nonce(1),
        DepositCashMetadata::new(&[file], recipient).unwrap(),
    );
    let deposit_block = BlockHash([93; 32]);
    let signer_before_deposit = ledger.account(&signer).unwrap().clone();
    assert!(ledger.account(&recipient).is_none());

    ledger
        .apply_signed_qcash_transaction_in_block(
            &sign_qcash(deposit, &keypair),
            Height(106),
            deposit_block,
        )
        .unwrap();
    assert!(ledger.account(&recipient).is_some());
    ledger.rollback_qcash_block(deposit_block).unwrap();
    assert_eq!(ledger.account(&signer).unwrap(), &signer_before_deposit);
    assert!(ledger.account(&recipient).is_none());
    assert_eq!(ledger.qcash_utxos.spendable_balance(), Ok(Amount(10 * XPQ)));
}

#[test]
fn ledger_applies_and_rolls_back_segwit_block() {
    let keypair = generate_keypair();
    let signer = address_from_public_key(&keypair.public_key);
    let miner = Address([100; 20]);
    let metadata = WithdrawCashMetadata::with_denominations(
        Amount(XPQ),
        &[CashDenomination::One],
        &[cash_coin_commitment(&[101; 32])],
    )
    .unwrap();
    let transaction =
        QCashTransaction::withdraw(signer, Amount(XPQ), Amount(5), Nonce(0), metadata);
    let signed = sign_qcash(transaction, &keypair);
    let activation_height = 1;
    let anchor = Block::new(
        Height(activation_height - 1),
        crate::crypto::Hash([0; crate::crypto::HASH_SIZE]),
        miner,
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    let anchor_hash = anchor.hash();
    let mut ledger = Ledger::new();
    ledger.chain.blocks.insert(anchor.height(), anchor);
    ledger.chain.tip_height = Some(Height(activation_height - 1));
    ledger.chain.tip_hash = Some(anchor_hash);
    ledger.create_account(signer, Amount(2 * XPQ)).unwrap();
    let account_before = ledger.account(&signer).unwrap().clone();

    let mut block = Block::with_all_transactions(
        Height(activation_height),
        anchor_hash,
        miner,
        1,
        1_700_000_001,
        Nonce(0),
        vec![],
        vec![signed],
    )
    .unwrap();
    let state_root = ledger.state_root_after_block(&block).unwrap();
    block.set_state_root(state_root);
    let block_hash = block.hash();

    ledger.apply_block(block).unwrap();
    assert_eq!(ledger.tip_height(), Some(Height(activation_height)));
    assert_eq!(ledger.tip_hash(), Some(block_hash));
    assert_eq!(ledger.account(&signer).unwrap().nonce, Nonce(1));
    assert_eq!(ledger.qcash_utxos.total_value(), Ok(Amount(XPQ)));

    ledger.rollback_block(block_hash).unwrap();
    assert_eq!(ledger.tip_height(), Some(Height(activation_height - 1)));
    assert_eq!(ledger.tip_hash(), Some(anchor_hash));
    assert_eq!(ledger.account(&signer).unwrap(), &account_before);
    assert_eq!(ledger.qcash_utxos.total_value(), Ok(Amount(0)));
}
