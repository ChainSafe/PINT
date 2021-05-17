// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;
use crate::types::{AssetWithdrawal, RedemptionState};
use frame_support::sp_runtime::FixedU128;
use frame_support::{assert_noop, assert_ok};
use pallet::types::{AssetAvailability, IndexAssetData};
use pallet_asset_depository::MultiAssetDepository;
use pallet_price_feed::PriceFeed;
use sp_runtime::traits::BadOrigin;
use sp_runtime::FixedPointNumber;
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

        assert_eq!(AssetIndex::index_token_balance(&ADMIN_ACCOUNT_ID), 5);
        assert_eq!(AssetIndex::index_token_issuance(), 5);
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
        assert_eq!(AssetIndex::index_token_issuance(), expected_balance + 5);
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

#[test]
fn can_withdraw() {
    let initial_balances: Vec<(AccountId, Balance)> = vec![(ADMIN_ACCOUNT_ID, 0)];
    new_test_ext(initial_balances).execute_with(|| {
        let a_units = 100;
        let b_units = 3000;
        let a_tokens = 500;
        let b_tokens = 100;

        // create liquid assets
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            a_units,
            AssetAvailability::Liquid(MultiLocation::Null),
            a_tokens
        ));
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_B_ID,
            b_units,
            AssetAvailability::Liquid(MultiLocation::Null),
            b_tokens
        ));

        // deposit some funds into the index from an user account
        assert_ok!(AssetDepository::deposit(&ASSET_A_ID, &ASHLEY, 1_000));
        assert_ok!(AssetDepository::deposit(&ASSET_B_ID, &ASHLEY, 2_000));
        assert_ok!(AssetIndex::deposit(
            Origin::signed(ASHLEY),
            ASSET_A_ID,
            1_000
        ));
        assert_ok!(AssetIndex::deposit(
            Origin::signed(ASHLEY),
            ASSET_B_ID,
            2_000
        ));

        // make sure the total issuance is equals the converted assets using price feed
        let total_pint = AssetIndex::index_token_issuance();
        assert_eq!(
            total_pint,
            a_tokens
                + b_tokens
                + 1_000 * ASSET_A_PRICE_MULTIPLIER
                + 2_000 * ASSET_B_PRICE_MULTIPLIER
        );
        let user_pint = AssetIndex::index_token_balance(&ASHLEY);
        assert_eq!(
            user_pint,
            1_000 * ASSET_A_PRICE_MULTIPLIER + 2_000 * ASSET_B_PRICE_MULTIPLIER
        );

        assert_noop!(
            AssetIndex::withdraw(Origin::signed(ASHLEY), 1),
            pallet::Error::<Test>::MinimumRedemption
        );

        let total_nav = AssetIndex::total_nav().unwrap();
        let asset_a_units = pallet::Holdings::<Test>::get(&ASSET_A_ID).unwrap().units;
        assert_eq!(asset_a_units, a_units + 1_000);
        let asset_b_units = pallet::Holdings::<Test>::get(&ASSET_B_ID).unwrap().units;
        assert_eq!(asset_b_units, b_units + 2_000);
        let total_value = total_nav * AssetIndex::index_token_issuance();
        assert_eq!(
            total_value,
            asset_a_units * ASSET_A_PRICE_MULTIPLIER + asset_b_units * ASSET_B_PRICE_MULTIPLIER
        );

        // ratio of asset a to total value
        let a_proportional =
            FixedU128::checked_from_rational(asset_a_units * ASSET_A_PRICE_MULTIPLIER, total_value)
                .unwrap();

        let b_proportional =
            FixedU128::checked_from_rational(asset_b_units * ASSET_B_PRICE_MULTIPLIER, total_value)
                .unwrap();

        // proportional pint to withdraw
        let a_proportional_tokens = a_proportional.checked_mul_int(user_pint).unwrap();
        let b_proportional_tokens = b_proportional.checked_mul_int(user_pint).unwrap();

        // the redeemed distribution of units, converted with price
        let a_redeemed_units = a_proportional_tokens / ASSET_A_PRICE_MULTIPLIER;
        let b_redeemed_units = b_proportional_tokens / ASSET_B_PRICE_MULTIPLIER;

        assert!(a_proportional_tokens + b_proportional_tokens <= user_pint);

        // all SAFT holdings are ignored during withdrawal and don't have any effect on the payout
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            99,
            1_000,
            AssetAvailability::Saft,
            2_000
        ));

        // withdraw all funds
        assert_ok!(AssetIndex::withdraw(
            Origin::signed(ASHLEY),
            AssetIndex::index_token_balance(&ASHLEY)
        ));

        // account for rounding
        let remaining = user_pint - (a_proportional_tokens + b_proportional_tokens);
        assert_eq!(AssetIndex::index_token_balance(&ASHLEY), remaining);

        let mut pending = pallet::PendingWithdrawals::<Test>::get(&ASHLEY)
            .expect("pending withdrawals should be present");

        assert_eq!(pending.len(), 1);
        let pending = pending.remove(0);
        assert_eq!(pending.assets.len(), 2);

        assert_eq!(
            pending
                .assets
                .iter()
                .filter(|x| x.asset == ASSET_A_ID)
                .next()
                .expect("asset should be present"),
            &AssetWithdrawal {
                asset: ASSET_A_ID,
                state: RedemptionState::Unbonding,
                units: a_redeemed_units
            }
        );

        assert_eq!(
            pending
                .assets
                .iter()
                .filter(|x| x.asset == ASSET_B_ID)
                .next()
                .expect("asset should be present"),
            &AssetWithdrawal {
                asset: ASSET_B_ID,
                state: RedemptionState::Unbonding,
                units: b_redeemed_units
            }
        );
    })
}
