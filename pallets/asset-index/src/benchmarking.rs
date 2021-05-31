//! benchmarks
//!
//! add_asset
//! deposit
//! withdraw
use super::*;
use crate::types::{AssetAvailability, IndexAssetData};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use xcm::v0::MultiLocation;

benchmarks! {
    add_asset {
        let caller: T::AccountId = whitelisted_caller();
        let million = 1_000_000u32.into();
        let asset_id = 42.into();
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
        assert_eq!(<Pallet<T>>::index_token_balance(&caller), million);
        assert_eq!(<Pallet<T>>::index_token_issuance(), million);
    }
}
