// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::benchmarks;
use frame_support::{assert_ok, dispatch::UnfilteredDispatchable, sp_std::convert::TryInto, traits::EnsureOrigin};
use primitives::traits::MaybeTryFrom;

use crate::Pallet as PriceFeed;

benchmarks! {
	map_asset_price_feed {
		let asset_id = T::AssetId::try_from(2u8).unwrap();
		let origin = T::AdminOrigin::successful_origin();
		let feed_id = 0u32.try_into().ok().unwrap();
		let call = Call::<T>::map_asset_price_feed(
					asset_id.clone(),
					feed_id
		);
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(
			PriceFeed::<T>::asset_feed(asset_id),
			Some(feed_id)
		);
	}

	unmap_asset_price_feed {
		let asset_id = T::AssetId::try_from(2u8).unwrap();
		let origin = T::AdminOrigin::successful_origin();
		let feed_id = 0u32.try_into().ok().unwrap();
		assert_ok!(PriceFeed::<T>::map_asset_price_feed(origin.clone(), asset_id.clone(), feed_id));
		let call = Call::<T>::unmap_asset_price_feed(
					asset_id.clone(),
		);
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(
			PriceFeed::<T>::asset_feed(asset_id),
			None
		);
	}
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;

	use crate::mock::{new_test_ext, FeedBuilder, Test};

	use super::*;

	#[test]
	fn map_asset_price_feed() {
		new_test_ext().execute_with(|| {
			assert_ok!(FeedBuilder::new().description(b"X".to_vec()).build_and_store());
			assert_ok!(Pallet::<Test>::test_benchmark_map_asset_price_feed());
		});
	}

	#[test]
	fn unmap_asset_price_feed() {
		new_test_ext().execute_with(|| {
			assert_ok!(FeedBuilder::new().description(b"X".to_vec()).build_and_store());
			assert_ok!(Pallet::<Test>::test_benchmark_unmap_asset_price_feed());
		});
	}
}
