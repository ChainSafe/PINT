// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Remote Treasury Pallet
//!
//! Similar to the local treasury but manages remote treasury balances via XCM instead

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned, Convert, Zero},
		traits::Get,
		transactional, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::XcmTransfer;
	use xcm::{
		opaque::v1::{Junction, NetworkId},
		v1::MultiLocation,
	};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Origin that is allowed to manage the treasury balance and initiate
		/// XCM withdrawals
		type AdminOrigin: EnsureOrigin<Self::Origin>;
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

		/// PalletId used to generate the `AccountId` which holds the balance of the
		/// treasury.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The native asset id
		#[pallet::constant]
		type SelfAssetId: Get<Self::AssetId>;

		/// Identifier for the relay chain's specific asset
		#[pallet::constant]
		type RelayChainAssetId: Get<Self::AssetId>;

		/// The interface to Cross-chain transfer.
		type XcmAssetTransfer: XcmTransfer<Self::AccountId, Self::Balance, Self::AssetId>;

		/// Convert a `T::AssetId` to its relative `MultiLocation` identifier.
		type AssetIdConvert: Convert<Self::AssetId, Option<MultiLocation>>;

		/// Convert `Self::Account` to `AccountId32`
		type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

		/// The weight for this pallet's extrinsics.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Thrown if a transfer can't be executed because the given asset was not found or is the
		/// assets chain location is invalid
		InvalidAsset,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Admin successfully transferred some funds from the treasury to
		/// another account parameters. \[asset, recipient, amount\]
		Withdrawn(T::AssetId, T::AccountId, T::Balance),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::extra_constants]
	impl<T: Config> Pallet<T> {
		/// Returns the `AccountId` of the treasury account.
		pub fn treasury_account() -> T::AccountId {
			T::PalletId::get().into_account()
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer the amount of the given asset from the parachain's account into the recipient
		/// account on it's native location.
		///
		/// This will be a noop for `amount == 0`.
		///
		/// Only callable by the AdminOrigin.
		///
		/// Emits `Withdrawn`.
		#[pallet::weight(T::WeightInfo::transfer())]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			asset: T::AssetId,
			amount: T::Balance,
			recipient: T::AccountId,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			if asset == T::SelfAssetId::get() {
				return Err(Error::<T>::InvalidAsset.into());
			}

			if amount.is_zero() {
				return Ok(());
			}

			T::XcmAssetTransfer::transfer(
				Self::treasury_account(),
				asset,
				amount,
				Self::destination(asset, recipient.clone())?,
				// TODO: add actual weight
				100_000_000,
			)?;

			Self::deposit_event(Event::<T>::Withdrawn(asset, recipient, amount));

			Ok(())
		}

		/// Transfer the amount of the relay chain asset from the parachain's account into the
		/// recipient account on the relay chain.
		///
		/// This will be a noop for `amount == 0`.
		///
		/// Only callable by the AdminOrigin.
		///
		/// Emits `Withdrawn`.
		#[pallet::weight(T::WeightInfo::transfer_relaychain_asset())]
		#[transactional]
		pub fn transfer_relaychain_asset(
			origin: OriginFor<T>,
			amount: T::Balance,
			recipient: T::AccountId,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			if amount.is_zero() {
				return Ok(());
			}

			let asset = T::RelayChainAssetId::get();
			T::XcmAssetTransfer::transfer(
				Self::treasury_account(),
				asset,
				amount,
				Self::destination(asset, recipient.clone())?,
				// TODO: add actual weight
				100_000_000,
			)?;

			Self::deposit_event(Event::<T>::Withdrawn(asset, recipient, amount));

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// The `MultiLocation` destination of the recipient's account on the asset's native
		/// location.
		fn destination(asset: T::AssetId, recipient: T::AccountId) -> Result<MultiLocation, DispatchError> {
			let mut dest: MultiLocation = T::AssetIdConvert::convert(asset).ok_or(Error::<T>::InvalidAsset)?;
			let id = T::AccountId32Convert::convert(recipient);
			dest.push_interior(Junction::AccountId32 { network: NetworkId::Any, id })
				.map_err(|_| Error::<T>::InvalidAsset)?;
			Ok(dest)
		}
	}

	/// Trait for the remote treasury pallet extrinsic weights.
	pub trait WeightInfo {
		fn transfer() -> Weight;
		fn transfer_relaychain_asset() -> Weight;
	}

	/// For backwards compatibility and tests
	impl WeightInfo for () {
		fn transfer() -> Weight {
			Default::default()
		}
		fn transfer_relaychain_asset() -> Weight {
			Default::default()
		}
	}
}
