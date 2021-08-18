// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Remote Asset Manager Pallet
//!
//! The Remote Asset Manager pallet provides capabilities to bond/unbond
//! and transfer assets on other chains.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod traits;
pub mod types;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
	use cumulus_primitives_core::ParaId;
	use frame_support::{
		dispatch::DispatchResultWithPostInfo,
		pallet_prelude::*,
		sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned, Convert, Saturating, StaticLookup, Zero},
		sp_std::{self, mem, prelude::*},
		traits::Get,
		transactional,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{location::Parse, GetByKey, MultiCurrency, XcmTransfer};
	use xcm::v0::{ExecuteXcm, MultiLocation, OriginKind, Outcome, SendXcm, Xcm};

	use primitives::traits::{MultiAssetRegistry, RemoteAssetManager, UnbondingOutcome};
	use xcm_calls::{
		assets::{AssetParams, AssetsCall, AssetsCallEncoder, AssetsWeights},
		proxy::{ProxyCall, ProxyCallEncoder, ProxyConfig, ProxyParams, ProxyState, ProxyType, ProxyWeights},
		staking::{
			Bond, RewardDestination, StakingCall, StakingCallEncoder, StakingConfig, StakingLedger, StakingWeights,
		},
		PalletCall, PalletCallEncoder,
	};

	use crate::{traits::BalanceMeter, types::StatemintConfig};
	use xcm_calls::staking::UnlockChunk;

	type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
	type LookupSourceFor<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;
	type BalanceFor<T> = <T as Config>::Balance;
	type AssetIdFor<T> = <T as Config>::AssetId;
	type StakingLedgerFor<T> =
		StakingLedger<LookupSourceFor<T>, <T as Config>::Balance, <T as frame_system::Config>::BlockNumber>;

	// A `pallet_staking` dispatchable on another chain
	type PalletStakingCall<T> = StakingCall<LookupSourceFor<T>, BalanceFor<T>, AccountIdFor<T>>;

	// A `pallet_proxy` dispatchable on another chain
	// expects a `ProxyType` of u8 and blocknumber of u32
	type PalletProxyCall<T> = ProxyCall<AccountIdFor<T>, ProxyType, <T as frame_system::Config>::BlockNumber>;

	// A `pallet_assets` dispatchable on another chain
	pub type PalletAssetsCall<T> = AssetsCall<AssetIdFor<T>, LookupSourceFor<T>, BalanceFor<T>>;

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
		type AssetId: Parameter + Member + Clone + Copy + MaybeSerializeDeserialize;

		/// Convert a `T::AssetId` to its relative `MultiLocation` identifier.
		type AssetIdConvert: Convert<Self::AssetId, Option<MultiLocation>>;

		/// Convert `Self::Account` to `AccountId32`
		type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

		/// The encoder to use for encoding when transacting a `pallet_staking`
		/// Call
		type PalletStakingCallEncoder: StakingCallEncoder<
			<Self::Lookup as StaticLookup>::Source,
			Self::Balance,
			Self::AccountId,
			Context = Self::AssetId,
		>;

		/// The encoder to use for encoding when transacting a `pallet_proxy`
		/// Call
		type PalletProxyCallEncoder: ProxyCallEncoder<
			Self::AccountId,
			ProxyType,
			Self::BlockNumber,
			Context = Self::AssetId,
		>;

		/// The encoder to use for encoding when transacting a `pallet_assets`
		/// Call
		type PalletAssetsCallEncoder: AssetsCallEncoder<
			Self::AssetId,
			<Self::Lookup as StaticLookup>::Source,
			Self::Balance,
			Context = Self::AssetId,
		>;

		/// The account that holds the PINT that were moved to the statemint
		/// parachain
		#[pallet::constant]
		type StatemintCustodian: Get<Self::AccountId>;

		/// Minimum amount that can be transferred via XCM to the statemint
		/// parachain
		#[pallet::constant]
		type MinimumStatemintTransferAmount: Get<Self::Balance>;

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

		/// The minimum amount that should be held in stash (must remain
		/// unbonded).
		/// Withdrawals are only authorized if the updated stash balance does
		/// exceeds this.
		///
		/// This must be at least the `ExistentialDeposit` as configured on the
		/// asset's native chain (e.g. DOT/Polkadot)
		type MinimumRemoteStashBalance: GetByKey<Self::AssetId, Self::Balance>;

		/// Currency type for deposit/withdraw xcm assets
		///
		/// NOTE: it is assumed that the total issuance/total balance of an
		/// asset reflects the total balance of the PINT parachain
		/// account on the asset's native chain
		type Assets: MultiCurrency<Self::AccountId, CurrencyId = Self::AssetId, Balance = Self::Balance>;

		/// Executor for cross chain messages.
		type XcmExecutor: ExecuteXcm<<Self as frame_system::Config>::Call>;

		/// The type that handles all the cross chain asset transfers
		type XcmAssetTransfer: XcmTransfer<Self::AccountId, Self::Balance, Self::AssetId>;

		/// Origin that is allowed to send cross chain messages on behalf of the
		/// PINT chain
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
		StakingConfig<T::AccountId, T::Balance, T::BlockNumber>,
		OptionQuery,
	>;

	/// The current state of PINT sovereign account bonding in `pallet_staking`.
	#[pallet::storage]
	pub type PalletStakingLedger<T: Config> =
		StorageMap<_, Twox64Concat, <T as Config>::AssetId, StakingLedgerFor<T>, OptionQuery>;

	/// The config of `pallet_proxy` in the runtime of the parachain.
	#[pallet::storage]
	pub type PalletProxyConfig<T: Config> =
		StorageMap<_, Twox64Concat, <T as Config>::AssetId, ProxyConfig, OptionQuery>;

	/// Denotes the current state of proxies on a parachain for the PINT chain's
	/// account with the delegates being the second key in this map
	///
	/// `location identifier` -> `delegate` -> `proxies`
	#[pallet::storage]
	pub type Proxies<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, T::AssetId, Twox64Concat, AccountIdFor<T>, ProxyState, ValueQuery>;

	/// The config of the statemint parachain and the internal `pallet_assets`
	///
	/// Provides information that is required when sending XCM Transact calls:,
	///  - `id`: The identifier of the corresponding PINT asset in the `pallet_assets` on the
	///    statemint parachain.
	///  - `parachain id`: the parachain of the statemint chain
	///  - `weights`: the weights to use for the call
	///  - `pallet_index`: the index of `pallet_assets` within the statemint parachain's runtime.
	///    This is required so that the call gets decoded correctly on the receiver end.
	///
	/// *NOTE*: It is assumed that the sovereign account of the PINT parachain
	/// has admin privileges of the statemint PINT asset in the `pallet_assets`
	/// on statemint.
	#[pallet::storage]
	pub type StatemintParaConfig<T: Config> = StorageValue<_, StatemintConfig<T::AssetId>, OptionQuery>;

	#[pallet::genesis_config]
	#[allow(clippy::type_complexity)]
	pub struct GenesisConfig<T: Config> {
		/// key-value pairs for the `PalletStakingConfig` storage map
		pub staking_configs: Vec<(T::AssetId, StakingConfig<T::AccountId, T::Balance, T::BlockNumber>)>,
		/// key-value pairs for the `PalletProxyConfig` storage map
		pub proxy_configs: Vec<(T::AssetId, ProxyConfig)>,
		/// configures the statemint parachain
		pub statemint_config: Option<StatemintConfig<T::AssetId>>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { staking_configs: Default::default(), proxy_configs: Default::default(), statemint_config: None }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.staking_configs.iter().for_each(|(id, config)| PalletStakingConfig::<T>::insert(id, config));

			self.proxy_configs.iter().for_each(|(id, config)| PalletProxyConfig::<T>::insert(id, config));

			if let Some(config) = self.statemint_config.clone() {
				StatemintParaConfig::<T>::put(config)
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Successfully sent a cross chain message to bond. \[asset,
		/// controller, amount\]
		SentBond(T::AssetId, LookupSourceFor<T>, T::Balance),
		/// Successfully sent a cross chain message to bond extra. \[asset,
		/// amount\]
		SentBondExtra(T::AssetId, T::Balance),
		/// Successfully sent a cross chain message to bond extra. \[asset,
		/// amount\]
		SentUnbond(T::AssetId, T::Balance),
		/// Successfully sent a cross chain message to withdraw unbonded funds.
		/// \[asset \]
		SentWithdrawUnbonded(T::AssetId),
		/// Successfully sent a cross chain message to add a proxy. \[asset,
		/// delegate, proxy type\]
		SentAddProxy(T::AssetId, AccountIdFor<T>, ProxyType),
		/// Successfully sent a cross chain message to remove a proxy. \[asset,
		/// delegate, proxy type\]
		SentRemoveProxy(T::AssetId, AccountIdFor<T>, ProxyType),
		/// Updated the staking weights of an asset. \[asset, old weights, new
		/// weights\]
		UpdatedStakingCallWeights(T::AssetId, StakingWeights, StakingWeights),
		/// Updated the proxy weights of an asset. \[asset, old weights, new
		/// weights\]
		UpdatedProxyCallWeights(T::AssetId, ProxyWeights, ProxyWeights),
		/// Updated the `pallet_assets` weights of the statemint config. \[old
		/// weights, new weights\]
		UpdatedStatemintCallWeights(AssetsWeights, AssetsWeights),
		/// Enabled xcm support for the statemint parachain.
		/// Transacting XCM calls to the statemint parachain is now possible
		StatemintTransactionsEnabled,
		/// Disabled xcm support for the statemint parachain.
		/// Transacting XCM calls to the statemint parachain is now frozen
		StatemintTransactionsDisabled,
		/// Set statemint config. \[statemint config\]
		SetStatemintConfig(StatemintConfig<T::AssetId>),
		/// Transfer to statemint succeeded. \[account, value\]
		StatemintTransfer(T::AccountId, T::Balance),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Thrown when the proxy type was already set.
		AlreadyProxy,
		/// Thrown when the requested proxy type to removed was not added
		/// before.
		NoProxyFound,
		/// Thrown when the requested cross-chain call could not be encoded for
		/// the given location.
		NotEncodableForLocation,
		/// Thrown when no config was found for the requested location
		NoPalletConfigFound,
		/// Thrown when no config was found for `statemint`
		NoStatemintConfigFound,
		/// Thrown when statemint support is currently disabled
		StatemintDisabled,
		/// Thrown when sending an Xcm `pallet_staking::bond` failed
		FailedToSendBondXcm,
		/// Thrown when sending an Xcm `pallet_staking::bond_extra` failed
		FailedToSendBondExtraXcm,
		/// Thrown when sending an Xcm `pallet_staking::unbond` failed
		FailedToSendUnbondXcm,
		/// Thrown when sending an Xcm `pallet_staking::withdraw_unbonded`
		/// failed
		FailedToSendWithdrawUnbondedXcm,
		/// Thrown when sending an Xcm `pallet_proxy::add_proxy` failed
		FailedToSendAddProxyXcm,
		/// Thrown when sending an Xcm `pallet_proxy::remove_proxy` failed
		FailedToSendRemoveProxyXcm,
		/// Thrown when sending an Xcm `pallet_assets::mint` failed
		FailedToSendAssetsMint,
		/// PINT's stash is already bonded.
		AlreadyBonded,
		/// PINT's stash is not bonded yet with  [`bond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond).
		NotBonded,
		/// Thrown when no location was found for the given asset.
		UnknownAsset,
		/// Thrown if the PINT parachain account is not allowed to executed
		/// pallet staking extrinsics that require controller origin
		NoControllerPermission,
		/// Thrown if the no more `unbond` chunks can be scheduled
		NoMoreUnbondingChunks,
		/// Thrown if no funds are currently unbonded
		NothingToWithdraw,
		/// Balance would fall below the minimum requirements for bond
		InsufficientBond,
		/// Thrown if the balance of the PINT parachain account would fall below
		/// the `MinimumRemoteStashBalance`
		InusufficientStash,
		/// Thrown if liquid asset has invalid chain location
		InvalidChainLocation,
		/// Currency is not cross-chain transferable.
		NotCrossChainTransferableCurrency,
		/// Thrown if the given amount of PINT to send to statemint is too low
		MinimumStatemintTransfer,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Send a `pallet_staking` [`bond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond) call to the location of the asset.
		///
		/// This will encode the `bond` call accordingly and dispatch to the
		/// location of the given asset. Limited to the council origin
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn send_bond(
			origin: OriginFor<T>,
			asset: T::AssetId,
			controller: LookupSourceFor<T>,
			value: T::Balance,
			payee: RewardDestination<AccountIdFor<T>>,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin.clone())?;
			T::AdminOrigin::ensure_origin(origin)?;
			if value.is_zero() {
				return Ok(().into());
			}

			let dest = T::AssetRegistry::native_asset_location(&asset)
				.ok_or(Error::<T>::UnknownAsset)?
				.chain_part()
				.ok_or(Error::<T>::InvalidChainLocation)?;
			log::info!(target: "pint_xcm", "Attempting bond on: {:?} with controller {:?}", dest, controller, );

			// ensures that the call is encodable for the destination
			ensure!(T::PalletStakingCallEncoder::can_encode(&asset), Error::<T>::NotEncodableForLocation);
			// can't bond again
			ensure!(!PalletStakingLedger::<T>::contains_key(&asset), Error::<T>::AlreadyBonded);

			let config = PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

			// ensures enough balance is available to bond
			Self::ensure_free_stash(asset, value)?;

			let call = PalletStakingCall::<T>::Bond(Bond { controller: controller.clone(), value, payee });
			let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

			let xcm = Xcm::Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: config.weights.bond,
				call: encoder.encode_runtime_call(config.pallet_index).encode().into(),
			};

			let result = T::XcmSender::send_xcm(dest, xcm);
			log::info!(target: "pint_xcm", "sent pallet_staking::bond xcm: {:?} ",result);
			ensure!(result.is_ok(), Error::<T>::FailedToSendBondXcm);

			// insert the ledger to mark as bonded
			let state =
				StakingLedger { controller: controller.clone(), active: value, total: value, unlocking: Vec::new() };
			PalletStakingLedger::<T>::insert(&asset, state);

			Self::deposit_event(Event::SentBond(asset, controller, value));
			Ok(().into())
		}

		/// Transacts a `pallet_proxy::Call::add_proxy` call to add a proxy on
		/// behalf of the PINT parachain's account on the target chain.
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

			let dest = T::AssetRegistry::native_asset_location(&asset)
				.ok_or(Error::<T>::UnknownAsset)?
				.chain_part()
				.ok_or(Error::<T>::InvalidChainLocation)?;
			log::info!(target: "pint_xcm", "Attempting add_proxy {:?} on: {:?} with delegate {:?}", proxy_type, dest,  delegate);

			// ensures that the call is encodable for the destination
			ensure!(T::PalletProxyCallEncoder::can_encode(&asset), Error::<T>::NotEncodableForLocation);

			let mut proxies = Proxies::<T>::get(&asset, &delegate);
			ensure!(!proxies.contains(&proxy_type), Error::<T>::AlreadyProxy);

			let config = PalletProxyConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

			let call = PalletProxyCall::<T>::AddProxy(ProxyParams {
				delegate: delegate.clone(),
				proxy_type,
				delay: T::BlockNumber::zero(),
			});
			let encoder = call.encoder::<T::PalletProxyCallEncoder>(&asset);

			let xcm = Xcm::Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: config.weights.add_proxy,
				call: encoder.encode_runtime_call(config.pallet_index).encode().into(),
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

		/// Updates the configured staking weights for the given asset.
		///
		/// Callable by the admin origin
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn update_staking_weights(
			origin: OriginFor<T>,
			asset: T::AssetId,
			weights: StakingWeights,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			let old_weights = PalletStakingConfig::<T>::try_mutate(
				&asset,
				|maybe_config| -> sp_std::result::Result<_, DispatchError> {
					let config = maybe_config.as_mut().ok_or(Error::<T>::NoPalletConfigFound)?;
					let old = mem::replace(&mut config.weights, weights.clone());
					Ok(old)
				},
			)?;

			Self::deposit_event(Event::UpdatedStakingCallWeights(asset, old_weights, weights));

			Ok(())
		}

		/// Updates the configured proxy weights for the given asset.
		///
		/// Callable by the admin origin
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn update_proxy_weights(origin: OriginFor<T>, asset: T::AssetId, weights: ProxyWeights) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			let old_weights = PalletProxyConfig::<T>::try_mutate(
				&asset,
				|maybe_config| -> sp_std::result::Result<_, DispatchError> {
					let config = maybe_config.as_mut().ok_or(Error::<T>::NoPalletConfigFound)?;
					let old = mem::replace(&mut config.weights, weights.clone());
					Ok(old)
				},
			)?;

			Self::deposit_event(Event::UpdatedProxyCallWeights(asset, old_weights, weights));

			Ok(())
		}

		/// Updates the configured assets weights the statemint parachain
		///
		/// Callable by the admin origin
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn update_statemint_assets_weights(origin: OriginFor<T>, weights: AssetsWeights) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			let old_weights =
				StatemintParaConfig::<T>::try_mutate(|maybe_config| -> sp_std::result::Result<_, DispatchError> {
					let config = maybe_config.as_mut().ok_or(Error::<T>::NoStatemintConfigFound)?;
					let old = mem::replace(&mut config.assets_config.weights, weights.clone());
					Ok(old)
				})?;

			Self::deposit_event(Event::UpdatedStatemintCallWeights(old_weights, weights));

			Ok(())
		}

		/// Enables XCM transactions for the statemint parachain, if configured.
		///
		/// This is a noop if it's already enabled
		/// Callable by the admin origin
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn enable_statemint_xcm(origin: OriginFor<T>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			// enable xcm support
			let was_enabled = Self::update_statemint_xcm_state(true)?;
			if !was_enabled {
				Self::deposit_event(Event::StatemintTransactionsEnabled);
			}
			Ok(())
		}

		/// Disables XCM transactions for the statemint parachain, if
		/// configured.
		///
		/// This is a noop if it's already disabled
		/// Callable by the admin origin
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn disable_statemint_xcm(origin: OriginFor<T>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			// enable xcm support
			let was_enabled = Self::update_statemint_xcm_state(false)?;
			if was_enabled {
				Self::deposit_event(Event::StatemintTransactionsDisabled);
			}
			Ok(())
		}

		/// Sets the statemint config.
		///
		/// Callable by the admin origin
		///
		/// *NOTE* It is assumed that the PINT parachain's local account on
		/// the statemint parachain (sibling account:
		/// `polkadot_parachain::primitives::Sibling`) has the permission to
		/// modify the statemint PINT asset.
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn set_statemint_config(origin: OriginFor<T>, config: StatemintConfig<T::AssetId>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			StatemintParaConfig::<T>::put(config.clone());

			Self::deposit_event(Event::SetStatemintConfig(config));
			Ok(())
		}

		/// Attempts to transfer the given amount of index token to statemint.
		///
		/// The given amount is transferred from the sender's balance to the
		/// `StatemintCustodian`. This amount is then minted via XCM into the
		/// caller's account via XCM on the statemint parachain.
		///
		/// *NOTE* this currently assumes successful minting on statemint,
		///  there is no response with the result: https://github.com/ChainSafe/PINT/issues/173
		///
		/// *NOTE* to interact with `pallet_assets` on statemint, an account
		/// must already exist for the sender with `ExistingDeposit`.
		#[pallet::weight(10_000)] // TODO: Set weights
		#[transactional]
		pub fn transfer_to_statemint(origin: OriginFor<T>, amount: T::Balance) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(amount >= T::MinimumStatemintTransferAmount::get(), Error::<T>::MinimumStatemintTransfer);

			let config = StatemintParaConfig::<T>::get().ok_or(Error::<T>::NoStatemintConfigFound)?;
			ensure!(config.enabled, Error::<T>::StatemintDisabled);

			let pint_asset = T::SelfAssetId::get();

			// transfer the given amount to the custodian
			T::Assets::transfer(pint_asset, &who, &T::StatemintCustodian::get(), amount)?;

			let dest = config.location();
			let beneficiary = T::Lookup::unlookup(who.clone());
			let call = PalletAssetsCall::<T>::Mint(AssetParams { id: config.pint_asset_id, beneficiary, amount });
			let encoder = call.encoder::<T::PalletAssetsCallEncoder>(&pint_asset);

			let xcm = Xcm::Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: config.assets_config.weights.mint,
				call: encoder.encode_runtime_call(config.assets_config.pallet_index).encode().into(),
			};

			let result = T::XcmSender::send_xcm(dest, xcm);
			log::info!(target: "pint_xcm", "sent statemint pallet_assets::mint xcm: {:?} ",result);
			ensure!(result.is_ok(), Error::<T>::FailedToSendAssetsMint);

			Self::deposit_event(Event::StatemintTransfer(who, amount));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Sets the `enabled` flag of the `StatemintConfig` to the given
		/// parameter.
		///
		/// Returns the replaced value
		fn update_statemint_xcm_state(state: bool) -> sp_std::result::Result<bool, DispatchError> {
			StatemintParaConfig::<T>::try_mutate(|maybe_config| -> sp_std::result::Result<_, DispatchError> {
				let config = maybe_config.as_mut().ok_or(Error::<T>::NoStatemintConfigFound)?;
				Ok(mem::replace(&mut config.enabled, state))
			})
		}

		/// Sends an XCM [`bond_extra`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond_extra) call
		pub fn do_send_bond_extra(asset: T::AssetId, amount: T::Balance) -> DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}

			let dest = T::AssetRegistry::native_asset_location(&asset)
				.ok_or(Error::<T>::UnknownAsset)?
				.chain_part()
				.ok_or(Error::<T>::InvalidChainLocation)?;
			// ensures that the call is encodable for the destination
			ensure!(T::PalletProxyCallEncoder::can_encode(&asset), Error::<T>::NotEncodableForLocation);

			let config = PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

			let mut state = PalletStakingLedger::<T>::get(&asset).ok_or(Error::<T>::NotBonded)?;

			// ensures enough balance is available to bond extra
			Self::ensure_free_stash(asset, amount)?;

			let call = PalletStakingCall::<T>::BondExtra(amount);
			let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

			let xcm = Xcm::Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: config.weights.bond_extra,
				call: encoder.encode_runtime_call(config.pallet_index).encode().into(),
			};

			let result = T::XcmSender::send_xcm(dest, xcm);
			log::info!(target: "pint_xcm", "sent pallet_staking::bond_extra xcm: {:?} ",result);
			ensure!(result.is_ok(), Error::<T>::FailedToSendBondExtraXcm);

			state.bond_extra(amount);
			PalletStakingLedger::<T>::insert(&asset, state);

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

			let dest = T::AssetRegistry::native_asset_location(&asset)
				.ok_or(Error::<T>::UnknownAsset)?
				.chain_part()
				.ok_or(Error::<T>::InvalidChainLocation)?;
			// ensures that the call is encodable for the destination
			ensure!(T::PalletProxyCallEncoder::can_encode(&asset), Error::<T>::NotEncodableForLocation);
			let config = PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

			let mut ledger = PalletStakingLedger::<T>::get(&asset).ok_or(Error::<T>::NotBonded)?;

			// ensure that we have enough balance bonded to unbond
			ensure!(amount < ledger.active.saturating_sub(config.minimum_balance), Error::<T>::InsufficientBond);

			// Can't schedule unbond before withdrawing the unlocked funds first
			ensure!(ledger.unlocking.len() < pallet_staking::MAX_UNLOCKING_CHUNKS, Error::<T>::NoMoreUnbondingChunks);

			// ensure that the PINT parachain account is the controller, because unbond
			// requires controller origin
			Self::ensure_staking_controller(ledger.controller.clone())?;

			let call = PalletStakingCall::<T>::Unbond(amount);
			let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

			let xcm = Xcm::Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: config.weights.unbond,
				call: encoder.encode_runtime_call(config.pallet_index).encode().into(),
			};

			let result = T::XcmSender::send_xcm(dest, xcm);
			log::info!(target: "pint_xcm", "sent pallet_staking::unbond xcm: {:?} ",result);
			ensure!(result.is_ok(), Error::<T>::FailedToSendUnbondXcm);

			// insert the unlock chunk with its deadline, on this system
			let end = frame_system::Pallet::<T>::block_number().saturating_add(config.bonding_duration);

			// move from active to unlocking
			ledger.active -= amount;
			ledger.unlocking.push(UnlockChunk { value: amount, end });

			PalletStakingLedger::<T>::insert(&asset, ledger);
			Self::deposit_event(Event::SentUnbond(asset, amount));
			Ok(())
		}

		/// Sends an XCM [`withdraw_unbonded`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.withdraw_unbonded) call
		///
		/// Remove any unlocked chunks from the `unlocking` queue.
		/// An `withdraw_unbonded` call must be signed by the controller
		/// account.
		/// This essentially gives the PNIT's sovereign hold of the balance
		pub fn do_send_withdraw_unbonded(asset: T::AssetId) -> DispatchResult {
			let dest = T::AssetRegistry::native_asset_location(&asset)
				.ok_or(Error::<T>::UnknownAsset)?
				.chain_part()
				.ok_or(Error::<T>::InvalidChainLocation)?;
			// ensures that the call is encodable for the destination
			ensure!(T::PalletProxyCallEncoder::can_encode(&asset), Error::<T>::NotEncodableForLocation);
			let config = PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

			let mut ledger = PalletStakingLedger::<T>::get(&asset).ok_or(Error::<T>::NotBonded)?;

			Self::ensure_staking_controller(ledger.controller.clone())?;

			// ensure that at least one chunk is withdrawable
			ensure!(
				ledger.consolidate_unlocked(frame_system::Pallet::<T>::block_number()),
				Error::<T>::NothingToWithdraw
			);

			// NOTE: this sets `num_slashing_spans` to 0, to not clear slashing metadata
			let call = PalletStakingCall::<T>::WithdrawUnbonded(0);
			let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

			let xcm = Xcm::Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: config.weights.withdraw_unbonded,
				call: encoder.encode_runtime_call(config.pallet_index).encode().into(),
			};

			let result = T::XcmSender::send_xcm(dest, xcm);
			log::info!(target: "pint_xcm", "sent pallet_staking::withdraw_unbonded xcm: {:?} ",result);
			ensure!(result.is_ok(), Error::<T>::FailedToSendWithdrawUnbondedXcm);

			PalletStakingLedger::<T>::insert(&asset, ledger);

			Self::deposit_event(Event::SentWithdrawUnbonded(asset));
			Ok(())
		}

		/// Ensures that the controller account of
		fn ensure_staking_controller(controller: LookupSourceFor<T>) -> DispatchResult {
			ensure!(
				<T as frame_system::Config>::Lookup::lookup(controller)? == T::SelfParaId::get().into_account(),
				Error::<T>::NoControllerPermission
			);
			Ok(())
		}
	}

	impl<T: Config> RemoteAssetManager<T::AccountId, T::AssetId, T::Balance> for Pallet<T> {
		fn transfer_asset(
			recipient: T::AccountId,
			asset: T::AssetId,
			amount: T::Balance,
		) -> sp_std::result::Result<Outcome, DispatchError> {
			// asset's native chain location
			let dest: MultiLocation =
				T::AssetIdConvert::convert(asset).ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;

			// ensures the min stash is still available after the transfer
			Self::ensure_free_stash(asset, amount)?;
			T::XcmAssetTransfer::transfer(recipient, asset, amount, dest, 100_000_000)
		}

		fn bond(asset: T::AssetId, amount: T::Balance) -> DispatchResult {
			Self::do_send_bond_extra(asset, amount)
		}

		fn unbond(_asset: T::AssetId, _amount: T::Balance) -> UnbondingOutcome {
			// TODO this will check the balance meter for the asset, if unbonding is
			// supported it will check the current stash
			// Self::do_send_unbond(asset, amount)
			UnbondingOutcome::NotSupported
		}
	}

	impl<T: Config> BalanceMeter<T::Balance, T::AssetId> for Pallet<T> {
		/// This will return the total issuance of the given `asset` minus the
		/// amount that is currently unvavailable due to staking
		fn free_stash_balance(asset: T::AssetId) -> T::Balance {
			// this is the amount that is currently reserved by staking, either `bonded` or
			// `unbonded` but not yet withdrawn
			let active = PalletStakingLedger::<T>::get(&asset).map(|ledger| ledger.total).unwrap_or_else(Zero::zero);
			// The total issuance is equal to the inflow of the remote asset via xcm which
			// is locked in the parachain's sovereign account on the asset's native chain
			T::Assets::total_issuance(asset).saturating_sub(active)
		}

		fn ensure_free_stash(asset: T::AssetId, amount: T::Balance) -> DispatchResult {
			let min_stash = Self::minimum_free_stash_balance(&asset);
			ensure!(Self::free_stash_balance(asset).saturating_sub(amount) > min_stash, Error::<T>::InusufficientStash);
			Ok(())
		}

		fn minimum_free_stash_balance(asset: &T::AssetId) -> T::Balance {
			T::MinimumRemoteStashBalance::get(asset)
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
