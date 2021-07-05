// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Remote Asset Manager Pallet
//!
//! The Remote Asset Manager pallet provides capabilities to bond/unbond
//! and transfer assets on other chains.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;

mod traits;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    pub use crate::traits::*;
    use cumulus_primitives_core::ParaId;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::Saturating,
        sp_runtime::traits::{
            AccountIdConversion, AtLeast32BitUnsigned, Convert, StaticLookup, Zero,
        },
        sp_std::prelude::*,
        traits::Get,
    };
    use frame_system::pallet_prelude::*;
    use primitives::traits::MultiAssetRegistry;
    use xcm::{
        opaque::v0::SendXcm,
        v0::{ExecuteXcm, MultiLocation, OriginKind, Xcm},
    };
    use xcm_executor::traits::Convert as XcmConvert;

    use orml_traits::{GetByKey, MultiCurrency};
    use xcm_assets::XcmAssetHandler;
    use xcm_calls::{
        proxy::{ProxyCall, ProxyCallEncoder, ProxyConfig, ProxyParams, ProxyState, ProxyType},
        staking::{
            Bond, RewardDestination, StakingBondState, StakingCall, StakingCallEncoder,
            StakingConfig,
        },
        PalletCall, PalletCallEncoder,
    };

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    type LookupSourceFor<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;
    type BalanceFor<T> = <T as Config>::Balance;

    // A `pallet_staking` dispatchable on another chain
    type PalletStakingCall<T> = StakingCall<LookupSourceFor<T>, BalanceFor<T>, AccountIdFor<T>>;

    // A `pallet_proxy` dispatchable on another chain
    // expects a `ProxyType` of u8 and blocknumber of u32
    type PalletProxyCall<T> =
        ProxyCall<AccountIdFor<T>, ProxyType, <T as frame_system::Config>::BlockNumber>;

    #[pallet::config]
    pub trait Config: frame_system::Config  {
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

        /// The minimum amount that should be held in stash (must remain unbonded)
        /// Withdrawals are only authorized if the updated stash balance does exceeds this.
        ///
        /// This must be at least the `ExistentialDeposit` as configured on the asset's
        /// native chain (e.g. DOT/Polkadot)
        type MinimumRemoteStashBalance: GetByKey<Self::AssetId, Self::Balance>;

        /// Currency type for deposit/withdraw xcm assets
        ///
        /// NOTE: it is assumed that the total issuance/total balance of an asset
        /// reflects the total balance of the PINT parachain account on the asset's native chain
        type Assets: MultiCurrency<
            Self::AccountId,
            CurrencyId = Self::AssetId,
            Balance = Self::Balance,
        >;

        /// Executor for cross chain messages.
        type XcmExecutor: ExecuteXcm<<Self as frame_system::Config>::Call>;

        /// The type that handles all the cross chain asset transfers
        type XcmAssets: XcmAssetHandler<Self::AccountId, Self::Balance, Self::AssetId>;

        /// Origin that is allowed to send cross chain messages on behalf of the PINT chain
        type AdminOrigin: EnsureOrigin<Self::Origin>;

        /// How to send an onward XCM message.
        type XcmSender: SendXcm;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Asset registry with all the locations
        type AssetRegistry: MultiAssetRegistry<Self::AssetId>;

        /// The weight for this pallet's extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    /// The config of `pallet_staking` in the runtime of the parachain.
    #[pallet::storage]
    pub type PalletStakingConfig<T: Config> = StorageMap<
        _,
        Twox64Concat,
        <T as Config>::AssetId,
        StakingConfig<T::AccountId, T::Balance>,
        OptionQuery,
    >;

    /// The current state of PINT sovereign account bonding in `pallet_staking`.
    #[pallet::storage]
    pub type PalletStakingBondState<T: Config> = StorageMap<
        _,
        Twox64Concat,
        <T as Config>::AssetId,
        StakingBondState<LookupSourceFor<T>, T::Balance>,
        OptionQuery,
    >;

    /// The config of `pallet_proxy` in the runtime of the parachain.
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
    #[allow(clippy::type_complexity)]
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
        fn build(&self) {
            self.staking_configs
                .iter()
                .for_each(|(id, config)| <PalletStakingConfig<T>>::insert(id, config));

            self.proxy_configs
                .iter()
                .for_each(|(id, config)| <PalletProxyConfig<T>>::insert(id, config));
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Successfully sent a cross chain message to bond. \[asset, controller, amount\]
        SentBond(T::AssetId, LookupSourceFor<T>, T::Balance),
        /// Successfully sent a cross chain message to bond extra. \[asset, amount\]
        SentBondExtra(T::AssetId, T::Balance),
        /// Successfully sent a cross chain message to bond extra. \[asset, amount, block number\]
        SentUnbond(T::AssetId, T::Balance, T::BlockNumber),
        /// Successfully sent a cross chain message to withdraw unbonded funds. \[asset, amount \]
        SentWithdrawUnbonded(T::AssetId, T::Balance),
        /// Successfully sent a cross chain message to add a proxy. \[asset, delegate, proxy type\]
        SentAddProxy(T::AssetId, AccountIdFor<T>, ProxyType),
        /// Successfully sent a cross chain message to remove a proxy. \[asset, delegate, proxy type\]
        SentRemoveProxy(T::AssetId, AccountIdFor<T>, ProxyType),
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
        /// Thrown when sending an Xcm `pallet_staking::bond` failed
        FailedToSendBondXcm,
        /// Thrown when sending an Xcm `pallet_staking::bond_extra` failed
        FailedToSendBondExtraXcm,
        /// Thrown when sending an Xcm `pallet_staking::unbond` failed
        FailedToSendUnbondXcm,
        /// Thrown when sending an Xcm `pallet_staking::withdraw_unbonded` failed
        FailedToSendWithdrawUnbondedXcm,
        /// Thrown when sending an Xcm `pallet_proxy::add_proxy` failed
        FailedToSendAddProxyXcm,
        /// Thrown when sending an Xcm `pallet_proxy::remove_proxy` failed
        FailedToSendRemoveProxyXcm,
        /// PINT's stash is already bonded.
        AlreadyBonded,
        /// PINT's stash is not bonded yet with  [`bond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond).
        NotBonded,
        /// Thrown when no location was found for the given asset.
        UnknownAsset,
        /// Thrown if the PINT parachain account is not allowed to executed pallet staking extrinsics that require controller origin
        NoControllerPermission,
        /// Thrown if the no more `unbond` chunks can be scheduled
        NoMoreUnbondingChunks,
        /// Thrown if no funds are currently unbonded
        NothingToWithdraw,
        /// Balance would fall below the minimum requirements for bond
        InsufficientBond,
        /// Thrown if the balance of the PINT parachain account would fall below the `MinimumRemoteStashBalance`
        InusufficientStash,
        /// Error occurred during XCM
        XcmError,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Send a `pallet_staking` [`bond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond) call to the location of the asset.
        ///
        /// This will encode the `bond` call accordingly and dispatch to the location of the given asset.
        /// Limited to the council origin
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn send_bond(
            origin: OriginFor<T>,
            asset: T::AssetId,
            controller: LookupSourceFor<T>,
            value: T::Balance,
            payee: RewardDestination<AccountIdFor<T>>,
        ) -> DispatchResultWithPostInfo {
            if value.is_zero() {
                return Ok(().into());
            }
            let _ = ensure_signed(origin.clone())?;
            T::AdminOrigin::ensure_origin(origin)?;

            let dest =
                T::AssetRegistry::native_asset_location(&asset).ok_or(Error::<T>::UnknownAsset)?;
            log::info!(target: "pint_xcm", "Attempting bond on: {:?} with controller {:?}", dest, controller, );

            // ensures that the call is encodable for the destination
            ensure!(
                T::PalletStakingCallEncoder::can_encode(&asset),
                Error::<T>::NotEncodableForLocation
            );
            // can't bond again
            ensure!(
                !PalletStakingBondState::<T>::contains_key(&asset),
                Error::<T>::AlreadyBonded
            );

            let config =
                PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

            // ensures enough balance is available to bond
            Self::ensure_stash(asset.clone(), value)?;

            let call = PalletStakingCall::<T>::Bond(Bond {
                controller: controller.clone(),
                value,
                payee,
            });
            let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

            let xcm = Xcm::Transact {
                origin_type: OriginKind::SovereignAccount,
                require_weight_at_most: config.weights.bond,
                call: encoder
                    .encode_runtime_call(config.pallet_index)
                    .encode()
                    .into(),
            };

            let result = T::XcmSender::send_xcm(dest, xcm);
            log::info!(target: "pint_xcm", "sent pallet_staking::bond xcm: {:?} ",result);
            ensure!(result.is_ok(), Error::<T>::FailedToSendBondXcm);

            // mark as bonded
            let state = StakingBondState {
                controller: controller.clone(),
                bonded: value,
                unbonded: Zero::zero(),
                unlocked_chunks: Zero::zero(),
            };
            PalletStakingBondState::<T>::insert(&asset, state);

            Self::deposit_event(Event::SentBond(asset, controller, value));
            Ok(().into())
        }

        /// Transacts a `pallet_proxy::Call::add_proxy` call to add a proxy on behalf
        /// of the PINT parachain's account on the target chain.
        ///
        /// Limitied to the council origin
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn send_add_proxy(
            origin: OriginFor<T>,
            asset: T::AssetId,
            proxy_type: ProxyType,
            delegate: Option<AccountIdFor<T>>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin.clone())?;
            T::AdminOrigin::ensure_origin(origin)?;
            let delegate = delegate.unwrap_or(who);

            let dest =
                T::AssetRegistry::native_asset_location(&asset).ok_or(Error::<T>::UnknownAsset)?;
            log::info!(target: "pint_xcm", "Attempting add_proxy {:?} on: {:?} with delegate {:?}", proxy_type, dest,  delegate);

            // ensures that the call is encodable for the destination
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

            let result = T::XcmSender::send_xcm(dest, xcm);
            log::info!(target: "pint_xcm", "sent pallet_proxy::add_proxy xcm: {:?} ",result);
            ensure!(result.is_ok(), Error::<T>::FailedToSendAddProxyXcm);

            // update the proxy for this delegate
            proxies.add(proxy_type);
            Proxies::<T>::insert(&asset, delegate.clone(), proxies);

            Self::deposit_event(Event::SentAddProxy(asset, delegate, proxy_type));
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Ensures that the given amount can be removed from PINT's sovereign account
        /// without falling below the configured `MinimumRemoteStashBalance`
        pub fn ensure_stash(asset: T::AssetId, amount: T::Balance) -> DispatchResult {
            let min_stash = T::MinimumRemoteStashBalance::get(&asset);
            ensure!(
                Self::stash_balance(asset).saturating_sub(amount) > min_stash,
                Error::<T>::InusufficientStash
            );
            Ok(())
        }

        /// The assumed balance of the PINT's parachain sovereign account on the asset's
        /// native chain that is not bonded
        pub fn stash_balance(asset: T::AssetId) -> T::Balance {
            let contributed = PalletStakingBondState::<T>::get(&asset)
                .map(|state| state.total_balance())
                .unwrap_or_else(Zero::zero);
            T::Assets::total_issuance(asset).saturating_sub(contributed)
        }

        /// Sends an XCM [`bond_extra`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond_extra) call
        pub fn do_send_bond_extra(asset: T::AssetId, amount: T::Balance) -> DispatchResult {
            if amount.is_zero() {
                return Ok(());
            }

            let dest =
                T::AssetRegistry::native_asset_location(&asset).ok_or(Error::<T>::UnknownAsset)?;
            // ensures that the call is encodable for the destination
            ensure!(
                T::PalletProxyCallEncoder::can_encode(&asset),
                Error::<T>::NotEncodableForLocation
            );

            let config =
                PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

            let mut state =
                PalletStakingBondState::<T>::get(&asset).ok_or(Error::<T>::NotBonded)?;

            // ensures enough balance is available to bond extra
            Self::ensure_stash(asset.clone(), amount)?;

            let call = PalletStakingCall::<T>::BondExtra(amount);
            let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

            let xcm = Xcm::Transact {
                origin_type: OriginKind::SovereignAccount,
                require_weight_at_most: config.weights.bond_extra,
                call: encoder
                    .encode_runtime_call(config.pallet_index)
                    .encode()
                    .into(),
            };

            let result = T::XcmSender::send_xcm(dest, xcm);
            log::info!(target: "pint_xcm", "sent pallet_staking::bond_extra xcm: {:?} ",result);
            ensure!(result.is_ok(), Error::<T>::FailedToSendBondExtraXcm);

            state.add_bond(amount);
            PalletStakingBondState::<T>::insert(&asset, state);

            Self::deposit_event(Event::SentBondExtra(asset, amount));
            Ok(())
        }

        /// Sends an XCM [`unbond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.unbond) call
        ///
        /// An `unbond` call must be signed by the controller account.
        pub fn do_send_unbond(asset: T::AssetId, amount: T::Balance) -> DispatchResult {
            if amount.is_zero() {
                return Ok(());
            }

            let dest =
                T::AssetRegistry::native_asset_location(&asset).ok_or(Error::<T>::UnknownAsset)?;
            // ensures that the call is encodable for the destination
            ensure!(
                T::PalletProxyCallEncoder::can_encode(&asset),
                Error::<T>::NotEncodableForLocation
            );
            let config =
                PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

            let mut state =
                PalletStakingBondState::<T>::get(&asset).ok_or(Error::<T>::NotBonded)?;

            // ensure that we have enough balance bonded to unbond
            ensure!(
                amount < state.bonded.saturating_sub(config.minimum_balance),
                Error::<T>::InsufficientBond
            );

            // Can't schedule unbond before withdrawing the unlocked funds first
            ensure!(
                (state.unlocked_chunks as usize) < pallet_staking::MAX_UNLOCKING_CHUNKS,
                Error::<T>::NoMoreUnbondingChunks
            );

            // ensure that the PINT parachain account is the controller, because unbond requires controller origin
            ensure!(
                <T as frame_system::Config>::Lookup::lookup(state.controller.clone())?
                    == T::SelfParaId::get().into_account(),
                Error::<T>::NoControllerPermission
            );

            let call = PalletStakingCall::<T>::Unbond(amount);
            let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

            let xcm = Xcm::Transact {
                origin_type: OriginKind::SovereignAccount,
                require_weight_at_most: config.weights.unbond,
                call: encoder
                    .encode_runtime_call(config.pallet_index)
                    .encode()
                    .into(),
            };

            let result = T::XcmSender::send_xcm(dest, xcm);
            log::info!(target: "pint_xcm", "sent pallet_staking::unbond xcm: {:?} ",result);
            ensure!(result.is_ok(), Error::<T>::FailedToSendUnbondXcm);

            // adjust the balances and keep track of new chunk
            state.unbond(amount);

            PalletStakingBondState::<T>::insert(&asset, state);
            Self::deposit_event(Event::SentUnbond(
                asset,
                amount,
                frame_system::Pallet::<T>::block_number(),
            ));
            Ok(())
        }

        /// Sends an XCM [`withdraw_unbonded`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.withdraw_unbonded) call
        ///
        /// Remove any unlocked chunks from the `unlocking` queue.
        /// An `withdraw_unbonded` call must be signed by the controller account.
        pub fn do_send_withdraw_unbonded(asset: T::AssetId) -> DispatchResult {
            let dest =
                T::AssetRegistry::native_asset_location(&asset).ok_or(Error::<T>::UnknownAsset)?;
            // ensures that the call is encodable for the destination
            ensure!(
                T::PalletProxyCallEncoder::can_encode(&asset),
                Error::<T>::NotEncodableForLocation
            );
            let config =
                PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

            let mut state =
                PalletStakingBondState::<T>::get(&asset).ok_or(Error::<T>::NotBonded)?;

            ensure!(state.unlocked_chunks > 0, Error::<T>::NothingToWithdraw);

            ensure!(
                <T as frame_system::Config>::Lookup::lookup(state.controller.clone())?
                    == T::SelfParaId::get().into_account(),
                Error::<T>::NoControllerPermission
            );

            let call = PalletStakingCall::<T>::WithdrawUnbonded(0);
            let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

            let xcm = Xcm::Transact {
                origin_type: OriginKind::SovereignAccount,
                require_weight_at_most: config.weights.withdraw_unbonded,
                call: encoder
                    .encode_runtime_call(config.pallet_index)
                    .encode()
                    .into(),
            };

            let result = T::XcmSender::send_xcm(dest, xcm);
            log::info!(target: "pint_xcm", "sent pallet_staking::withdraw_unbonded xcm: {:?} ",result);
            ensure!(result.is_ok(), Error::<T>::FailedToSendWithdrawUnbondedXcm);

            // adjust the balances and keep track of new chunk
            let unbonded = state.unbonded;
            state.unbonded = Zero::zero();
            state.unlocked_chunks = Zero::zero();
            PalletStakingBondState::<T>::insert(&asset, state);

            Self::deposit_event(Event::SentWithdrawUnbonded(asset, unbonded));
            Ok(())
        }
    }

    impl<T: Config> RemoteAssetManager<AccountIdFor<T>, T::AssetId, T::Balance> for Pallet<T> {
        fn transfer_asset(
            who: AccountIdFor<T>,
            asset: T::AssetId,
            amount: T::Balance,
        ) -> DispatchResult {
            // ensures the min stash is still available after the transfer
            Self::ensure_stash(asset.clone(), amount)?;

            let outcome = T::XcmAssets::execute_xcm_transfer(who, asset, amount)
                .map_err(|_| Error::<T>::XcmError)?;
            outcome
                .ensure_complete()
                .map_err(|_| Error::<T>::XcmError)?;
            Ok(())
        }

        fn bond(asset: T::AssetId, amount: T::Balance) -> DispatchResult {
            Self::do_send_bond_extra(asset, amount)
        }

        fn unbond(asset: T::AssetId, amount: T::Balance) -> DispatchResult {
            Self::do_send_unbond(asset, amount)
        }
    }

    /// Trait for the asset-index pallet extrinsic weights.
    pub trait WeightInfo {
        fn transfer() -> Weight;
    }

    /// For backwards compatibility and tests
    impl WeightInfo for () {
        fn transfer() -> Weight {
            Default::default()
        }
    }
}
