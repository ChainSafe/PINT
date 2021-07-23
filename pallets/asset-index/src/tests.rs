// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok, sp_runtime::FixedU128};
use orml_traits::MultiCurrency;
use pallet::{
    traits::AssetRecorder,
    types::{AssetAvailability, AssetWithdrawal, RedemptionState},
};
use pallet_price_feed::PriceFeed;
use sp_runtime::{traits::BadOrigin, FixedPointNumber};
use xcm::v0::MultiLocation;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub fn new_test_ext_with_balance(
    balances: Vec<(AccountId, AssetId, Balance)>,
) -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().with_balances(balances).build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn non_admin_cannot_call_get_asset() {
    new_test_ext_with_balance(vec![]).execute_with(|| {
        assert_noop!(
            AssetIndex::add_asset(
                Origin::signed(ASHLEY),
                ASSET_A_ID,
                100,
                MultiLocation::Null,
                200
            ),
            BadOrigin
        );
        assert_eq!(pallet::Assets::<Test>::contains_key(ASSET_A_ID), false)
    });
}

#[test]
fn admin_can_add_asset() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            MultiLocation::Null,
            5
        ));
        assert_eq!(
            pallet::Assets::<Test>::get(ASSET_A_ID),
            Some(AssetAvailability::Liquid(MultiLocation::Null))
        );
        assert_eq!(AssetIndex::index_total_asset_balance(ASSET_A_ID), 100);

        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 5);

        assert_eq!(AssetIndex::index_token_balance(&ADMIN_ACCOUNT_ID), 5);
        assert_eq!(AssetIndex::index_token_issuance(), 5);
    });
}

#[test]
fn native_asset_disallowed() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AssetIndex::add_asset(
                Origin::signed(ADMIN_ACCOUNT_ID),
                PINT_ASSET_ID,
                100,
                MultiLocation::Null,
                5
            ),
            pallet::Error::<Test>::NativeAssetDisallowed
        );
    });
}

#[test]
fn admin_can_add_asset_twice_and_units_accumulate() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            MultiLocation::Null,
            5
        ));
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            MultiLocation::Null,
            5
        ));
        assert_eq!(
            pallet::Assets::<Test>::get(ASSET_A_ID),
            Some(AssetAvailability::Liquid(MultiLocation::Null))
        );
        assert_eq!(AssetIndex::index_total_asset_balance(ASSET_A_ID), 200);
        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 10);
    });
}

#[test]
fn non_admin_cannot_set_metadata() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AssetIndex::set_metadata(
                Origin::signed(ASHLEY),
                ASSET_A_ID,
                b"dot".to_vec(),
                b"dot".to_vec(),
                8,
            ),
            BadOrigin
        );
    });
}

#[test]
fn admin_can_set_metadata() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetIndex::set_metadata(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            b"dot".to_vec(),
            b"dot".to_vec(),
            8,
        ));
    });
}

#[test]
fn admin_can_update_metadata() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetIndex::set_metadata(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            b"dot".to_vec(),
            b"dot".to_vec(),
            8,
        ));

        assert_eq!(
            <pallet::Metadata<Test>>::get(ASSET_A_ID).name,
            b"dot".to_vec()
        );

        assert_ok!(AssetIndex::set_metadata(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            b"pint".to_vec(),
            b"pint".to_vec(),
            8,
        ));

        assert_eq!(
            <pallet::Metadata<Test>>::get(ASSET_A_ID).name,
            b"pint".to_vec()
        );
    });
}

#[test]
fn deposit_only_works_for_added_liquid_assets() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), ASSET_A_ID, 1_000),
            pallet::Error::<Test>::UnsupportedAsset
        );
        assert_ok!(AssetIndex::add_saft(&ADMIN_ACCOUNT_ID, ASSET_A_ID, 100, 5));
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), ASSET_A_ID, 1_000),
            pallet::Error::<Test>::UnsupportedAsset
        );
    });
}

#[test]
fn deposit_fail_for_native_asset() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), PINT_ASSET_ID, 1_000),
            pallet::Error::<Test>::NativeAssetDisallowed
        );
    });
}

#[test]
fn deposit_works_with_user_balance() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            MultiLocation::Null,
            5
        ));
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), ASSET_A_ID, 1_000),
            orml_tokens::Error::<Test>::BalanceTooLow
        );
        // deposit some funds in the account
        assert_ok!(Currency::deposit(ASSET_A_ID, &ASHLEY, 1_000));
        assert_ok!(AssetIndex::deposit(
            Origin::signed(ASHLEY),
            ASSET_A_ID,
            1_000
        ));
        assert_eq!(Currency::total_balance(ASSET_A_ID, &ASHLEY), 0);

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
    new_test_ext().execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            MultiLocation::Null,
            5
        ));
        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), UNKNOWN_ASSET_ID, 1_000),
            pallet::Error::<Test>::UnsupportedAsset
        );
    })
}

#[test]
fn deposit_ok_for_when_price_feed_unavailable() {
    let balance = vec![(ADMIN_ACCOUNT_ID, UNKNOWN_ASSET_ID, 1000)];
    new_test_ext_with_balance(balance).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            UNKNOWN_ASSET_ID,
            100,
            MultiLocation::Null,
            5
        ));
        assert_ok!(Currency::deposit(UNKNOWN_ASSET_ID, &ASHLEY, 1_000));
        assert_ok!(AssetIndex::deposit(
            Origin::signed(ASHLEY),
            UNKNOWN_ASSET_ID,
            1
        ),);
    })
}

#[test]
fn can_add_saft() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetIndex::add_saft(&ADMIN_ACCOUNT_ID, ASSET_A_ID, 100, 5),);
        assert_eq!(
            pallet::Assets::<Test>::get(ASSET_A_ID),
            Some(AssetAvailability::Saft)
        );
        assert_eq!(AssetIndex::index_total_asset_balance(ASSET_A_ID), 100);
        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), 5);
        assert_eq!(AssetIndex::index_token_balance(&ADMIN_ACCOUNT_ID), 5);
        assert_eq!(AssetIndex::index_token_issuance(), 5);
    });
}

#[test]
fn add_saft_fails_on_liquid_already_registered() {
    let balance = vec![(ADMIN_ACCOUNT_ID, UNKNOWN_ASSET_ID, 1000)];
    new_test_ext_with_balance(balance).execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            UNKNOWN_ASSET_ID,
            100,
            MultiLocation::Null,
            5
        ));
        assert_noop!(
            AssetIndex::add_saft(&ADMIN_ACCOUNT_ID, UNKNOWN_ASSET_ID, 100, 5),
            pallet::Error::<Test>::ExpectedSAFT
        );
    })
}

#[test]
fn deposit_fails_on_overflowing() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            100,
            MultiLocation::Null,
            5
        ));

        assert_noop!(
            AssetIndex::deposit(Origin::signed(ASHLEY), ASSET_A_ID, Balance::MAX),
            pallet::Error::<Test>::AssetVolumeOverflow
        );
    })
}

#[test]
fn can_calculates_nav() {
    new_test_ext().execute_with(|| {
        let a_units = 100;
        let b_units = 3000;
        let liquid_units = 5;
        let saft_units = 50;

        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            a_units,
            MultiLocation::Null,
            liquid_units
        ));

        assert_ok!(AssetIndex::add_saft(
            &ADMIN_ACCOUNT_ID,
            ASSET_B_ID,
            b_units,
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

        assert_ok!(Currency::deposit(ASSET_A_ID, &ASHLEY, 100_000));
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
    new_test_ext().execute_with(|| {
        let a_units = 100;
        let b_units = 3000;
        let a_tokens = 500;
        let b_tokens = 100;

        // create liquid assets
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A_ID,
            a_units,
            MultiLocation::Null,
            a_tokens
        ));
        assert_ok!(AssetIndex::add_asset(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_B_ID,
            b_units,
            MultiLocation::Null,
            b_tokens
        ));

        // deposit some funds into the index from an user account
        assert_ok!(Currency::deposit(ASSET_A_ID, &ASHLEY, 1_000));
        assert_ok!(Currency::deposit(ASSET_B_ID, &ASHLEY, 2_000));
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
        let asset_a_units = AssetIndex::index_total_asset_balance(ASSET_A_ID);

        assert_eq!(asset_a_units, a_units + 1_000);
        let asset_b_units = AssetIndex::index_total_asset_balance(ASSET_B_ID);
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

        // all SAFT holdings are ignored during withdrawal and don't have any effect on
        // the payout
        assert_ok!(AssetIndex::add_saft(
            &ADMIN_ACCOUNT_ID,
            SAFT_ASSET_ID,
            1_000,
            2_000
        ));

        // try to withdraw all funds, but are locked
        assert_noop!(
            AssetIndex::withdraw(
                Origin::signed(ASHLEY),
                AssetIndex::index_token_balance(&ASHLEY)
            ),
            pallet_balances::Error::<Test>::LiquidityRestrictions
        );

        // all index token are currently locked
        assert_eq!(
            pallet::LockedIndexToken::<Test>::get(&ASHLEY),
            AssetIndex::index_token_balance(&ASHLEY)
        );

        // advance the block number so that the lock expires
        frame_system::Pallet::<Test>::set_block_number(LockupPeriod::get() + 1);

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
                .find(|x| x.asset == ASSET_A_ID)
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

        // make sure the holding balance is updated
        assert_eq!(
            AssetIndex::index_total_asset_balance(ASSET_A_ID),
            asset_a_units - a_redeemed_units
        );
        assert_eq!(
            AssetIndex::index_total_asset_balance(ASSET_B_ID),
            asset_b_units - b_redeemed_units
        );
    })
}
