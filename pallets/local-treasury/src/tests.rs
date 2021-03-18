use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

const ASHLEY: AccountId = 0;

fn assert_balances(balances: &[(AccountId, Balance)]) {
    for (account, balance) in balances {
        assert_eq!(&Balances::free_balance(account), balance)
    }
}

#[test]
fn unprivileged_account_can_deposit() {
    const INITIAL_BALANCE: Balance = 100;
    const AMOUNT: Balance = 10;

    let initial_balances: Vec<(u64, u64)> =
        vec![(local_treasury_account_id(), 0), (ASHLEY, INITIAL_BALANCE)];

    let final_balances: Vec<(u64, u64)> = vec![
        (local_treasury_account_id(), AMOUNT),
        (ASHLEY, INITIAL_BALANCE - AMOUNT),
    ];

    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(Balances::transfer(
            Origin::signed(ASHLEY),
            local_treasury_account_id(),
            AMOUNT
        ));
        assert_balances(&final_balances);
    });
}

#[test]
fn unprivileged_account_cannot_withdraw() {
    const INITIAL_BALANCE: Balance = 100;
    const AMOUNT: Balance = 10;

    let initial_balances: Vec<(u64, u64)> =
        vec![(local_treasury_account_id(), 0), (ASHLEY, INITIAL_BALANCE)];

    new_test_ext(initial_balances.clone()).execute_with(|| {
        assert_noop!(
            LocalTreasury::withdraw(Origin::signed(ASHLEY), AMOUNT, ASHLEY),
            BadOrigin
        );
        assert_balances(&initial_balances);
    });
}

#[test]
fn admin_account_can_withdraw() {
    const INITIAL_BALANCE: Balance = 100;
    const AMOUNT: Balance = 1;

    let initial_balances: Vec<(u64, u64)> = vec![
        (local_treasury_account_id(), INITIAL_BALANCE),
        (ADMIN_ACCOUNT_ID, 0),
    ];

    let final_balances: Vec<(u64, u64)> = vec![
        (local_treasury_account_id(), INITIAL_BALANCE - AMOUNT),
        (ADMIN_ACCOUNT_ID, AMOUNT),
    ];

    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(LocalTreasury::withdraw(
            Origin::signed(ADMIN_ACCOUNT_ID),
            AMOUNT,
            ADMIN_ACCOUNT_ID
        ));
        assert_balances(&final_balances);
    });
}
