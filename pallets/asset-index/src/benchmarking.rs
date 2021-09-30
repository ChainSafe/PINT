// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, vec};
use frame_support::{
	assert_ok,
	dispatch::UnfilteredDispatchable,
	sp_runtime::{
		traits::{AccountIdConversion, Bounded, One},
		FixedPointNumber,
	},
	traits::{EnsureOrigin, Get},
};
use orml_traits::MultiCurrency;
use pallet_price_feed::{PriceFeed, PriceFeedBenchmarks};
use primitives::{traits::NavProvider, AssetAvailability};
use xcm::v1::MultiLocation;

use crate::Pallet as AssetIndex;

use super::*;
use crate::types::DepositRange;

benchmarks! {
	add_asset {
		let asset_id :T::AssetId = T::try_convert(2u8).unwrap();
		let origin = T::AdminOrigin::successful_origin();
		let million = 1_000_000u32.into();
		let location = MultiLocation::Null;

		assert_ok!(
			AssetIndex::<T>::register_asset(
				origin.clone(),
				asset_id,
				AssetAvailability::Liquid(MultiLocation::Null)
			)
		);
		let call = Call::<T>::add_asset(
					asset_id,
					million,
					million
		);
		let balance = T::Currency::total_balance(asset_id, &T::TreasuryPalletId::get().into_account());
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(
			AssetIndex::<T>::assets(asset_id),
			Some(AssetAvailability::Liquid(location))
		);
	   assert_eq!(
			T::Currency::total_balance(asset_id, &T::TreasuryPalletId::get().into_account()),
			million + balance
		);
	}

	complete_withdraw {
		let asset_id : T::AssetId = T::try_convert(2u8).unwrap();
		let units = 100_u32.into();
		let tokens = 500_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let origin_account_id = T::AdminOrigin::ensure_origin(origin.clone()).unwrap();
		let deposit_units = 1000_u32.into();

		// create liquid assets
		assert_ok!(AssetIndex::<T>::register_asset(
			origin.clone(),
			asset_id,
			AssetAvailability::Liquid(MultiLocation::Null)
		));
		assert_ok!(AssetIndex::<T>::add_asset(
			origin.clone(),
			asset_id,
			units,
			tokens
		));

		// create price feed
		T::PriceFeedBenchmarks::create_feed(origin_account_id.clone(), asset_id).unwrap();

		// deposit some funds into the index from an user account
		assert_ok!(T::Currency::deposit(asset_id, &origin_account_id, deposit_units));
		assert_ok!(AssetIndex::<T>::deposit(origin.clone(), asset_id, deposit_units));

		// advance the block number so that the lock expires
		<frame_system::Pallet<T>>::set_block_number(
			<frame_system::Pallet<T>>::block_number()
				+ T::LockupPeriod::get()
				+ 1_u32.into(),
		);

		// start withdraw
		assert_ok!(AssetIndex::<T>::withdraw(
			origin.clone(),
			42_u32.into(),
		));
		let call = Call::<T>::complete_withdraw();
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(pallet::PendingWithdrawals::<T>::get(&origin_account_id), None);
	}

	deposit {
		let asset_id = T::try_convert(2u8).unwrap();
		let origin = T::AdminOrigin::successful_origin();
		let origin_account_id = T::AdminOrigin::ensure_origin(origin.clone()).unwrap();
		let admin_deposit = 1_000_000u32;
		let units = 1_000u32.into();

		assert_ok!(AssetIndex::<T>::register_asset(
			origin.clone(),
			asset_id,
			AssetAvailability::Liquid(MultiLocation::Null)
		));
		assert_ok!(AssetIndex::<T>::add_asset(
			origin.clone(),
			asset_id,
			100u32.into(),
			admin_deposit.into(),
		));

		let index_tokens = AssetIndex::<T>::index_token_balance(&origin_account_id).into();
		T::PriceFeedBenchmarks::create_feed(origin_account_id.clone(), asset_id).unwrap();
		assert_ok!(T::Currency::deposit(asset_id, &origin_account_id, units));

		// construct call
		let call = Call::<T>::deposit(asset_id, units);
	}: { call.dispatch_bypass_filter(origin)? } verify {
		let nav = AssetIndex::<T>::nav().unwrap();
		let deposit_value = T::PriceFeed::get_price(asset_id).unwrap().checked_mul_int(units.into()).unwrap();
		let received = nav.reciprocal().unwrap().saturating_mul_int(deposit_value);

		// NOTE:
		//
		// the result will be 0 or 1
		//
		// - 0 for tests
		// - 1 for benchmarks ( transaction fee )
		assert!(AssetIndex::<T>::index_token_balance(&origin_account_id).into() - (index_tokens + received) < 2);
	}

	// TODO:
	//
	// This extrinsic requires `remote-asset-manager`
	//
	// ----
	//
	// remove_asset {
	// 	let asset_id =  T::try_convert(2u8).unwrap();
	// 	let units = 100_u32.into();
	// 	let amount = 1_000u32.into();
	// 	let origin = T::AdminOrigin::successful_origin();
	// 	let origin_account_id = T::AdminOrigin::ensure_origin(origin.clone()).unwrap();
	// 	let receiver = whitelisted_account::<T>("receiver", 0);
	//
	// 	// create liquid assets
	// 	assert_ok!(<AssetIndex<T>>::add_asset(
	// 		origin.clone(),
	// 		asset_id,
	// 		units,
	// 		MultiLocation::Null,
	// 		amount
	// 	));
	//
	// 	// create price feed
	// 	T::PriceFeedBenchmarks::create_feed(origin_account_id.clone(), asset_id).unwrap();
	//
	// 	// construct call
	// 	let call = Call::<T>::remove_asset(asset_id, units, Some(receiver));
	// }: { call.dispatch_bypass_filter(origin.clone())? } verify {
	// 	assert_eq!(T::IndexToken::total_balance(&origin_account_id), 0u32.into());
	// }

	register_asset {
		let asset_id :T::AssetId =  T::try_convert(2u8).unwrap();
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
		let asset_id :T::AssetId =  T::try_convert(0u8).unwrap();
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

	set_deposit_range {
		let origin = T::AdminOrigin::successful_origin();
		let range = DepositRange {minimum : T::Balance::one(), maximum: T::Balance::max_value()};
		let call = Call::<T>::set_deposit_range(
						range.clone()
		);
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(range, IndexTokenDepositRange::<T>::get());
	}

	withdraw {
		let asset_id :T::AssetId =  T::try_convert(2u8).unwrap();
		let units = 100_u32.into();
		let tokens = 500_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let origin_account_id = T::AdminOrigin::ensure_origin(origin.clone()).unwrap();
		let deposit_units = 1_000_u32.into();

		// create liquid assets
		assert_ok!(AssetIndex::<T>::register_asset(
			origin.clone(),
			asset_id,
			AssetAvailability::Liquid(MultiLocation::Null)
		));
		assert_ok!(AssetIndex::<T>::add_asset(
			origin.clone(),
			asset_id,
			units,
			tokens
		));

		// create price feed
		T::PriceFeedBenchmarks::create_feed(origin_account_id.clone(), asset_id).unwrap();

		// deposit some funds into the index from an user account
		assert_ok!(T::Currency::deposit(asset_id, &origin_account_id, deposit_units));
		assert_ok!(AssetIndex::<T>::deposit(origin.clone(), asset_id, deposit_units));

		// advance the block number so that the lock expires
		<frame_system::Pallet<T>>::set_block_number(
			<frame_system::Pallet<T>>::block_number()
				+ T::LockupPeriod::get()
				+ 1_u32.into(),
		);

		let call = Call::<T>::withdraw(42_u32.into());
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(pallet::PendingWithdrawals::<T>::get(&origin_account_id).expect("pending withdrawals should be present").len(), 1);
	}

	unlock {
		let asset_id :T::AssetId =  T::try_convert(2u8).unwrap();
		let origin = T::AdminOrigin::successful_origin();
		let origin_account_id = T::AdminOrigin::ensure_origin(origin.clone()).unwrap();
		let amount = 500u32.into();
		let units = 100u32.into();

		// create price feed
		T::PriceFeedBenchmarks::create_feed(origin_account_id.clone(), asset_id).unwrap();

		assert_ok!(AssetIndex::<T>::register_asset(
			origin.clone(),
			asset_id,
			AssetAvailability::Liquid(MultiLocation::Null)
		));
		assert_ok!(AssetIndex::<T>::add_asset(origin.clone(), asset_id, units, amount));
		assert_ok!(T::Currency::deposit(asset_id, &origin_account_id, units));
		assert_ok!(AssetIndex::<T>::deposit(origin.clone(), asset_id, units));

		let call = Call::<T>::unlock();
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(pallet::IndexTokenLocks::<T>::get(&origin_account_id), vec![types::IndexTokenLock{
			locked: AssetIndex::<T>::index_token_equivalent(asset_id, units).unwrap(),
			end_block: frame_system::Pallet::<T>::block_number() + T::LockupPeriod::get() - 1u32.into()
		}]);
	}
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;

	use crate::mock::{new_test_ext, new_test_ext_from_genesis, Test};

	use super::*;

	#[test]
	fn add_asset() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_add_asset());
		});
	}

	#[test]
	fn complete_withdraw() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_complete_withdraw());
		});
	}

	#[test]
	fn set_metadata() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_set_metadata());
		});
	}

	#[test]
	fn set_deposit_range() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_set_deposit_range());
		});
	}

	#[test]
	fn deposit() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_deposit());
		});
	}

	#[test]
	fn register_asset() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_register_asset());
		});
	}

	// #[test]
	// fn remove_asset() {
	// 	new_test_ext().execute_with(|| {
	// 		assert_ok!(Pallet::<Test>::test_benchmark_remove_asset());
	// 	});
	// }

	#[test]
	fn unlock() {
		new_test_ext_from_genesis().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_unlock());
		});
	}

	#[test]
	fn withdraw() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_withdraw());
		});
	}
}
