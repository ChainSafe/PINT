// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use crate::types::{AssetAvailability, IndexAssetData};
use frame_benchmarking::{benchmarks, whitelisted_caller, Zero};
use frame_support::{assert_ok, traits::Currency};
use frame_system::RawOrigin;
use pallet_chainlink_feed::Pallet as ChainlinkFeed;
use xcm::v0::MultiLocation;

benchmarks! {
    add_asset {
        let asset_id = 42.into();
        let caller: T::AccountId = whitelisted_caller();
        let million = 1_000_000u32.into();
        T::IndexToken::deposit_creating(&caller, million);
    }: _(
        RawOrigin::Signed(caller.clone()),
        asset_id,
        million,
        AssetAvailability::Liquid(MultiLocation::Null),
        million
    ) verify {
        assert_eq!(
            <Holdings<T>>::get(asset_id),
            Some(IndexAssetData::new(
                million,
                AssetAvailability::Liquid(MultiLocation::Null)
            ))
        );
        // TODO:
        //
        // We are using `deposit_into_existing` currently now which means
        // we could not add asset to an account which has no balance
        //
        // assert_eq!(<Pallet<T>>::index_token_balance(&caller), total);

        // TODO:
        //
        // The value of `total_token_issuance` is not correct
        //
        // assert_eq!(<Pallet<T>>::index_token_issuance(), total);
    }

    // TODO:
    //
    // AssetPriceFeed not found
    //
    //
    // deposit {
    //     let feed_id = Zero::zero();
    //     let asset_id = 42.into();
    //     let round_id = 0.into();
    //     let caller: T::AccountId = whitelisted_caller();
    //     let million = 1_000_000u32.into();
    //     T::IndexToken::deposit_creating(&caller, million);
    //
    //     // submit price feed
    //     assert_ok!(<ChainlinkFeed<T>>::submit(
    //         RawOrigin::Root,
    //         feed_id,
    //         round_id,
    //         0,
    //     ));
    //
    //     // add_asset
    //     assert_ok!(<Pallet<T>>::add_asset(
    //         RawOrigin::Signed(caller.clone()).into(),
    //         asset_id,
    //         million,
    //         AssetAvailability::Liquid(MultiLocation::Null),
    //         million
    //     ));
    // }: _(
    //     RawOrigin::Signed(caller.clone()),
    //     asset_id,
    //     million
    // ) verify {
    //     // assert_eq!(
    //     //     <Holdings<T>>::get(asset_id),
    //     //     Some(IndexAssetData::new(
    //     //         million + million,
    //     //         AssetAvailability::Liquid(MultiLocation::Null)
    //     //     ))
    //     // );
    // }

    // TODO:
    //
    // Too complex and incompleted
    //
    // withdraw {
    //
    // }: _(
    //
    // ) verify {
    //
    // }
}
