// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks, vec};
use frame_support::{
	assert_ok,
	dispatch::UnfilteredDispatchable,
	sp_runtime::{traits::AccountIdConversion, FixedPointNumber},
	traits::{Currency as _, EnsureOrigin, Get},
};
use orml_traits::MultiCurrency;
use pallet_price_feed::{PriceFeed, PriceFeedBenchmarks};
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
		let asset_id :T::AssetId = T::try_convert(2u8).unwrap();
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
		let asset_id : T::AssetId = T::try_convert(2u8).unwrap();
		let units = 100_u32.into();
		let tokens = 500_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let origin_account_id = T::AdminOrigin::ensure_origin(origin.clone()).unwrap();
		let deposit_units = 1000_u32.into();

		// create liquid assets
		assert_ok!(<AssetIndex<T>>::add_asset(
			origin.clone(),
			asset_id,
			units,
			MultiLocation::Null,
			tokens
		));

		// create price feed
		T::PriceFeedBenchmarks::create_feed(origin_account_id.clone(), asset_id).unwrap();

		// deposit some funds into the index from an user account
		assert_ok!(T::Currency::deposit(asset_id, &origin_account_id, deposit_units));
		assert_ok!(<AssetIndex<T>>::deposit(origin.clone(), asset_id, deposit_units));

		// advance the block number so that the lock expires
		<frame_system::Pallet<T>>::set_block_number(
			<frame_system::Pallet<T>>::block_number()
				+ T::LockupPeriod::get()
				+ 1_u32.into(),
		);

		// start withdraw
		assert_ok!(<AssetIndex<T>>::withdraw(
			origin.clone(),
			42_u32.into(),
		));
		let call = Call::<T>::complete_withdraw();
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(pallet::PendingWithdrawals::<T>::get(&origin_account_id), None);
	}

	deposit {
		let asset_id = T::AssetId::try_convert(2u8).ok().unwrap();
		let origin = T::AdminOrigin::successful_origin();
		let origin_account_id = T::AdminOrigin::ensure_origin(origin.clone()).unwrap();
		let admin_deposit = 1_000_000u32.into();
		let units = 1_000u32.into();

		assert_ok!(AssetIndex::<T>::add_asset(
			origin.clone(),
			asset_id,
			100u32.into(),
			MultiLocation::Null,
			admin_deposit.into(),
		));

		let current_balance = AssetIndex::<T>::index_token_balance(&origin_account_id).into();
		T::PriceFeedBenchmarks::create_feed(origin_account_id.clone(), asset_id).unwrap();
		assert_ok!(T::Currency::deposit(asset_id, &origin_account_id, units));

		// construct call
		let call = Call::<T>::deposit(asset_id, units);
	}: { call.dispatch_bypass_filter(origin)? } verify {
		let nav = AssetIndex::<T>::nav().unwrap();
		let deposit_value = T::PriceFeed::get_price(asset_id).unwrap().checked_mul_int(units.into()).unwrap();
		let received = nav.reciprocal().unwrap().saturating_mul_int(deposit_value).saturating_add(1u128);

		// `-1` is about the transaction fee
		assert_eq!(AssetIndex::<T>::index_token_balance(&origin_account_id).into(), current_balance + received - 1u128);
	}

	remove_asset {
		let asset_id =  T::AssetId::try_from(2u8).ok().unwrap();
		let units = 100_u32.into();
		let amount = 1_000u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let origin_account_id = T::AdminOrigin::ensure_origin(origin.clone()).unwrap();
		let receiver = whitelisted_account::<T>("receiver", 0);

		// create liquid assets
		assert_ok!(<AssetIndex<T>>::add_asset(
			origin.clone(),
			asset_id,
			units,
			MultiLocation::Null,
			amount
		));

		// create price feed
		T::PriceFeedBenchmarks::create_feed(origin_account_id.clone(), asset_id).unwrap();

		// construct call
		let call = Call::<T>::remove_asset(asset_id, 1_u32.into(), Some(receiver));
	}: { call.dispatch_bypass_filter(origin.clone())? } verify {
		assert_eq!(T::IndexToken::total_balance(&origin_account_id), 0u32.into());
	}

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

	withdraw {
		let asset_id :T::AssetId =  T::try_convert(2u8).unwrap();
		let units = 100_u32.into();
		let tokens = 500_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let origin_account_id = T::AdminOrigin::ensure_origin(origin.clone()).unwrap();
		let deposit_units = 1_000_u32.into();

		// create liquid assets
		assert_ok!(<AssetIndex<T>>::add_asset(
			origin.clone(),
			asset_id,
			units,
			MultiLocation::Null,
			tokens
		));

		// create price feed
		T::PriceFeedBenchmarks::create_feed(origin_account_id.clone(), asset_id).unwrap();

		// deposit some funds into the index from an user account
		assert_ok!(T::Currency::deposit(asset_id, &origin_account_id, deposit_units));
		assert_ok!(<AssetIndex<T>>::deposit(origin.clone(), asset_id, deposit_units));

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

		assert_ok!(AssetIndex::<T>::add_asset(origin.clone(), asset_id, units, MultiLocation::Null, amount));
		assert_ok!(T::Currency::deposit(asset_id, &origin_account_id, units));
		assert_ok!(<AssetIndex<T>>::deposit(origin.clone(), asset_id, units));

		let call = Call::<T>::unlock();
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(<pallet::IndexTokenLocks<T>>::get(&origin_account_id), vec![types::IndexTokenLock{
			locked: <AssetIndex<T>>::index_token_equivalent(asset_id, units).unwrap(),
			end_block: <frame_system::Pallet<T>>::block_number() + T::LockupPeriod::get() - 1_u32.into(),
		}]);
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

	#[test]
	fn remove_asset() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_remove_asset());
		});
	}

	#[test]
	fn unlock() {
		new_test_ext().execute_with(|| {
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
