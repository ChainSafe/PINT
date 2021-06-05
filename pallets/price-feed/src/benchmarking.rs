// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{benchmarks, Zero};
use frame_support::{assert_noop, traits::Get};
use frame_system::Origin;

benchmarks! {
    track_asset_price_feed {
    }: _(
        <Origin<T>>::Root,
        T::SelfAssetId::get(),
        Zero::zero()
    ) verify {
        assert_noop!(
            <Pallet<T>>::get_price(T::SelfAssetId::get()),
            <Error<T>>::AssetPriceFeedNotFound
        );
    }

    untrack_asset_price_feed {
    }: _(
        <Origin<T>>::Root,
        T::SelfAssetId::get()
    ) verify {
        assert_eq!(<AssetFeeds<T>>::get::<T::AssetId>(T::SelfAssetId::get()), None);
    }
}
