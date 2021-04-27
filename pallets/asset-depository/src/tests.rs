// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use pallet::MultiAssetDepository;

const ASHLEY: AccountId = 0;
const BOB: AccountId = 1;
const ASSET_A: u32 = 0;

#[test]
fn depositing_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetDepository::deposit(&ASSET_A, &ASHLEY, 100));
        assert_noop!(
            AssetDepository::deposit(&ASSET_A, &BOB, u128::MAX),
            pallet::Error::<Test>::TotalBalanceOverflow
        );
        assert_ok!(AssetDepository::deposit(&ASSET_A, &BOB, 2_000));

        assert_eq!(AssetDepository::total_balance(&ASSET_A, &ASHLEY), 100);
        assert_eq!(AssetDepository::aggregated_balance(&ASSET_A), 2_100);
    });
}

#[test]
fn withdrawing_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetDepository::deposit(&ASSET_A, &ASHLEY, 100));
        assert_ok!(AssetDepository::deposit(&ASSET_A, &BOB, 2_000));

        assert_ok!(AssetDepository::withdraw(&ASSET_A, &BOB, 2_000));
        assert_eq!(AssetDepository::aggregated_balance(&ASSET_A), 100);

        assert_noop!(
            AssetDepository::withdraw(&ASSET_A, &BOB, 1),
            pallet::Error::<Test>::NotEnoughBalance
        );
    });
}
