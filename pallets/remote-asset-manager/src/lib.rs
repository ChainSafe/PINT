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
    pub use crate::traits::RemoteAssetManager;
    use cumulus_pallet_xcm::{ensure_sibling_para, Origin as CumulusOrigin};
    use cumulus_primitives_core::ParaId;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned},
        traits::Get,
    };
    use frame_system::pallet_prelude::*;
    use xcm::v0::{ExecuteXcm, MultiLocation};
    use xcm_executor::traits::Convert;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_staking::Config {
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
        type AssetIdConvert: Convert<Self::AssetId, MultiLocation>;

        /// Convert `Self::Account` to `AccountId32`
        type AccountId32Convert: frame_support::sp_runtime::traits::Convert<
            Self::AccountId,
            [u8; 32],
        >;

        /// The native asset id
        #[pallet::constant]
        type SelfAssetId: Get<Self::AssetId>;

        /// The location of the chain itself
        #[pallet::constant]
        type SelfLocation: Get<MultiLocation>;

        /// Returns the parachain ID we are running with.
        #[pallet::constant]
        type SelfParaId: Get<ParaId>;

        /// Identifier for the relay chain's specific asset
        #[pallet::constant]
        type RelayChainAssetId: Get<Self::AssetId>;

        /// Executor for cross chain messages.
        type XcmExecutor: ExecuteXcm<<Self as frame_system::Config>::Call>;

        /// The overarching call type; we assume sibling chains use the same type.
        type Call: From<pallet_staking::Call<Self>> + Encode;

        /// The origin type that can be converted into a cumulus origin
        type Origin: From<<Self as frame_system::Config>::Origin>
            + Into<Result<CumulusOrigin, <Self as Config>::Origin>>;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    // TODO: store xcm query id with unbond procedures?

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        Attempted(xcm::v0::Outcome),
        SentBondExtra,
        SentUnbond,
    }

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

    impl<T: Config> Pallet<T> {
        /// Transacts a `bond_extra` extrinsic
        pub fn xcm_bond_extra(dest: MultiLocation) {
            log::debug!(target: "pint_xcm", "Attempting bond_extra  on: {:?} with pint para account {:?}",dest,  AccountIdConversion::<AccountIdFor<T>>::into_account(&T::SelfParaId::get()));


        }
    }

    impl<T: Config> RemoteAssetManager<AccountIdFor<T>, T::AssetId, T::Balance> for Pallet<T> {
        fn reserve_withdraw_and_deposit(
            _who: AccountIdFor<T>,
            _asset: T::AssetId,
            _amount: T::Balance,
        ) -> DispatchResult {
            todo!()
        }

        fn bond(_asset: <T as Config>::AssetId, _amount: <T as Config>::Balance) -> DispatchResult {
            Ok(())
        }

        fn unbond(_asset: T::AssetId, _amount: T::Balance) -> DispatchResult {
            Ok(())
        }

        fn withdraw_unbonded(
            _who: AccountIdFor<T>,
            _asset: T::AssetId,
            _amount: T::Balance,
        ) -> DispatchResult {
            Ok(())
        }
    }
}
