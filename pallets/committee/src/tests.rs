use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use pallet_balances::Error as BalancesError;
use sp_runtime::traits::BadOrigin;

#[test]
fn something() {
    new_test_ext().execute_with(|| {
        assert_eq!(1, 1);
    });
}
