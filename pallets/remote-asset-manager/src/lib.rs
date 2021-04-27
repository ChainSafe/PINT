// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Remote Asset Manager Pallet
//!
//! The Remote Asset Manager pallet provides capabilities to bond/unbond
//! and transfer assets on other chains.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod traits;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::{AtLeast32BitUnsigned, Convert},
        traits::Get,
    };
    use frame_system::pallet_prelude::*;
    use xcm::v0::{MultiLocation};

    pub use crate::traits::RemoteAssetManager;
    use crate::traits::XcmHandler;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The balance type for cross chain transfers
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Into<u128>;

        /// Asset Id that is used to identify different kinds of assets.
        type AssetId: Parameter + Member + Clone;

        /// Convert a `T::AssetId` to its relative `MultiLocation` identifier.
        type AssetIdConvert: Convert<Self::AssetId, Option<MultiLocation>>;

        /// Convert `Self::Account` to `AccountId32`
        type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

        /// The native asset id
        #[pallet::constant]
        type SelfAssetId: Get<Self::AssetId>;

        /// The location of the chain itself
        #[pallet::constant]
        type SelfLocation: Get<MultiLocation>;

        /// Identifier for the relay chain's specific asset
        #[pallet::constant]
        type RelayChainAssetId: Get<Self::AssetId>;

        /// Used to convert accounts to locations
        type AccountIdConverter: Convert<MultiLocation, Option<AccountIdFor<Self>>>;

        /// Executor for cross chain messages.
        type XcmHandler: XcmHandler<AccountIdFor<Self>, Self::Call>;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn transfer(_origin: OriginFor<T>, _amount: T::Balance) -> DispatchResultWithPostInfo {
            Ok(().into())
        }
    }
}
