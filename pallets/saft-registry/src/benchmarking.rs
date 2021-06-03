// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::Origin;

benchmarks! {
    add_saft {
        let caller = whitelisted_caller();
        let asset_id: T::AssetId = 0.into();
    }: _(
        <Origin<T>>::Signed(caller),
        asset_id,
        100_u32.into(),
        20_u32.into()
    ) verify {
    }

    remove_saft {
    }: _(
        <Origin<T>>::Signed(whitelisted_caller()),
        0.into(),
        0
    ) verify {
    }

    report_nav {
    }: _(
        <Origin<T>>::Signed(whitelisted_caller()),
        0.into(),
        0,
        100_u32.into()
    ) verify {
    }
}
