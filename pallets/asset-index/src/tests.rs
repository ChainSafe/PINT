// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use pallet::types::{AssetAvailability, IndexAssetData};
use sp_runtime::traits::BadOrigin;
use xcm::v0::MultiLocation;

const ASHLEY: AccountId = 0;
const ASSET_A: u32 = 0;

#[test]
fn non_admin_cannot_call_get_asset() {
    let initial_balances: Vec<(u64, u64)> = vec![(ASHLEY, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_noop!(
            AssetIndex::add_asset(
                Origin::signed(ASHLEY),
                ASSET_A,
                100,
                AssetAvailability::Liquid(MultiLocation::Null),
                200
            ),
            BadOrigin
        );
        assert_eq!(pallet::Holdings::<Test>::contains_key(ASSET_A), false)
    });
}

#[test]
fn admin_can_add_asset() {
    let initial_balances: Vec<(u64, u64)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_eq!(
            pallet::Holdings::<Test>::get(ASSET_A),
            Some(IndexAssetData::new(
                100,
                AssetAvailability::Liquid(MultiLocation::Null)
            ))
        );
        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 5);
    });
}

#[test]
fn admin_can_add_asset_twice_and_units_accumulate() {
    let initial_balances: Vec<(u64, u64)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_eq!(
            pallet::Holdings::<Test>::get(ASSET_A),
            Some(IndexAssetData::new(
                200,
                AssetAvailability::Liquid(MultiLocation::Null)
            ))
        );
        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 10);
    });
}
