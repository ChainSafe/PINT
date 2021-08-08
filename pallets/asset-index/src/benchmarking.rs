// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::benchmarks;
use frame_support::{
	dispatch::UnfilteredDispatchable,
	sp_runtime::traits::AccountIdConversion,
	traits::{EnsureOrigin, Get},
};
use orml_traits::MultiCurrency;
use primitives::AssetAvailability;
use xcm::v0::MultiLocation;

use crate::Pallet as AssetIndex;

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
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{ExtBuilder, Test};
	use frame_support::assert_ok;

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
}
