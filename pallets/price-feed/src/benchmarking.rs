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

use pallet_price_feed::PriceFeed;
use primitives::AssetAvailability;

use crate::Pallet as PriceFeed;

// benchmarks! {
// 	track_asset_price_feed {
// 	}: _(
// 		<Origin<T>>::Root,
// 		T::SelfAssetId::get(),
// 		Zero::zero()
// 	) verify {
// 		assert_noop!(
// 			<Pallet<T>>::get_price(T::SelfAssetId::get()),
// 			<Error<T>>::AssetPriceFeedNotFound
// 		);
// 	}
//
// 	untrack_asset_price_feed {
// 	}: _(
// 		<Origin<T>>::Root,
// 		T::SelfAssetId::get()
// 	) verify {
// 		assert_eq!(<AssetFeeds<T>>::get::<T::AssetId>(T::SelfAssetId::get()), None);
// 	}
// }
