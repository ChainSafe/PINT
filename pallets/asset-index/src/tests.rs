// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use pallet::types::{AssetAvailability, IndexAssetData};
use pallet_asset_depository::MultiAssetDepository;
use pallet_price_feed::PriceFeed;
use sp_runtime::traits::BadOrigin;
use xcm::v0::MultiLocation;

const ASHLEY: AccountId = 0;

#[test]
fn non_admin_cannot_call_get_asset() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ASHLEY, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_noop!(
            AssetIndex::add_asset(
                Origin::signed(ASHLEY),
                ASSET_A_ID,
                100,
                AssetAvailability::Liquid(MultiLocation::Null),
                200
            ),
            BadOrigin
        );
        assert_eq!(pallet::Holdings::<Test>::contains_key(ASSET_A_ID), false)
    });
}

#[test]
fn admin_can_add_asset() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_eq!(
            pallet::Holdings::<Test>::get(ASSET_A_ID),
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
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_eq!(
            pallet::Holdings::<Test>::get(ASSET_A_ID),
            Some(IndexAssetData::new(
                200,
                AssetAvailability::Liquid(MultiLocation::Null)
            ))
        );
        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 10);
    });
}

#[test]
fn deposit_only_works_for_added_liquid_assets() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), ASSET_A_ID, 1_000),
            pallet::Error::<Test>::UnsupportedAsset
        );
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::SAFT,
            5
        ));
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), ASSET_A_ID, 1_000),
            pallet::Error::<Test>::UnsupportedAsset
        );
    });
}

#[test]
fn deposit_works_with_user_balance() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), ASSET_A_ID, 1_000),
            pallet_asset_depository::Error::<Test>::NotEnoughBalance
        );

        // deposit some funds in the account
        assert_ok!(AssetDepository::deposit(&ASSET_A_ID, &ASHLEY, 1_000));
        assert_ok!(AssetIndex::deposit(
            Origin::signed(ASHLEY),
            ASSET_A_ID,
            1_000
        ));
        assert_eq!(AssetDepository::total_balance(&ASSET_A_ID, &ASHLEY), 0);

        let expected_balance = MockPriceFeed::get_price(ASSET_A_ID)
            .unwrap()
            .volume(1_000)
            .unwrap();

        assert_eq!(AssetIndex::index_token_balance(&ASHLEY), expected_balance);
    });
}

#[test]
fn deposit_fails_for_unknown_assets() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), UNKNOWN_ASSET_ID, 1_000),
            pallet::Error::<Test>::UnsupportedAsset
        );
    })
}

#[test]
fn deposit_fails_for_when_price_feed_unavailable() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            UNKNOWN_ASSET_ID,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), UNKNOWN_ASSET_ID, 1_000),
            pallet::Error::<Test>::UnsupportedAsset
        );
    })
}

#[test]
fn deposit_fails_on_overflowing() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));
        assert_ok!(AssetDepository::deposit(&ASSET_A_ID, &ASHLEY, Balance::MAX));
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), ASSET_A_ID, Balance::MAX),
            pallet::Error::<Test>::AssetVolumeOverflow
        );
        assert_ok!(
            AssetIndex::deposit(Origin::signed(ASHLEY), ASSET_A_ID, 1_000)
        );
    })
}