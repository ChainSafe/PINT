// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Remote Asset Manager Pallet
//!
//! The Remote Asset Manager pallet provides capabilities to bond/unbond
//! and transfer assets on other chains.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod calls;
mod traits;
mod types;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    use crate::calls::staking::StakingConfig;
    pub use crate::calls::*;
    pub use crate::traits::*;
    pub use crate::types::*;
    use cumulus_primitives_core::ParaId;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::{
            traits::{AccountIdConversion, AtLeast32BitUnsigned, Convert},
            MultiAddress,
        },
        sp_std::{prelude::*, result::Result},
        traits::Get,
    };
    use frame_system::pallet_prelude::*;
    use xcm::{
        opaque::v0::SendXcm,
        v0::{Error as XcmError, ExecuteXcm, MultiLocation, OriginKind, Xcm},
    };
    use xcm_executor::traits::Convert as XcmConvert;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

    // // A `pallet_staking` dispatchable on another chain
    // type PalletStakingCall<T> = StakingCall<AccountIdFor<T>, WrappedEncoded, WrappedEncoded>;

    // A `pallet_proxy` dispatchable on another chain
    // expects a `ProxyType` of u8 and blocknumber of u32
    type PalletProxyCall<T> = ProxyCall<AccountIdFor<T>, u8, u32>;

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
        type AssetId: Parameter + Member + Clone + MaybeSerializeDeserialize;

        /// Convert a `T::AssetId` to its relative `MultiLocation` identifier.
        type AssetIdConvert: XcmConvert<Self::AssetId, MultiLocation>;

        /// Convert `Self::Account` to `AccountId32`
        type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

        /// Encodes the local `Balance` type into the representation expected on the asset's parachain.
        type BalanceEncoder: EncodeWith<Self::AssetId, Self::Balance>;

        /// Encodes the local `AccountId` type into the representation expected on the asset's parachain.
        type LookupSourceEncoder: EncodeWith<Self::AssetId, Self::AccountId>;

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

        /// How to send an onward XCM message.
        type XcmSender: SendXcm;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    // TODO: store xcm query id with unbond procedures?

    /// The index of `pallet_staking` in the runtime of the parachain.
    // TODO: Location as key?
    #[pallet::storage]
    pub type AssetStakingConfig<T: Config> = StorageMap<
        _,
        Twox64Concat,
        <T as Config>::AssetId,
        // TODO: this assumes the same balance type, possibly conversion necessary
        StakingConfig<T::AccountId, T::Balance>,
        OptionQuery,
    >;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// key-value pairs for the `PalletStakingIndex` storage map
        pub staking_configs: Vec<(T::AssetId, StakingConfig<T::AccountId, T::Balance>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                staking_configs: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {}
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        SentBond(Result<(), XcmError>),
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
        pub fn transfer(
            origin: OriginFor<T>,
            dest: MultiLocation,
            asset: T::AssetId,
            amount: T::Balance,
            weight: u64,
        ) -> DispatchResultWithPostInfo {
            // let who = ensure_signed(origin)?;
            // let account: T::AccountId = T::SelfParaId::get().into_account();
            //
            // log::info!(target: "pint_xcm", "Attempting bond_nominate  on: {:?} with pint para account {:?}",dest,  account);
            //
            // let amount: WrappedEncoded = T::BalanceEncoder::encoded_with(&asset, amount)
            //     .expect("Should not fail")
            //     .into();
            //
            // #[derive(codec::Encode)]
            // enum TransferCall<AccountId, Value> {
            //     #[codec(index = 0)]
            //     Transfer(MultiAddress<AccountId, ()>, Value),
            // }
            //
            // let transfer = TransferCall::Transfer(who.into(), amount);
            //
            // let xcm = Xcm::Transact {
            //     origin_type: OriginKind::SovereignAccount,
            //     require_weight_at_most: weight,
            //     call: RuntimeCall {
            //         pallet_index: 4,
            //         call: transfer,
            //     }
            //     .encode()
            //     .into(),
            // };
            //
            // let result = T::XcmSender::send_xcm(dest, xcm);
            // log::info!(target: "pint_xcm", "Bond xcm send result: {:?} ",result);
            // Self::deposit_event(Event::SentBond(result));

            Ok(().into())
        }

        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn bond(
            _origin: OriginFor<T>,
            dest: MultiLocation,
            asset: T::AssetId,
            controller: T::Balance,
        ) -> DispatchResultWithPostInfo {
            // log::info!(target: "pint_xcm", "Attempting bond_nominate  on: {:?} with pint para account {:?}",dest,  AccountIdConversion::<AccountIdFor<T>>::into_account(&T::SelfParaId::get()));
            //
            // let weight = StakingWeights::polkadot();
            //
            // let account: T::AccountId = T::SelfParaId::get().into_account();
            // let bond = PalletStakingCall::<T>::Bond(
            //     // controller
            //     T::LookupSourceEncoder::encoded_with(&asset, account)
            //         .expect("Should not fail")
            //         .into(),
            //     // amount
            //     T::BalanceEncoder::encoded_with(&asset, controller)
            //         .expect("Should not fail")
            //         .into(),
            //     // rewards
            //     RewardDestination::Staked,
            // );
            //
            // let xcm = Xcm::Transact {
            //     origin_type: OriginKind::SovereignAccount,
            //     require_weight_at_most: weight.bond_extra * 2,
            //     call: bond
            //         .into_runtime_call(POLKADOT_PALLET_STAKING_INDEX)
            //         .encode()
            //         .into(),
            // };
            //
            // let result = T::XcmSender::send_xcm(dest, xcm);
            // log::info!(target: "pint_xcm", "Bond xcm send result: {:?} ",result);
            // Self::deposit_event(Event::SentBond(result));
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Transacts a `bond_extra` extrinsic
        pub fn xcm_bond_extra(
            dest: MultiLocation,
            asset: T::AssetId,
            amount: T::Balance,
        ) -> Result<(), XcmError> {
            // log::debug!(target: "pint_xcm", "Attempting bond_extra  on: {:?} with pint para account {:?}", dest,  AccountIdConversion::<AccountIdFor<T>>::into_account(&T::SelfParaId::get()));
            //
            // if let Some(config) = AssetStakingConfig::<T>::get(&asset) {
            //     let call =
            //         StakingCall::<T::AccountId, _, MultiAddress<T::AccountId, ()>>::BondExtra(
            //             T::BalanceEncoder::encoded_with(&asset, amount).expect("Should not fail"),
            //         );
            //     let xcm = Xcm::Transact {
            //         origin_type: OriginKind::SovereignAccount,
            //         require_weight_at_most: config.weights.bond_extra,
            //         call: call.into_runtime_call(config.pallet_index).encode().into(),
            //     };
            //
            //     Ok(T::XcmSender::send_xcm(dest, xcm)?)
            // } else {
            //     // nothing to bond
            //     Ok(())
            // }

            Ok(())
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
