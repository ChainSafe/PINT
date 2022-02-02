// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Remote Proxy Manager Pallet
//!
//! The Remote Proxy Manager pallet handles proxies on remote locations

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
#[frame_support::pallet]
pub mod pallet {
	use cumulus_primitives_core::ParaId;
	use frame_support::{
		dispatch::DispatchResultWithPostInfo,
		pallet_prelude::*,
		sp_runtime::traits::{Convert, Zero},
		sp_std::{self, mem, prelude::*},
		traits::Get,
		transactional,
	};
	use frame_system::pallet_prelude::*;
	use xcm::latest::prelude::*;

	use primitives::traits::MaybeAssetIdConvert;
	use xcm_calls::{proxy::*, PalletCall, PalletCallEncoder};

	type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

	// A `pallet_proxy` dispatchable on another chain
	// expects a `ProxyType` of u8 and blocknumber of u32
	type PalletProxyCall<T> = ProxyCall<AccountIdFor<T>, ProxyType, <T as frame_system::Config>::BlockNumber>;

	#[pallet::config]
	pub trait Config: frame_system::Config + MaybeAssetIdConvert<u8, Self::AssetId> {
		/// Asset Id that is used to identify different kinds of assets.
		type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize;

		/// Convert a `T::AssetId` to its relative `MultiLocation` identifier.
		type AssetIdConvert: Convert<Self::AssetId, Option<MultiLocation>>;

		/// The encoder to use for encoding when transacting a `pallet_proxy`
		/// Call
		type PalletProxyCallEncoder: ProxyCallEncoder<
			Self::AccountId,
			ProxyType,
			Self::BlockNumber,
			Context = Self::AssetId,
		>;

		/// The location of the chain itself
		#[pallet::constant]
		type SelfLocation: Get<MultiLocation>;

		/// Returns the parachain ID we are running with.
		#[pallet::constant]
		type SelfParaId: Get<ParaId>;

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

	#[pallet::genesis_config]
	#[allow(clippy::type_complexity)]
	pub struct GenesisConfig<T: Config> {
		/// key-value pairs for the `PalletProxyConfig` storage map
		pub proxy_configs: Vec<(T::AssetId, ProxyConfig)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { proxy_configs: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.proxy_configs.iter().for_each(|(id, config)| PalletProxyConfig::<T>::insert(id, config));
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Successfully sent a cross chain message to add a proxy. \[asset,
		/// delegate, proxy type\]
		SentAddProxy(T::AssetId, AccountIdFor<T>, ProxyType),
		/// Successfully sent a cross chain message to remove a proxy. \[asset,
		/// delegate, proxy type\]
		SentRemoveProxy(T::AssetId, AccountIdFor<T>, ProxyType),
		/// Updated the proxy weights of an asset. \[asset, old weights, new
		/// weights\]
		UpdatedProxyCallWeights(T::AssetId, ProxyWeights, ProxyWeights),
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
		/// Thrown when sending a Xcm `pallet_proxy::add_proxy` failed
		FailedToSendAddProxyXcm,
		/// Thrown when sending a Xcm `pallet_proxy::remove_proxy` failed
		FailedToSendRemoveProxyXcm,
		/// Thrown when no config was found for the requested location
		NoPalletConfigFound,
		/// Thrown if liquid asset has invalid chain location
		InvalidAssetChainLocation,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transacts a `pallet_proxy::Call::add_proxy` call to add a proxy on
		/// behalf of the PINT parachain's account on the target chain.
		///
		/// Limited to the council origin
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn send_add_proxy(
			origin: OriginFor<T>,
			asset: T::AssetId,
			proxy_type: ProxyType,
			delegate: AccountIdFor<T>,
		) -> DispatchResultWithPostInfo {
			T::AdminOrigin::ensure_origin(origin)?;
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
	}

	impl<T: Config> Pallet<T> {
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
		fn wrap_call_into_xcm(call: Vec<u8>, require_weight_at_most: Weight, fee: u128) -> Xcm<()> {
			let asset = MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungibility::Fungible(fee) };
			Xcm(vec![
				WithdrawAsset(asset.clone().into()),
				BuyExecution { fees: asset, weight_limit: Unlimited },
				Transact { origin_type: OriginKind::SovereignAccount, require_weight_at_most, call: call.into() },
				DepositAsset {
					assets: All.into(),
					max_assets: u32::MAX,
					beneficiary: MultiLocation { parents: 1, interior: X1(Parachain(T::SelfParaId::get().into())) },
				},
			])
		}
	}

	/// Trait for the pallet extrinsic weights.
	pub trait WeightInfo {
		fn set_xcm_dest_weight() -> Weight;
	}

	/// For backwards compatibility and tests
	impl WeightInfo for () {
		fn set_xcm_dest_weight() -> Weight {
			Default::default()
		}
	}
}
