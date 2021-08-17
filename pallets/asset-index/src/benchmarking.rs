// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks};
use frame_support::{
	assert_ok,
	dispatch::UnfilteredDispatchable,
	sp_runtime::{traits::AccountIdConversion, FixedPointNumber},
	traits::{EnsureOrigin, Get},
};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use pallet_price_feed::PriceFeed;
use primitives::{traits::NavProvider, AssetAvailability};
use xcm::v0::MultiLocation;

use crate::Pallet as AssetIndex;

use super::*;

const SEED: u32 = 0;

fn whitelisted_account<T: Config>(name: &'static str, counter: u32) -> T::AccountId {
	let acc = account(name, counter, SEED);
	whitelist_acc::<T>(&acc);
	acc
}

fn whitelist_acc<T: Config>(acc: &T::AccountId) {
	frame_benchmarking::benchmarking::add_to_whitelist(frame_system::Account::<T>::hashed_key_for(acc).into());
}

benchmarks! {
	add_asset {
		// ASSET_A_ID
		let asset_id: T::AssetId = 1_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let million = 1_000_000u32.into();
		let location = MultiLocation::Null;
		let call = Call::<T>::add_asset(
					asset_id,
					million,
					location.clone(),
					million
		);

	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(
			AssetIndex::<T>::assets(asset_id),
			Some(AssetAvailability::Liquid(location))
		);
	   assert_eq!(
			T::Currency::total_balance(asset_id, &T::TreasuryPalletId::get().into_account()),
			million
		);

	}

	complete_withdraw {
		let asset_id = 42_u32.into();
		let units = 100_u32.into();
		let tokens = 500_u32.into();
		let admin = T::AdminOrigin::successful_origin();
		let origin = whitelisted_account::<T>("origin", 0);

		// create liquid assets
		assert_ok!(<AssetIndex<T>>::add_asset(
			admin,
			asset_id,
			units,
			MultiLocation::Null,
			tokens
		));

		// start withdraw
		assert_ok!(<AssetIndex<T>>::withdraw(
			RawOrigin::Signed(origin.clone()).into(),
			<AssetIndex<T>>::index_token_balance(&origin),
		));

		assert_eq!(pallet::PendingWithdrawals::<T>::get(&origin).expect("pending withdrawals should be present").len(), 1);
	}: _(
		RawOrigin::Signed(origin.clone())
	) verify {
		assert_eq!(pallet::PendingWithdrawals::<T>::get(&origin).expect("pending withdrawals should be present").len(), 0);

		// TODO:
		//
		// Verify withdraw result
		//
		// assert_eq!(<AssetIndex<T>>::index_token_balance(&origin), );
	}

	deposit {
		// ASSET_A_ID
		let asset_id = 1_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let depositor = whitelisted_account::<T>("depositor", 0);
		let admin_deposit = 5u32.into();
		assert_ok!(AssetIndex::<T>::add_asset(origin, asset_id, 100u32.into(),MultiLocation::Null,admin_deposit
		));
		let units = 1_000u32.into();
		assert_ok!(T::Currency::deposit(asset_id, &depositor, units));
		let nav = AssetIndex::<T>::nav().unwrap();
	}: _(
		RawOrigin::Signed(depositor.clone()),
		asset_id,
		units
	) verify {
			let deposit_value = T::PriceFeed::get_price(asset_id).unwrap().checked_mul_int(units.into()).unwrap();
			let received = nav.reciprocal().unwrap().saturating_mul_int(deposit_value);
			assert_eq!(AssetIndex::<T>::index_token_balance(&depositor).into(), received);
	}

	remove_asset {
		let asset_id = 1_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let depositor = whitelisted_account::<T>("depositor", 0);
		let units: u32 = 100;
		let admin_deposit = 5u32.into();
		assert_ok!(AssetIndex::<T>::add_asset(origin.clone(), asset_id, units.into(), MultiLocation::Null,admin_deposit
		));

		// construct remove call
		let call = Call::<T>::remove_asset(asset_id, units.into(), None);
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(
			AssetIndex::<T>::assets(asset_id),
			None
		);
	}

	register_asset {
		let asset_id = 1337_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let availability = AssetAvailability::Saft;
		let call = Call::<T>::register_asset(
						asset_id,
						availability,
		);
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(
			AssetIndex::<T>::assets(asset_id),
			Some(AssetAvailability::Saft)
		);
	}

	set_metadata {
		let asset_id = 0_u32.into();
		let name = b"pint".to_vec();
		let symbol = b"pint".to_vec();
		let decimals = 8_u8;
		let origin = T::AdminOrigin::successful_origin();
		let call = Call::<T>::set_metadata(
						asset_id,
						name.clone(),
						symbol.clone(),
						decimals
		);
	}: { call.dispatch_bypass_filter(origin)? } verify {
		let metadata = Metadata::<T>::get(asset_id);
		assert_eq!(metadata.name.as_slice(), name.as_slice());
		assert_eq!(metadata.symbol.as_slice(), symbol.as_slice());
		assert_eq!(metadata.decimals, decimals);
	}

	withdraw {
		let asset_id = 42_u32.into();
		let units = 100_u32.into();
		let tokens = 500_u32.into();
		let admin = T::AdminOrigin::successful_origin();
		let origin = whitelisted_account::<T>("origin", 0);

		// create liquid assets
		assert_ok!(<AssetIndex<T>>::add_asset(
			admin,
			asset_id,
			units,
			MultiLocation::Null,
			tokens
		));
	}: _(
		RawOrigin::Signed(origin.clone()),
		<AssetIndex<T>>::index_token_balance(&origin)
	) verify {
		let pending =
			pallet::PendingWithdrawals::<T>::get(&origin).expect("pending withdrawals should be present");
		assert_eq!(pending.len(), 1);
	}

	unlock {
		// ASSET_A_ID
		let asset_id = 1_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let depositor = whitelisted_account::<T>("depositor", 0);
		let admin_deposit = 5u32.into();
		assert_ok!(AssetIndex::<T>::add_asset(origin, asset_id, 100u32.into(),MultiLocation::Null,admin_deposit
		));
		let units = 1_000u32.into();
		assert_ok!(T::Currency::deposit(asset_id, &depositor, units));
		let nav = AssetIndex::<T>::nav().unwrap();

		// deposit
		assert_ok!(<AssetIndex<T>>::deposit(RawOrigin::Signed(depositor.clone()).into(), asset_id, units));
	}: _(
		RawOrigin::Signed(depositor.clone())
	) verify {
		// TODO:
		//
		// verify unlock
	}
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;

	use crate::mock::{new_test_ext, Test};

	use super::*;

	#[test]
	fn add_asset() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_add_asset::<Test>());
		});
	}

	#[test]
	fn complete_withdraw() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_complete_withdraw::<Test>());
		});
	}

	#[test]
	fn set_metadata() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_metadata::<Test>());
		});
	}

	#[test]
	fn deposit() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_deposit::<Test>());
		});
	}

	#[test]
	fn register_asset() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_register_asset::<Test>());
		});
	}

	fn remove_asset() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_remove_asset::<Test>());
		});
	}

	#[test]
	fn unlock() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_unlock::<Test>());
		});
	}

	#[test]
	fn withdraw() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_withdraw::<Test>());
		});
	}
}
