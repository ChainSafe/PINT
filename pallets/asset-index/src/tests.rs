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
fn admin_can_remove_saft_asset() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Saft,
            5
        ));

        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 5);

        // remove saft asset
        assert_ok!(AssetIndex::remove_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            None,
            None,
            5,
        ));

        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 0);
    });
}

#[test]
fn admin_can_remove_asset_twice_and_units_accumulate() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Saft,
            5
        ));
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Saft,
            5
        ));
        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 10);

        // remove assets
        assert_ok!(AssetIndex::remove_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            None,
            None,
            5
        ));

        assert_ok!(AssetIndex::remove_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            None,
            None,
            5
        ));

        assert_eq!(
            pallet::Holdings::<Test>::get(ASSET_A_ID),
            Some(IndexAssetData::new(0, AssetAvailability::Saft))
        );
        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 0);
    });
}

#[test]
fn admin_can_remove_liquid_asset() {
    let initial_balances: Vec<(AccountId, Balance)> =
        vec![(ADMIN_ACCOUNT_ID, 0), (RECEIPIENT_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            AssetAvailability::Liquid(MultiLocation::Null),
            5
        ));

        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 5);

        // remove saft asset
        assert_ok!(AssetIndex::remove_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            Some(MultiLocation::Null),
            Some(RECEIPIENT_ACCOUNT_ID),
            5,
        ));

        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 0);
        assert_eq!(Balances::free_balance(RECEIPIENT_ACCOUNT_ID), 5);
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
            AssetAvailability::Saft,
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
        assert_ok!(AssetIndex::deposit(
            Origin::signed(ASHLEY),
            ASSET_A_ID,
            1_000
        ));
    })
}

#[test]
fn can_calculates_nav() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        let a_units = 100;
        let b_units = 3000;
        let liquid_units = 5;
        let saft_units = 50;

        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            a_units,
            AssetAvailability::Liquid(MultiLocation::Null),
            liquid_units
        ));

        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_B_ID,
            b_units,
            AssetAvailability::Saft,
            saft_units
        ));

        let total_pint = AssetIndex::index_token_issuance();
        assert_eq!(total_pint, saft_units + liquid_units);

        let asset_volume = a_units * ASSET_A_PRICE_MULTIPLIER + b_units * ASSET_B_PRICE_MULTIPLIER;

        let total_nav = AssetIndex::total_nav().unwrap();
        assert_eq!(total_nav, asset_volume / total_pint);

        let saft_nav = AssetIndex::saft_nav().unwrap();
        assert_eq!(saft_nav, b_units * ASSET_B_PRICE_MULTIPLIER / total_pint);

        let liquid_nav = AssetIndex::liquid_nav().unwrap();
        assert_eq!(liquid_nav, a_units * ASSET_A_PRICE_MULTIPLIER / total_pint);

        assert_ok!(AssetDepository::deposit(&ASSET_A_ID, &ASHLEY, 100_000));
        assert_ok!(AssetIndex::deposit(
            Origin::signed(ASHLEY),
            ASSET_A_ID,
            1_000
        ));

        let total_pint = AssetIndex::index_token_issuance();
        assert_eq!(
            total_pint,
            saft_units + liquid_units + 1_000 * ASSET_A_PRICE_MULTIPLIER
        );

        let asset_volume =
            (a_units + 1_000) * ASSET_A_PRICE_MULTIPLIER + b_units * ASSET_B_PRICE_MULTIPLIER;
        let total_nav = AssetIndex::total_nav().unwrap();
        assert_eq!(total_nav, asset_volume / total_pint);
    })
}
