// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Remote Asset Manager Pallet
//!
//! The Remote Asset Manager pallet provides capabilities to bond/unbond
//! and transfer assets on other chains.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
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
	use orml_traits::{MultiCurrency, XcmTransfer};
	use xcm::latest::{prelude::*, Error as XcmError, Result as XcmResult};

	use primitives::traits::{MaybeAssetIdConvert, RemoteAssetManager};
	use xcm_calls::{
		proxy::{ProxyCall, ProxyCallEncoder, ProxyConfig, ProxyParams, ProxyState, ProxyType, ProxyWeights},
		staking::{
			Bond, RewardDestination, StakingCall, StakingCallEncoder, StakingConfig, StakingLedger, StakingWeights,
		},
		PalletCall, PalletCallEncoder,
	};

	use crate::{
		traits::{BalanceMeter, StakingCap},
		types::{AssetLedger, StatemintConfig, XcmStakingMessageCount},
	};
	use xcm_calls::staking::UnlockChunk;

	// -------  Various type aliases

	type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
	type BalanceFor<T> = <T as Config>::Balance;

	/// The lookup source type configured for the chain's runtime
	type LookupSourceFor<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

	/// Simplified type for the `StakingLedger` that keeps track of the chains actively assets
	type StakingLedgerFor<T> =
		StakingLedger<LookupSourceFor<T>, <T as Config>::Balance, <T as frame_system::Config>::BlockNumber>;

	/// Simplified type for the staking config
	type StakingConfigFor<T> = StakingConfig<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
		<T as frame_system::Config>::BlockNumber,
	>;

	// A `pallet_staking` dispatchable on another chain
	type PalletStakingCall<T> = StakingCall<LookupSourceFor<T>, BalanceFor<T>, AccountIdFor<T>>;

	// A `pallet_proxy` dispatchable on another chain
	// expects a `ProxyType` of u8 and blocknumber of u32
	type PalletProxyCall<T> = ProxyCall<AccountIdFor<T>, ProxyType, <T as frame_system::Config>::BlockNumber>;

	#[pallet::config]
	pub trait Config: frame_system::Config + MaybeAssetIdConvert<u8, Self::AssetId> {
		/// The balance type for cross chain transfers
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Into<u128>;

		/// Asset Id that is used to identify different kinds of assets.
		type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize;

		/// Convert a `T::AssetId` to its relative `MultiLocation` identifier.
		type AssetIdConvert: Convert<Self::AssetId, Option<MultiLocation>>;

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

		/// Unbonding slashing spans for unbonding on the relaychain.
		#[pallet::constant]
		type AssetUnbondingSlashingSpans: Get<u32>;

		/// Determines the threshold amounts when operating with staked assets.
		type AssetStakingCap: StakingCap<Self::AssetId, Self::Balance>;

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

		/// The weight for this pallet's extrinsics.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	/// The config of `pallet_staking` in the runtime of the parachain.
	#[pallet::storage]
	#[pallet::getter(fn staking_config)]
	pub type PalletStakingConfig<T: Config> =
		StorageMap<_, Twox64Concat, <T as Config>::AssetId, StakingConfigFor<T>, OptionQuery>;

	/// The current state of PINT sovereign account bonding in `pallet_staking`.
	#[pallet::storage]
	#[pallet::getter(fn skating_ledger)]
	pub type PalletStakingLedger<T: Config> =
		StorageMap<_, Twox64Concat, <T as Config>::AssetId, StakingLedgerFor<T>, OptionQuery>;

	/// The ledger of deposited funds and funds about to be withdrawn
	/// This mirrors the `deposited` and `withdrawn` amounts from the asset index until
	/// Separating this from the `MultiCurrency` balances, which is used to represent any kind of
	/// funds (for example assets minted via governance proposal), ensures that this the real inflow
	/// of reserve backed assets.
	///
	/// NOTE: This expects the `deposited` funds to be backed by reserve deposits on the assets
	/// native location. For DOT for example this expects the `deposited` funds to be available on
	/// the Polkadot relay chain in the sovereign account of the PINT parachain.
	#[pallet::storage]
	#[pallet::getter(fn asset_balance)]
	pub type AssetBalance<T: Config> =
		StorageMap<_, Twox64Concat, <T as Config>::AssetId, AssetLedger<T::Balance>, ValueQuery>;

	/// The total number of xmc related messages sent to the `pallet_staking` pallet of the asset's
	/// location
	#[pallet::storage]
	#[pallet::getter(fn xcm_staking_count)]
	pub(super) type XcmStakingCount<T: Config> =
		StorageMap<_, Twox64Concat, T::AssetId, XcmStakingMessageCount, ValueQuery>;

	/// The config of `pallet_proxy` in the runtime of the parachain.
	#[pallet::storage]
	#[pallet::getter(fn proxy_config)]
	pub type PalletProxyConfig<T: Config> =
		StorageMap<_, Twox64Concat, <T as Config>::AssetId, ProxyConfig, OptionQuery>;

	/// Denotes the current state of proxies on a parachain for the PINT chain's
	/// account with the delegates being the second key in this map
	///
	/// `location identifier` -> `delegate` -> `proxies`
	#[pallet::storage]
	#[pallet::getter(fn proxies)]
	pub type Proxies<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, T::AssetId, Twox64Concat, AccountIdFor<T>, ProxyState, ValueQuery>;

	/// The extra weight for cross-chain XCM transfers.
	/// xcm_dest_weight: value: Weight
	#[pallet::storage]
	#[pallet::getter(fn xcm_dest_weight)]
	pub type XcmDestWeight<T: Config> = StorageValue<_, Weight, ValueQuery>;

	/// The config of the statemint parachain.
	///
	/// Provides information that is required when sending XCM calls to transfer PINT:,
	///  - `id`: The identifier of the corresponding PINT asset in the `pallet_assets` on the
	///    statemint parachain. Which is `u32` on statemint.
	///  - `parachain id`: the parachain of the statemint chain
	///  - `weights`: the weights to use for the call
	#[pallet::storage]
	#[pallet::getter(fn statemint_para_config)]
	pub type StatemintParaConfig<T> = StorageValue<_, StatemintConfig, OptionQuery>;

	#[pallet::genesis_config]
	#[allow(clippy::type_complexity)]
	pub struct GenesisConfig<T: Config> {
		/// key-value pairs for the `PalletStakingConfig` storage map
		pub staking_configs: Vec<(T::AssetId, StakingConfigFor<T>)>,
		/// key-value pairs for the `PalletProxyConfig` storage map
		pub proxy_configs: Vec<(T::AssetId, ProxyConfig)>,
		/// configures the statemint parachain
		pub statemint_config: Option<StatemintConfig>,
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
		/// amount\]
		SentUnbond(T::AssetId, T::Balance),
		/// Failed to send a bond_extra call. \[error, asset,
		/// amount\]
		ErrorSendingBondExtra(XcmError, T::AssetId, T::Balance),
		/// Failed to send a withdraw_unbonded call. \[error, asset,
		/// amount\]
		ErrorSendingWithdrawUnbonded(XcmError, T::AssetId, T::Balance),
		/// Successfully sent a cross chain message to bond extra. \[asset,
		/// Failed to send a unbond call. \[error, asset,
		/// amount\]
		ErrorSendingUnbond(XcmError, T::AssetId, T::Balance),
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
		/// Enabled xcm support for the statemint parachain.
		/// Transacting XCM calls to the statemint parachain is now possible
		StatemintTransactionsEnabled,
		/// Disabled xcm support for the statemint parachain.
		/// Transacting XCM calls to the statemint parachain is now frozen
		StatemintTransactionsDisabled,
		/// Set statemint config. \[statemint config\]
		SetStatemintConfig(StatemintConfig),
		/// Transfer to statemint succeeded. \[account, value\]
		StatemintTransfer(T::AccountId, T::Balance),
		/// The asset is frozen for XCM related operations.  \[asset id\]
		Frozen(T::AssetId),
		/// The asset was thawed for XCM related operations.  \[asset id\]
		Thawed(T::AssetId),
		/// A new weight for XCM transfers has been set.\[new_weight\]
		XcmDestWeightSet(Weight),
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
		/// PINT's stash is already bonded.
		AlreadyBonded,
		/// PINT's stash is not bonded yet with  [`bond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond).
		NotBonded,
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
		/// the minimum reserve balance
		InusufficientStash,
		/// Thrown if liquid asset has invalid chain location
		InvalidAssetChainLocation,
		/// Currency is not cross-chain transferable.
		NotCrossChainTransferableCurrency,
		/// Thrown if the given amount of PINT to send to statemint is too low
		MinimumStatemintTransfer,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Check for staking related XCM we need to get in to this block
		///
		/// This will compare the pending withdrawals against the free balance of each asset and
		/// determine whether an XCM unbond, bond_extra or withdrawal is in order.
		///
		/// Executing XCM on another chain is subject to fees that will be paid for with the balance
		/// which is under control of the PINT's account on that chain (the reserve held for all xcm
		/// deposits sent to PINT), therefore it is feasible that a certain amount of each asset is
		/// kept idle (not staked) at all times to mitigate the risk of lacking founds for the fees
		/// for unstaking. This reserve balance has fixed lower barrier and an additional buffer
		/// which reflects the NAV_asset of the asset in relation to the NAV of the index, since the
		/// amount of on asset a LP receives upon redeeming their PINT directly correlates with the
		/// value of each asset.
		///
		/// If an asset's chain supports staking (it was previously bonded with
		/// `pallet_staking::bond`) then we need to check what action is appropriate in this for
		/// this asset's staking pallet. Following states exists:
		///    - `Idle`: nothing to bond_extra, unbond or withdraw
		///    - `BondExtra`: new deposits were added to the PINT's parachain balance sheet and can
		///      now be bonded. This will also check if currently unbonded funds can be rebonded
		///      again instead.
		///    - `Unbond`: pending withdrawals reached a threshold were we need to unbund staked
		///      funds.
		///    - `Withdraw`: The bonding duration of an unlocking chunk is over and the funds are
		///      now safe to withdraw via `withdraw_unbonded`
		///
		/// The maximum number of separate xcm calls we send here is limited to the number of liquid
		/// assets with staking support.
		fn on_idle(now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			// check all assets with enabled cross chain staking support and a valid destination
			// TODO handle weight
			for (asset, config, dest) in PalletStakingConfig::<T>::iter()
				.filter_map(|(asset, config)| Self::asset_destination(asset).ok().map(|dest| (asset, config, dest)))
			{
				// consider only location which are already bonded
				if let Some(mut ledger) = PalletStakingLedger::<T>::get(&asset) {
					// derive the appropriate action based on the current balances
					let mut balances = AssetBalance::<T>::get(&asset);
					// cancel the deposits against the withdrawals since the last action
					balances.consolidate();

					// check if the additional funds would warrant a bond extra
					if balances.deposited >= T::AssetStakingCap::minimum_bond_extra(asset) {
						// TODO: could check against the currently unbonding balance and rebond

						// only if the free remote is above the reserve threshold
						if Self::ensure_free_stash(asset, balances.deposited).is_ok() {
							// attempt to send bond extra
							match Self::do_transact_bond_extra(&config, asset, balances.deposited, dest) {
								Ok(()) => {
									XcmStakingCount::<T>::mutate(asset, |count| {
										count.bond_extra = count.bond_extra.saturating_add(1)
									});
									Self::deposit_event(Event::SentBondExtra(asset, balances.deposited));
									ledger.bond_extra(balances.deposited);
									PalletStakingLedger::<T>::insert(&asset, ledger);
									balances.deposited = T::Balance::zero();
								}
								Err(err) => {
									Self::deposit_event(Event::ErrorSendingBondExtra(err, asset, balances.deposited));
								}
							}
						}
					} else if !balances.pending_redemption.is_zero() {
						// check if we need and able to unbond funds: only with we currently have enough active funds
						// and room for 1 more unlocking chunk
						if balances.pending_redemption < ledger.active.saturating_sub(config.minimum_balance) &&
							ledger.unlocking.len() < pallet_staking::MAX_UNLOCKING_CHUNKS
						{
							// attempt to send unbond
							match Self::do_transact_unbond(&config, asset, balances.pending_redemption, dest) {
								Ok(()) => {
									XcmStakingCount::<T>::mutate(asset, |count| {
										count.unbond = count.unbond.saturating_add(1)
									});
									Self::deposit_event(Event::SentUnbond(asset, balances.pending_redemption));

									// update the ledger
									let end = now.saturating_add(config.bonding_duration);
									ledger.active -= balances.pending_redemption;
									ledger.unlocking.push(UnlockChunk { value: balances.pending_redemption, end });
									PalletStakingLedger::<T>::insert(&asset, ledger);

									balances.pending_redemption = T::Balance::zero();
								}
								Err(err) => {
									Self::deposit_event(Event::ErrorSendingBondExtra(err, asset, balances.deposited));
								}
							}
						}

						// TODO automated withdrawing
					}
					// insert the updated balance back
					AssetBalance::<T>::insert(asset, balances);
				}
			}

			remaining_weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Send a `pallet_staking` [`bond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond) call to the location of the asset.
		///
		/// This will encode the `bond` call accordingly and dispatch to the
		/// location of the given asset. Limited to the council origin.
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn send_bond(
			origin: OriginFor<T>,
			asset: T::AssetId,
			controller: LookupSourceFor<T>,
			value: T::Balance,
			payee: RewardDestination<AccountIdFor<T>>,
		) -> DispatchResultWithPostInfo {
			T::AdminOrigin::ensure_origin(origin)?;
			if value.is_zero() {
				return Ok(().into());
			}

			let dest = Self::asset_destination(asset)?;

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

			let xcm = Self::wrap_call_into_xcm(
				encoder.encode_runtime_call(config.pallet_index).encode(),
				config.weights.bond,
				Self::xcm_dest_weight().into(),
			);

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
		/// Limited to the council origin
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

			let dest = Self::asset_destination(asset)?;

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

			let xcm = Self::wrap_call_into_xcm(
				encoder.encode_runtime_call(config.pallet_index).encode(),
				config.weights.add_proxy,
				Self::xcm_dest_weight().into(),
			);

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

		/// Disallow further remote transfers and halt remote staking.
		///
		/// - `asset_id`: The identifier of the asset to be frozen.
		///
		/// Callable by the admin origin.
		///
		/// Emits `Frozen`.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::freeze())]
		pub fn freeze(origin: OriginFor<T>, asset: T::AssetId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			PalletStakingConfig::<T>::try_mutate(&asset, |maybe_config| -> sp_std::result::Result<_, DispatchError> {
				let config = maybe_config.as_mut().ok_or(Error::<T>::NoPalletConfigFound)?;
				config.is_frozen = true;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::Frozen(asset));
			Ok(())
		}

		/// Allow remote transfers and enable remote staking again.
		///
		/// - `asset_id`: The identifier of the asset to be frozen.
		///
		/// Callable by the admin origin
		///
		/// Emits `Thawed`.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::thaw())]
		pub fn thaw(origin: OriginFor<T>, asset: T::AssetId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			PalletStakingConfig::<T>::try_mutate(&asset, |maybe_config| -> sp_std::result::Result<_, DispatchError> {
				let config = maybe_config.as_mut().ok_or(Error::<T>::NoPalletConfigFound)?;
				config.is_frozen = false;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::Thawed(asset));
			Ok(())
		}

		/// Sets the `xcm_dest_weight` for XCM transfers.
		///
		/// Callable by the admin origin
		///
		/// Parameters:
		/// - `xcm_dest_weight`: The new weight for XCM transfers.
		#[pallet::weight(< T as Config >::WeightInfo::set_xcm_dest_weight())]
		#[transactional]
		pub fn set_xcm_dest_weight(origin: OriginFor<T>, #[pallet::compact] xcm_dest_weight: Weight) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			XcmDestWeight::<T>::put(xcm_dest_weight);
			Self::deposit_event(Event::<T>::XcmDestWeightSet(xcm_dest_weight));
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
		pub fn set_statemint_config(origin: OriginFor<T>, config: StatemintConfig) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			StatemintParaConfig::<T>::put(config.clone());

			Self::deposit_event(Event::SetStatemintConfig(config));
			Ok(())
		}

		/// Attempts to transfer the given amount of index token to statemint.
		///
		/// This is executed as reserve based transfer, the given amount is transferred from the
		/// sender's balance to the account designated for the Statemint parachain. This amount is
		/// then send via XCM into the caller's account on the statemint parachain.
		#[pallet::weight(10_000)] // TODO: Set weights
		#[transactional]
		pub fn transfer_to_statemint(origin: OriginFor<T>, amount: T::Balance) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(amount >= T::MinimumStatemintTransferAmount::get(), Error::<T>::MinimumStatemintTransfer);

			let config = StatemintParaConfig::<T>::get().ok_or(Error::<T>::NoStatemintConfigFound)?;
			ensure!(config.enabled, Error::<T>::StatemintDisabled);

			T::XcmAssetTransfer::transfer_multi_asset(
				who.clone(),
				config.multi_asset(amount.into()),
				config.parahain_location(),
				Self::xcm_dest_weight().into(),
			)?;

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

			let dest = Self::asset_destination(asset)?;
			// ensures that the call is encodable for the destination
			ensure!(T::PalletProxyCallEncoder::can_encode(&asset), Error::<T>::NotEncodableForLocation);

			let config = PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

			let mut ledger = PalletStakingLedger::<T>::get(&asset).ok_or(Error::<T>::NotBonded)?;

			// ensures enough balance is available to bond extra
			Self::ensure_free_stash(asset, amount)?;

			Self::do_transact_bond_extra(&config, asset, amount, dest)
				.map_err(|_| Error::<T>::FailedToSendBondExtraXcm)?;

			ledger.bond_extra(amount);
			PalletStakingLedger::<T>::insert(&asset, ledger);

			Self::deposit_event(Event::SentBondExtra(asset, amount));
			Ok(())
		}

		/// Encodes the correct `Xcm::Transact` message and sends it to the given destination
		fn do_transact_bond_extra(
			config: &StakingConfigFor<T>,
			asset: T::AssetId,
			amount: T::Balance,
			dest: MultiLocation,
		) -> XcmResult {
			let call = PalletStakingCall::<T>::BondExtra(amount);
			let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

			let xcm = Self::wrap_call_into_xcm(
				encoder.encode_runtime_call(config.pallet_index).encode(),
				config.weights.bond_extra,
				Self::xcm_dest_weight().into(),
			);

			let result = T::XcmSender::send_xcm(dest, xcm);
			log::info!(target: "pint_xcm", "sent pallet_staking::bond_extra xcm: {:?} ",result);

			result.map_err(|e| e.into())
		}

		/// Sends an XCM [`unbond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.unbond) call
		///
		/// An `unbond` call must be signed by the controller account.
		pub fn do_send_unbond(asset: T::AssetId, amount: T::Balance) -> DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}

			let dest = Self::asset_destination(asset)?;
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

			Self::do_transact_unbond(&config, asset, amount, dest).map_err(|_| Error::<T>::FailedToSendUnbondXcm)?;

			// insert the unlock chunk with its deadline, on this system
			let end = frame_system::Pallet::<T>::block_number().saturating_add(config.bonding_duration);

			// move from active to unlocking
			ledger.active -= amount;
			ledger.unlocking.push(UnlockChunk { value: amount, end });

			PalletStakingLedger::<T>::insert(&asset, ledger);
			Self::deposit_event(Event::SentUnbond(asset, amount));
			Ok(())
		}

		/// Encodes the correct `Xcm::Transact` message and sends it to the given destination
		fn do_transact_unbond(
			config: &StakingConfigFor<T>,
			asset: T::AssetId,
			amount: T::Balance,
			dest: MultiLocation,
		) -> XcmResult {
			let call = PalletStakingCall::<T>::Unbond(amount);
			let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

			let xcm = Self::wrap_call_into_xcm(
				encoder.encode_runtime_call(config.pallet_index).encode(),
				config.weights.unbond,
				Self::xcm_dest_weight().into(),
			);

			let result = T::XcmSender::send_xcm(dest, xcm);
			log::info!(target: "pint_xcm", "sent pallet_staking::unbond xcm: {:?} ",result);
			result.map_err(|e| e.into())
		}

		/// Sends an XCM [`withdraw_unbonded`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.withdraw_unbonded) call
		///
		/// Remove any unlocked chunks from the `unlocking` queue.
		/// An `withdraw_unbonded` call must be signed by the controller
		/// account.
		/// This essentially gives the PINT's reserve account hold of the balance
		pub fn do_send_withdraw_unbonded(asset: T::AssetId) -> DispatchResult {
			let dest = Self::asset_destination(asset)?;

			// ensures that the call is encodable for the destination
			ensure!(T::PalletProxyCallEncoder::can_encode(&asset), Error::<T>::NotEncodableForLocation);

			// get the config for how staking is configured
			let config = PalletStakingConfig::<T>::get(&asset).ok_or(Error::<T>::NoPalletConfigFound)?;

			let mut ledger = PalletStakingLedger::<T>::get(&asset).ok_or(Error::<T>::NotBonded)?;

			// only controller account is allowed to send unbonded
			Self::ensure_staking_controller(ledger.controller.clone())?;

			// ensure that at least one chunk is withdrawable
			ensure!(
				ledger.consolidate_unlocked(frame_system::Pallet::<T>::block_number()),
				Error::<T>::NothingToWithdraw
			);

			let call = PalletStakingCall::<T>::WithdrawUnbonded(T::AssetUnbondingSlashingSpans::get());
			let encoder = call.encoder::<T::PalletStakingCallEncoder>(&asset);

			let xcm = Self::wrap_call_into_xcm(
				encoder.encode_runtime_call(config.pallet_index).encode(),
				config.weights.withdraw_unbonded,
				Self::xcm_dest_weight().into(),
			);

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

		/// The destination address of the asset's native location
		fn asset_destination(asset: T::AssetId) -> Result<MultiLocation, DispatchError> {
			let dest = T::AssetIdConvert::convert(asset).ok_or(Error::<T>::InvalidAssetChainLocation)?;
			Ok(dest)
		}

		/// Wrap the call into a Xcm instance.
		///  params:
		/// - call: The encoded call to be executed
		/// - fee: fee (in remote currency) used to buy the `weight` and `debt`.
		/// - require_weight_at_most: the weight limit used for the xcm transacted call.
		fn wrap_call_into_xcm(call: Vec<u8>, require_weight_at_most: Weight, _fee: u128) -> Xcm<()> {
			// let asset = MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungibility::Fungible(fee) };
			Xcm(vec![
				// WithdrawAsset(asset.clone().into()),
				// BuyExecution { fees: asset, weight_limit: Unlimited },
				Transact { origin_type: OriginKind::SovereignAccount, require_weight_at_most, call: call.into() },
				// DepositAsset {
				// 	assets: All.into(),
				// 	max_assets: u32::MAX,
				// 	beneficiary: MultiLocation { parents: 1, interior: X1(Parachain(T::SelfParaId::get().into())) },
				// },
			])
		}
	}

	impl<T: Config> RemoteAssetManager<T::AccountId, T::AssetId, T::Balance> for Pallet<T> {
		fn transfer_asset(recipient: T::AccountId, asset: T::AssetId, amount: T::Balance) -> DispatchResult {
			// asset's native chain location
			let dest: MultiLocation =
				T::AssetIdConvert::convert(asset).ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;

			// ensures the min stash is still available after the transfer
			Self::ensure_free_stash(asset, amount)?;
			T::XcmAssetTransfer::transfer(recipient, asset, amount, dest, 100_000_000)
		}

		fn deposit(asset: T::AssetId, amount: T::Balance) {
			AssetBalance::<T>::mutate(asset, |balance| balance.deposited = balance.deposited.saturating_add(amount))
		}

		fn announce_withdrawal(asset: T::AssetId, amount: T::Balance) {
			AssetBalance::<T>::mutate(asset, |balance| {
				balance.pending_redemption = balance.pending_redemption.saturating_add(amount)
			})
		}
	}

	impl<T: Config> BalanceMeter<T::Balance, T::AssetId> for Pallet<T> {
		/// This will return the total issuance of the given `asset` minus the
		/// amount that is currently unavailable due to staking
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
			T::AssetStakingCap::minimum_reserve_balance(*asset)
		}
	}

	/// Trait for the asset-index pallet extrinsic weights.
	pub trait WeightInfo {
		fn transfer() -> Weight;
		fn freeze() -> Weight;
		fn thaw() -> Weight;
		fn set_xcm_dest_weight() -> Weight;
	}

	/// For backwards compatibility and tests
	impl WeightInfo for () {
		fn transfer() -> Weight {
			Default::default()
		}
		fn freeze() -> Weight {
			Default::default()
		}
		fn thaw() -> Weight {
			Default::default()
		}
		fn set_xcm_dest_weight() -> Weight {
			Default::default()
		}
	}
}
