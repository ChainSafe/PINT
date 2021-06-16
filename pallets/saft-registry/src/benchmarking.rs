// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::benchmarks;
use frame_support::assert_ok;
use frame_system::Origin;

benchmarks! {
    add_saft {
    }: _(
        <Origin<T>>::Root,
        0_u32.into(),
        100_u32.into(),
        20_u32.into()
    ) verify {
        assert_eq!(
            <ActiveSAFTs<T>>::get::<T::AssetId>(0_u32.into()).as_slice(),
            &[SAFTRecord::new(100_u32.into(), 20_u32.into())]
        );
    }

    // TODO:
    //
    // require `remote_asset` of pallet asset-index
    //
    // https://github.com/ChainSafe/PINT/pull/73
    //
    // remove_saft {
    //     assert_ok!(<Pallet<T>>::add_saft(<Origin<T>>::Root.into(), 0.into(), 100_u32.into(), 20_u32.into()));
    // }: _(
    //     // <Origin<T>>::Signed(whitelisted_caller()),
    //     <Origin<T>>::Root,
    //     0.into(),
    //     0
    // ) verify {
    // }

    report_nav {
        assert_ok!(<Pallet<T>>::add_saft(
            <Origin<T>>::Root.into(),
            0.into(),
            100_u32.into(),
            20_u32.into(),
        ));
    }: _(
        <Origin<T>>::Root,
        0.into(),
        0,
        1000_u32.into()
    ) verify {
        assert_eq!(
            <ActiveSAFTs<T>>::get::<T::AssetId>(0_u32.into()).as_slice(),
            &[SAFTRecord::new(1000_u32.into(), 20_u32.into())]
        );
    }
}
