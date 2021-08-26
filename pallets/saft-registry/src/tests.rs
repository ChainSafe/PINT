// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::{mock::*, SAFTRecord};
use frame_support::{assert_noop, assert_ok};
use primitives::traits::MultiAssetRegistry;
use sp_runtime::traits::BadOrigin;
use xcm::v0::{Junction, MultiLocation};

const ASHLEY: AccountId = 0;
const ASSET_A: u32 = 0;
const ASSET_B: u32 = 1;

#[test]
fn non_admin_cannot_call_any_extrinsics() {
	new_test_ext().execute_with(|| {
		assert_noop!(SaftRegistry::add_saft(Origin::signed(ASHLEY), ASSET_A, 0, 0), BadOrigin);
		assert_noop!(SaftRegistry::remove_saft(Origin::signed(ASHLEY), ASSET_A, 0), BadOrigin);
		assert_noop!(SaftRegistry::report_nav(Origin::signed(ASHLEY), ASSET_A, 0, 0), BadOrigin);
	});
}

#[test]
fn native_asset_disallowed() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), PINTAssetId::get(), 100, 100),
			pallet_asset_index::Error::<Test>::NativeAssetDisallowed
		);
	});
}

#[test]
fn empty_deposit_does_nothing() {
	new_test_ext().execute_with(|| {
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, 0, 0));
		// counter is still at `0`
		assert_eq!(SaftRegistry::saft_counter(ASSET_A), 0);
	});
}

#[test]
fn admin_can_add_and_remove_saft() {
	let units = 20;
	let nav = 100;
	new_test_ext().execute_with(|| {
		// add
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, nav, units));
		let counter = SaftRegistry::saft_counter(ASSET_A);
		assert_eq!(counter, 1);
		let saft_id = counter - 1;
		assert_eq!(SaftRegistry::active_safts(ASSET_A, saft_id), Some(SAFTRecord::new(nav, units)));
		// total aggregated NAV
		assert_eq!(SaftRegistry::saft_nav(ASSET_A), nav);

		let additional_nav = 1337;
		let additional_units = 1345;
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, additional_nav, additional_units));
		assert_eq!(
			SaftRegistry::active_safts(ASSET_A, saft_id + 1),
			Some(SAFTRecord::new(additional_nav, additional_units))
		);

		let total_nav = nav + additional_nav;
		let total_units = units + additional_units;
		assert_eq!(AssetIndex::index_total_asset_balance(ASSET_A), total_units);
		assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), total_nav);
		assert_eq!(AssetIndex::index_token_balance(&ADMIN_ACCOUNT_ID), total_nav);
		assert_eq!(AssetIndex::index_token_issuance(), total_nav);

		assert_eq!(SaftRegistry::saft_nav(ASSET_A), total_nav);
		// remove
		assert_ok!(SaftRegistry::remove_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, saft_id));
		assert_eq!(SaftRegistry::active_safts(ASSET_A, saft_id), None);
		assert_eq!(SaftRegistry::saft_nav(ASSET_A), additional_nav);
	});
}

#[test]
fn admin_can_add_saft_twice() {
	let units = 20;
	let nav = 100;
	new_test_ext().execute_with(|| {
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, nav, units));
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_B, nav, units));
	});
}

#[test]
fn add_saft_depositing_index_tokens() {
	let units = 20;
	let nav = 100;
	new_test_ext().execute_with(|| {
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, nav, units));
		assert_eq!(AssetIndex::index_token_balance(&ADMIN_ACCOUNT_ID), nav);
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_B, nav, units));
		assert_eq!(AssetIndex::index_token_balance(&ADMIN_ACCOUNT_ID), nav * 2);
	});
}

#[test]
fn admin_can_add_then_update_saft() {
	new_test_ext().execute_with(|| {
		// add
		let nav = 100;
		let units = 20;
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, nav, units));
		assert_eq!(SaftRegistry::active_safts(ASSET_A, 0), Some(SAFTRecord::new(nav, units)));
		assert_eq!(SaftRegistry::saft_nav(ASSET_A), nav);
		// update
		assert_ok!(SaftRegistry::report_nav(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, 0, 1000));
		assert_eq!(SaftRegistry::active_safts(ASSET_A, 0), Some(SAFTRecord::new(1000, 20)));
		assert_eq!(SaftRegistry::saft_nav(ASSET_A), 1000);
	});
}

#[test]
fn admin_cannot_update_or_remove_invalid_index() {
	new_test_ext().execute_with(|| {
		// add
		let nav = 1337;
		let units = 13129;
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, nav, units));
		let saft_id = 0;
		assert_eq!(SaftRegistry::active_safts(ASSET_A, saft_id), Some(SAFTRecord::new(nav, units)));
		// try update invalid index
		assert_noop!(
			SaftRegistry::report_nav(
				Origin::signed(ADMIN_ACCOUNT_ID),
				ASSET_A,
				1, // invalid saft id
				1000
			),
			pallet::Error::<Test>::SAFTNotFound
		);

		assert_eq!(SaftRegistry::active_safts(ASSET_A, saft_id), Some(SAFTRecord::new(nav, units)));

		// try remove invalid index
		assert_noop!(
			SaftRegistry::remove_saft(
				Origin::signed(ADMIN_ACCOUNT_ID),
				ASSET_A,
				1, // invalid saft id
			),
			pallet::Error::<Test>::SAFTNotFound
		);
		assert_eq!(SaftRegistry::active_safts(ASSET_A, saft_id), Some(SAFTRecord::new(nav, units)));
	});
}

#[test]
fn can_convert_to_liquid() {
	new_test_ext().execute_with(|| {
		// add
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, 100, 20));
		assert!(!AssetIndex::is_liquid_asset(&ASSET_A));
		assert_eq!(SaftRegistry::active_safts(ASSET_A, 0), Some(SAFTRecord::new(100, 20)));

		let location: MultiLocation = (Junction::Parent, Junction::Parachain(100)).into();
		assert_ok!(SaftRegistry::convert_to_liquid(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, location.clone()));
		assert_eq!(AssetIndex::native_asset_location(&ASSET_A), Some(location));

		// everything is reset and purged
		assert_eq!(SaftRegistry::saft_counter(ASSET_A), 0);
		assert_eq!(SaftRegistry::saft_nav(ASSET_A), 0);
		assert_eq!(SaftRegistry::active_safts(ASSET_A, 0), None);
	});
}
