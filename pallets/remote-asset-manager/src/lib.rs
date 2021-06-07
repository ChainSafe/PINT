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
    use cumulus_primitives_core::ParaId;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned, Convert, StaticLookup},
        sp_std::{prelude::*, result::Result},
        traits::Get,
    };
    use frame_system::pallet_prelude::*;
    use xcm::{
        opaque::v0::SendXcm,
        v0::{Error as XcmError, ExecuteXcm, MultiLocation, OriginKind, Xcm},
    };
    use xcm_executor::traits::Convert as XcmConvert;

    use crate::calls::proxy::{
        ProxyCall, ProxyCallEncoder, ProxyConfig, ProxyParams, ProxyState, ProxyType,
    };
    use crate::calls::staking::{StakingCall, StakingCallEncoder, StakingConfig};
    pub use crate::calls::*;
    pub use crate::traits::*;
    pub use crate::types::*;
    use frame_support::sp_runtime::traits::Zero;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

    // // A `pallet_staking` dispatchable on another chain
    // type PalletStakingCall<T: Config> = StakingCall<AccountIdFor<T>, T::Balance, AccountIdFor<T>>;

    // A `pallet_proxy` dispatchable on another chain
    // expects a `ProxyType` of u8 and blocknumber of u32
    type PalletProxyCall<T> =
        ProxyCall<AccountIdFor<T>, ProxyType, <T as frame_system::Config>::BlockNumber>;

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

        /// The encoder to use for encoding when transacting a `pallet_staking` Call
        type PalletStakingCallEncoder: StakingCallEncoder<
            <Self::Lookup as StaticLookup>::Source,
            Self::Balance,
            Self::AccountId,
            Context = Self::AssetId,
        >;

        /// The encoder to use for encoding when transacting a `pallet_proxy` Call
        type PalletProxyCallEncoder: ProxyCallEncoder<
            Self::AccountId,
            ProxyType,
            Self::BlockNumber,
            Context = Self::AssetId,
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

        /// Origin that is allowed to send cross chain messages on behalf of the PINT chain
        type AdminOrigin: EnsureOrigin<Self::Origin>;

        /// How to send an onward XCM message.
        type XcmSender: SendXcm;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    /// The config of `pallet_staking` in the runtime of the parachain.
    // TODO: Location as key?
    #[pallet::storage]
    pub type PalletStakingConfig<T: Config> = StorageMap<
        _,
        Twox64Concat,
        <T as Config>::AssetId,
        StakingConfig<T::AccountId, T::Balance>,
        OptionQuery,
    >;

    /// The config of `pallet_proxy` in the runtime of the parachain.
    // TODO: Location as key?
    #[pallet::storage]
    pub type PalletProxyConfig<T: Config> =
        StorageMap<_, Twox64Concat, <T as Config>::AssetId, ProxyConfig, OptionQuery>;

    /// Denotes the current state of proxies on a parachain for the PINT chain's account with the delegates being the second key in this map
    ///
    /// `location identifier` -> `delegate` -> `proxies`
    #[pallet::storage]
    pub type Proxies<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        Twox64Concat,
        AccountIdFor<T>,
        ProxyState,
        ValueQuery,
    >;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// key-value pairs for the `PalletStakingConfig` storage map
        pub staking_configs: Vec<(T::AssetId, StakingConfig<T::AccountId, T::Balance>)>,
        /// key-value pairs for the `PalletProxyConfig` storage map
        pub proxy_configs: Vec<(T::AssetId, ProxyConfig)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                staking_configs: Default::default(),
                proxy_configs: Default::default(),
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
        SentBondExtra,
        SentUnbond,
        /// Successfully sent a cross chain message to add a proxy. \[destination, delegate, proxy type\]
        SentAddProxy(MultiLocation, AccountIdFor<T>, ProxyType),
        /// Successfully sent a cross chain message to remove a proxy. \[destination, delegate, proxy type\]
        SentRemoveProxy(MultiLocation, AccountIdFor<T>, ProxyType),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Thrown when the proxy type was already set.
        AlreadyProxy,
        /// Thrown when the requested proxy type to removed was not added before.
        NoProxyFound,
        /// Thrown when the requested cross-chain call could not be encoded for the given location.
        NotEncodableForLocation,
        /// Thrown when no config was found for the requested location
        NoPalletConfigFound,
        /// Thrown when sending an Xcm `pallet_proxy::add_proxy` failed
        FailedToSendAddProxyXcm,
        /// Thrown when sending an Xcm `pallet_proxy::remove_proxy` failed
        FailedToSendRemoveProxyXcm,
    }

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
            origin: OriginFor<T>,
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

        /// Transacts a `pallet_proxy::Call::add_proxy` call to add a proxy on behalf of the PINT parachain's account on the target chain.
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn send_add_proxy(
            origin: OriginFor<T>,
            dest: MultiLocation,
            asset: T::AssetId,
            proxy_type: ProxyType,
            delegate: Option<AccountIdFor<T>>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin.clone())?;
            T::AdminOrigin::ensure_origin(origin)?;
            let delegate = delegate.unwrap_or(who);

            log::info!(target: "pint_xcm", "Attempting add_proxy {:?} on: {:?} with delegate {:?}", proxy_type, dest,  delegate);

            // ensures that the call is encodeable for the destination
            ensure!(
                T::PalletProxyCallEncoder::can_encode(&asset),
                Error::<T>::NotEncodableForLocation
            );

            let mut proxies = Proxies::<T>::get(&asset, &delegate);
            ensure!(!proxies.contains(&proxy_type), Error::<T>::AlreadyProxy);

            let config =
                PalletProxyConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

            let call = PalletProxyCall::<T>::AddProxy(ProxyParams {
                delegate: delegate.clone(),
                proxy_type,
                delay: T::BlockNumber::zero(),
            });
            let encoder = call.encoder::<T::PalletProxyCallEncoder>(&asset);

            let xcm = Xcm::Transact {
                origin_type: OriginKind::SovereignAccount,
                require_weight_at_most: config.weights.add_proxy,
                call: encoder
                    .encode_runtime_call(config.pallet_index)
                    .encode()
                    .into(),
            };

            let result = T::XcmSender::send_xcm(dest.clone(), xcm);
            log::info!(target: "pint_xcm", "sent pallet_proxy::add_proxy xcm: {:?} ",result);

            ensure!(result.is_ok(), Error::<T>::FailedToSendAddProxyXcm);

            // update the proxy for this delegate
            proxies.add(proxy_type);
            Proxies::<T>::insert(asset, delegate.clone(), proxies);

            Self::deposit_event(Event::SentAddProxy(dest, delegate, proxy_type));
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
