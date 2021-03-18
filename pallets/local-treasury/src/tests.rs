use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

const ASHLEY: AccountId = 0;

#[test]
fn unprivileged_account_can_deposit() {
    const INITIAL_BALANCE: Balance = 100;
    const SEND: Balance = 10;

    new_test_ext(vec![(ASHLEY, INITIAL_BALANCE)]).execute_with(|| {
        assert_eq!(Balances::free_balance(ASHLEY), INITIAL_BALANCE);
        assert_ok!(Balances::transfer(
            Origin::signed(ASHLEY),
            local_treasury_account_id(),
            SEND
        ));
        assert_eq!(Balances::free_balance(ASHLEY), INITIAL_BALANCE - SEND);
        assert_eq!(Balances::free_balance(local_treasury_account_id()), SEND);
    });
}

#[test]
fn unprivileged_account_cannot_withdraw() {
    const INITIAL_BALANCE: Balance = 100;
    const AMOUNT: Balance = 1;

    new_test_ext(vec![
        (local_treasury_account_id(), INITIAL_BALANCE),
        (ASHLEY, 0),
    ])
    .execute_with(|| {
        assert_noop!(
            LocalTreasury::withdraw(Origin::signed(ASHLEY), AMOUNT, ASHLEY),
            BadOrigin
        );
        assert_eq!(Balances::free_balance(ASHLEY), 0);
    });
}

#[test]
fn admin_account_can_withdraw() {
    const INITIAL_BALANCE: Balance = 100;
    const AMOUNT: Balance = 1;

    new_test_ext(vec![
        (local_treasury_account_id(), INITIAL_BALANCE),
        (ADMIN_ACCOUNT_ID, 0),
    ])
    .execute_with(|| {
        assert_ok!(LocalTreasury::withdraw(
            Origin::signed(ADMIN_ACCOUNT_ID),
            AMOUNT,
            ADMIN_ACCOUNT_ID
        ));
        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), AMOUNT);
    });
}
