// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;
use crate::SAFTRecord;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

const ASHLEY: AccountId = 0;
const ASSET_A: u32 = 0;

#[test]
fn non_admin_cannot_call_any_extrinsics() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            SaftRegistry::add_saft(Origin::signed(ASHLEY), ASSET_A, 0, 0),
            BadOrigin
        );
        assert_noop!(
            SaftRegistry::remove_saft(Origin::signed(ASHLEY), ASSET_A, 0),
            BadOrigin
        );
        assert_noop!(
            SaftRegistry::report_nav(Origin::signed(ASHLEY), ASSET_A, 0, 0),
            BadOrigin
        );
    });
}

#[test]
fn admin_can_add_and_remove_saft() {
    let units = 20;
    let nav = 100;
    new_test_ext().execute_with(|| {
        // add
        assert_ok!(SaftRegistry::add_saft(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A,
            nav,
            units
        ));
        assert_eq!(
            super::ActiveSAFTs::<Test>::get(ASSET_A),
            vec![SAFTRecord::new(nav, units)]
        );
        assert_eq!(AssetIndex::index_total_asset_balance(ASSET_A), units);
        assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), nav);
        assert_eq!(AssetIndex::index_token_balance(&ADMIN_ACCOUNT_ID), nav);
        assert_eq!(AssetIndex::index_token_issuance(), nav);

        assert_eq!(
            super::ActiveSAFTs::<Test>::get(ASSET_A),
            vec![SAFTRecord::new(100, 20)]
        );
        // remove
        assert_ok!(SaftRegistry::remove_saft(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A,
            0
        ));
        assert_eq!(super::ActiveSAFTs::<Test>::get(ASSET_A), vec![]);
    });
}

#[test]
fn admin_can_add_then_update_saft() {
    new_test_ext().execute_with(|| {
        // add
        assert_ok!(SaftRegistry::add_saft(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A,
            100,
            20
        ));
        assert_eq!(
            super::ActiveSAFTs::<Test>::get(ASSET_A),
            vec![SAFTRecord::new(100, 20)]
        );
        // update
        assert_ok!(SaftRegistry::report_nav(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A,
            0,
            1000
        ));
        assert_eq!(
            super::ActiveSAFTs::<Test>::get(ASSET_A),
            vec![SAFTRecord::new(1000, 20)]
        );
    });
}

#[test]
fn admin_cannot_update_or_remove_invalid_index() {
    let expected_registry = vec![SAFTRecord::new(100, 20)];
    new_test_ext().execute_with(|| {
        // add
        assert_ok!(SaftRegistry::add_saft(
            Origin::signed(ADMIN_ACCOUNT_ID),
            ASSET_A,
            100,
            20
        ));
        assert_eq!(super::ActiveSAFTs::<Test>::get(ASSET_A), expected_registry);
        // try update invalid index
        assert_noop!(
            SaftRegistry::report_nav(
                Origin::signed(ADMIN_ACCOUNT_ID),
                ASSET_A,
                1, // index
                1000
            ),
            pallet::Error::<Test>::AssetIndexOutOfBounds
        );

        assert_eq!(super::ActiveSAFTs::<Test>::get(ASSET_A), expected_registry);

        // try remove invalid index
        assert_noop!(
            SaftRegistry::remove_saft(
                Origin::signed(ADMIN_ACCOUNT_ID),
                ASSET_A,
                1, // index
            ),
            pallet::Error::<Test>::AssetIndexOutOfBounds
        );

        assert_eq!(super::ActiveSAFTs::<Test>::get(ASSET_A), expected_registry);
    });
}
