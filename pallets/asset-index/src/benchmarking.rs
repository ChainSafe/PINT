// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use crate::types::AssetAvailability;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::sp_runtime::traits::AccountIdConversion;
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use xcm::v0::MultiLocation;

benchmarks! {
    add_asset {
        let asset_id = 42_u32.into();
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
            <Assets<T>>::get(asset_id),
            Some(

                AssetAvailability::Liquid(MultiLocation::Null)
            )
        );
       assert_eq!(
            T::Currency::total_balance(asset_id, &T::TreasuryPalletId::get().into_account())
            ,
            million
        );

    }

    set_metadata {
        let asset_id = 0_u32.into();
        let name = b"pint".to_vec();
        let symbol = b"pint".to_vec();
        let decimals = 8_u8;
    }: _(
        RawOrigin::Signed(whitelisted_caller()),
        asset_id,
        name.clone(),
        symbol.clone(),
        decimals
    ) verify {
        let metadata = <Metadata<T>>::get(asset_id);
        assert_eq!(metadata.name.as_slice(), name.as_slice());
        assert_eq!(metadata.symbol.as_slice(), symbol.as_slice());
        assert_eq!(metadata.decimals, decimals);
    }
}
