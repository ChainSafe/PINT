// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Transaction Payment Pallet
//!
//! ## Overview
//!
//! Transaction payment pallet that supports charging for weight and tip in
//! different assets

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::Currency};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;

use primitives::*;

#[allow(clippy::unused_unit)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        // Origin that is allowed to manage the SAFTs
        type AdminOrigin: EnsureOrigin<Self::Origin>;

        /// Native asset id
        ///
        /// the actual received asset type as fee for treasury.
        /// Should be PINT
        #[pallet::constant]
        type NativeAssetId: Get<AssetId>;

        /// The currency type in which fees will be paid.
        type Currency: Currency<Self::AccountId> + Send + Sync;

        /// Currency to transfer, reserve/unreserve, lock/unlock assets
        type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = AssetId, Balance = Balance>;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The weight for this pallet's extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::metadata(T::AssetId = "AssetId", T::Balance = "Balance")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    /// Trait for the pallet extrinsic weights.
    pub trait WeightInfo {}

    /// For backwards compatibility and tests
    impl WeightInfo for () {}
}
