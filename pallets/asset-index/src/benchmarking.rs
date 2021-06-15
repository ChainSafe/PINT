// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use crate::types::{AssetAvailability, IndexAssetData};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use xcm::v0::MultiLocation;

benchmarks! {
    add_asset {
        let asset_id = 42_u32.into();
        let caller: T::AccountId = whitelisted_caller();
        let million = 1_000_000u32.into();
        T::IndexToken::deposit_creating(&caller, million);
    }: _(
        RawOrigin::Signed(caller.clone()),
        Default::default(),
        asset_id,
        million,
        AssetAvailability::Liquid(MultiLocation::Null),
        million
    ) verify {
        assert_eq!(
            <Holdings<T>>::get(asset_id),
            Some(IndexAssetData::new(
                Default::default(),
                million,
                AssetAvailability::Liquid(MultiLocation::Null)
            ))
        );
    }
}
