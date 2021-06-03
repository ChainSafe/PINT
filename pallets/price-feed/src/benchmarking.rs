// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{benchmarks, whitelisted_caller, Zero};
use frame_support::traits::Get;
use frame_system::Origin;

benchmarks! {
    track_asset_price_feed {
        let caller: T::AccountId = whitelisted_caller();
    }: _(
        <Origin<T>>::Signed(caller.clone()),
        T::SelfAssetId::get(),
        Zero::zero()
    ) verify {

    }

    untrack_asset_price_feed {
        let caller: T::AccountId = whitelisted_caller();
    }: _(
        <Origin<T>>::Signed(caller.clone()),
        T::SelfAssetId::get()
    ) verify {

    }
}
