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
		assert!(super::ActiveSAFTs::<Test>::get(ASSET_A).is_empty(),);
	});
}

#[test]
fn admin_can_add_and_remove_saft() {
	let units = 20;
	let nav = 100;
	new_test_ext().execute_with(|| {
		// add
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, nav, units));
		assert_eq!(super::ActiveSAFTs::<Test>::get(ASSET_A), vec![SAFTRecord::new(nav, units)]);
		assert_eq!(AssetIndex::index_total_asset_balance(ASSET_A), units);
		assert_eq!(Balances::free_balance(ADMIN_ACCOUNT_ID), nav);
		assert_eq!(AssetIndex::index_token_balance(&ADMIN_ACCOUNT_ID), nav);
		assert_eq!(AssetIndex::index_token_issuance(), nav);

		assert_eq!(super::ActiveSAFTs::<Test>::get(ASSET_A), vec![SAFTRecord::new(nav, units)]);
		// remove
		assert_ok!(SaftRegistry::remove_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, 0));
		assert_eq!(super::ActiveSAFTs::<Test>::get(ASSET_A), vec![]);
	});
}

#[test]
fn admin_can_add_then_update_saft() {
	new_test_ext().execute_with(|| {
		// add
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, 100, 20));
		assert_eq!(super::ActiveSAFTs::<Test>::get(ASSET_A), vec![SAFTRecord::new(100, 20)]);
		// update
		assert_ok!(SaftRegistry::report_nav(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, 0, 1000));
		assert_eq!(super::ActiveSAFTs::<Test>::get(ASSET_A), vec![SAFTRecord::new(1000, 20)]);
	});
}

#[test]
fn admin_cannot_update_or_remove_invalid_index() {
	let expected_registry = vec![SAFTRecord::new(100, 20)];
	new_test_ext().execute_with(|| {
		// add
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, 100, 20));
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

#[test]
fn can_convert_to_liquid() {
	new_test_ext().execute_with(|| {
		// add
		assert_ok!(SaftRegistry::add_saft(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, 100, 20));
		assert!(!AssetIndex::is_liquid_asset(&ASSET_A));

		let location: MultiLocation = (Junction::Parent, Junction::Parachain(100)).into();
		assert_ok!(SaftRegistry::convert_to_liquid(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_A, location.clone()));
		assert_eq!(AssetIndex::native_asset_location(&ASSET_A), Some(location));
	});
}
