// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks};
use frame_support::{
	assert_ok,
	dispatch::UnfilteredDispatchable,
	sp_runtime::traits::AccountIdConversion,
	sp_std::convert::TryInto,
	traits::{EnsureOrigin, Get},
};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use xcm::v0::MultiLocation;

use pallet_price_feed::PriceFeed;
use primitives::AssetAvailability;

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

	register_asset {
		let asset_id = 1337_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let availability = AssetAvailability::Saft;
		let call = Call::<T>::register_asset(
						asset_id,
						availability.clone(),
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

	deposit {
		// ASSET_A_ID
		let asset_id = 1_u32.into();
		let origin = T::AdminOrigin::successful_origin();
		let depositor = whitelisted_account::<T>("depositor", 0);
		let admin_deposit = 5u32.into();
		assert_ok!(AssetIndex::<T>::add_asset(origin.clone(), asset_id, 100u32.into(),MultiLocation::Null,admin_deposit
			));
		let units = 1_000u32.into();
		assert_ok!(T::Currency::deposit(asset_id, &depositor, units));
	}: _(
		RawOrigin::Signed(depositor.clone()),
		asset_id,
		units
	)
	verify {
		let expected_balance =
			T::PriceFeed::get_price(asset_id).unwrap().volume(units.into()).unwrap().try_into().ok().unwrap();
		assert_eq!(AssetIndex::<T>::index_token_balance(&depositor), expected_balance);
		assert_eq!(AssetIndex::<T>::index_token_issuance(), expected_balance + admin_deposit);
	}
}

fn x() {}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;

	use crate::mock::{ExtBuilder, Test};

	use super::*;

	#[test]
	fn add_asset() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(test_benchmark_add_asset::<Test>());
		});
	}

	#[test]
	fn set_metadata() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(test_benchmark_set_metadata::<Test>());
		});
	}

	#[test]
	fn deposit() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(test_benchmark_deposit::<Test>());
		});
	}

	#[test]
	fn register_asset() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(test_benchmark_register_asset::<Test>());
		});
	}
}
