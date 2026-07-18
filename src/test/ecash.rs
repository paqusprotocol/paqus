use crate::consensus::supply::{Amount, XPQ};
use crate::ecash::{
    CashCoin, CashDenomination, EcashError, EcashMetadata, EcashOperation, WithdrawCashMetadata,
    format_cash_coins,
};

#[test]
fn formats_cash_with_canonical_denominations() {
    let coins = format_cash_coins(Amount(188 * XPQ)).unwrap();
    assert_eq!(
        coins,
        vec![
            CashCoin {
                denomination: CashDenomination::OneHundred,
                count: 1
            },
            CashCoin {
                denomination: CashDenomination::Fifty,
                count: 1
            },
            CashCoin {
                denomination: CashDenomination::Twenty,
                count: 1
            },
            CashCoin {
                denomination: CashDenomination::Ten,
                count: 1
            },
            CashCoin {
                denomination: CashDenomination::Five,
                count: 1
            },
            CashCoin {
                denomination: CashDenomination::Two,
                count: 1
            },
            CashCoin {
                denomination: CashDenomination::One,
                count: 1
            },
        ]
    );
}

#[test]
fn compresses_repeated_cash_coins() {
    assert_eq!(
        format_cash_coins(Amount(250 * XPQ)).unwrap(),
        vec![
            CashCoin {
                denomination: CashDenomination::OneHundred,
                count: 2
            },
            CashCoin {
                denomination: CashDenomination::Fifty,
                count: 1
            },
        ]
    );
}

#[test]
fn creates_deposit_and_withdraw_metadata() {
    let deposit = EcashMetadata::deposit(Amount(57 * XPQ)).unwrap();
    assert_eq!(deposit.operation, EcashOperation::Deposit);
    assert_eq!(deposit.amount(), Ok(Amount(57 * XPQ)));
    assert_eq!(deposit.validate(), Ok(()));

    let withdraw = EcashMetadata::withdraw(Amount(57 * XPQ)).unwrap();
    assert_eq!(withdraw.operation, EcashOperation::Withdraw);
    assert_eq!(withdraw.amount(), Ok(Amount(57 * XPQ)));
}

#[test]
fn rejects_zero_and_fractional_xpq_amounts() {
    assert_eq!(format_cash_coins(Amount(0)), Err(EcashError::ZeroAmount));
    assert_eq!(
        format_cash_coins(Amount(XPQ + 1)),
        Err(EcashError::FractionalXpQ)
    );
}

#[test]
fn rejects_non_canonical_metadata() {
    let metadata = EcashMetadata {
        operation: EcashOperation::Deposit,
        coins: vec![
            CashCoin {
                denomination: CashDenomination::One,
                count: 1,
            },
            CashCoin {
                denomination: CashDenomination::Ten,
                count: 1,
            },
        ],
    };
    assert_eq!(metadata.validate(), Err(EcashError::NonCanonicalCoins));
}

#[test]
fn automatic_withdraw_keeps_fractional_xpq_on_chain() {
    let requested = Amount(1_000 * XPQ + XPQ / 10);
    let plan = WithdrawCashMetadata::plan_automatic(requested).unwrap();

    assert_eq!(plan.requested_amount, requested);
    assert_eq!(plan.cash_amount, Amount(1_000 * XPQ));
    assert_eq!(plan.remainder, Amount(XPQ / 10));
    assert_eq!(plan.denominations, vec![CashDenomination::OneHundred; 10]);

    let commitments: Vec<[u8; 32]> = (0..10).map(|index| [index; 32]).collect();
    let metadata = WithdrawCashMetadata::from_automatic_plan(&plan, &commitments).unwrap();
    assert_eq!(metadata.amount(), Ok(plan.cash_amount));
}

#[test]
fn automatic_withdraw_rejects_amount_without_one_whole_xpq() {
    assert_eq!(
        WithdrawCashMetadata::plan_automatic(Amount(XPQ / 10)),
        Err(EcashError::NoCashableAmount)
    );
}
