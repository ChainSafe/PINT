// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
#[allow(clippy::large_enum_variant)]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::{
			traits::{AtLeast32BitUnsigned, CheckedAdd, One, Saturating, Zero},
			ArithmeticError,
		},
		sp_std::{self, convert::TryFrom, prelude::*, result::Result},
		transactional,
	};
	use frame_system::pallet_prelude::*;
	use primitives::{
		traits::{AssetRecorder, SaftRegistry},
		types::AssetAvailability,
		SAFTId,
	};
	use xcm::v0::MultiLocation;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// Origin that is allowed to manage the SAFTs
		type AdminOrigin: EnsureOrigin<Self::Origin>;
		type AssetRecorder: AssetRecorder<Self::AccountId, Self::AssetId, Self::Balance>;
		type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy;
		type AssetId: Parameter + Member + Copy + TryFrom<u8>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The weight for this pallet's extrinsics.
		type WeightInfo: WeightInfo;
	}

	/// Represents off-chain SAFT
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct SAFTRecord<Balance, NAV> {
		/// Net asset value of the SAFT, or the total value of `units`
		nav: NAV,
		/// How many units of the asset are included in the SAFT
		units: Balance,
	}

	impl<Balance, NAV> SAFTRecord<Balance, NAV> {
		pub fn new(nav: NAV, units: Balance) -> Self {
			Self { nav, units }
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Store a mapping (AssetId, SAFTId) -> SAFTRecord
	///
	/// Since `SAFTCounter(AssetId)` stores a running counter of `SAFTRecord`,
	/// this map is guaranteed to store less `SAFTRecord`s than the asset's
	/// `SAFTCounter`. If this maps stores a `None` value for a `SAFTId` lower
	/// than the counter, then this means the record was removed entirely.
	#[pallet::storage]
	#[pallet::getter(fn active_safts)]
	pub type ActiveSAFTs<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AssetId,
		Twox64Concat,
		SAFTId,
		SAFTRecord<T::Balance, T::Balance>,
		OptionQuery,
	>;

	/// A running counter used to determine the next SAFT id.
	#[pallet::storage]
	#[pallet::getter(fn saft_counter)]
	pub type SAFTCounter<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, SAFTId, ValueQuery>;

	/// Store a mapping (AssetId) -> NAV for each SAFT
	///
	/// Stores the aggregated NAV of all SAFTs, which is the sum the `nav` of
	/// all `SAFTRecord`s for each asset
	#[pallet::storage]
	#[pallet::getter(fn saft_nav)]
	pub type SAFTNetAssetValue<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, T::Balance, ValueQuery>;

	#[pallet::event]
	#[pallet::metadata(T::AssetId = "AssetId", T::Balance = "Balance")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new SAFT was added
		/// \[AssetId, AssetIndex\]
		SAFTAdded(T::AssetId, SAFTId),
		/// A SAFT was removed
		/// \[AssetId, AssetIndex\]
		SAFTRemoved(T::AssetId, SAFTId),
		/// The NAV for a SAFT was updated
		/// \[AssetId, AssetIndex, OldNav, NewNav\]
		NavUpdated(T::AssetId, SAFTId, T::Balance, T::Balance),
		/// A SAFT was converted into a liquid asset
		/// \[AssetId, MultiLocation\]
		ConvertedToLiquid(T::AssetId, MultiLocation),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No SAFT with the given saft id does not exist for the given AssetId
		SAFTNotFound,
		/// Thrown if the given asset was not a known SAFT.
		ExpectedSAFT,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Adds a new SAFT to the index and mints the given amount of
		/// IndexToken to reflect the added NAV of this SAFT.
		///
		/// Parameters:
		///   - `asset_id`: The identifier of the asset secured by the SAFT. If the asset
		///     identifying the SAFT's asset does not exist yet, it will get created.
		///   - `nav`: The NAV for the asset being secured by the SAFT at time of submission. This
		///     is essentially the amount of index token to mint to reflect the value the new SAFT
		///     secures.
		///   - `units`: Amount of assets being attested to
		/// the total value in index tokens the SAFT is worth. The `nav` of
		/// index token minted and awarded to the LP is specified as part of the
		/// associated proposal The id that was assigned to the SAFT when it was
		/// added with `add_saft`
		///
		/// Callable by the governance committee.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::add_saft())]
		#[transactional]
		pub fn add_saft(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			nav: T::Balance,
			units: T::Balance,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin.clone())?;
			let caller = ensure_signed(origin)?;
			if units.is_zero() {
				return Ok(());
			}
			// mint SAFT units into the index and credit the caller's account with PINT
			T::AssetRecorder::add_saft(&caller, asset_id, units, nav)?;

			// keep track of total nav
			SAFTNetAssetValue::<T>::try_mutate(asset_id, |val| -> Result<_, DispatchError> {
				*val = val.checked_add(&nav).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;

			// Determine the next id for the SAFT
			let saft_id = SAFTCounter::<T>::try_mutate(asset_id, |counter| -> Result<_, DispatchError> {
				let id = *counter;
				*counter = counter.checked_add(SAFTId::one()).ok_or(ArithmeticError::Overflow)?;
				Ok(id)
			})?;

			// insert the new record
			ActiveSAFTs::<T>::insert(asset_id, saft_id, SAFTRecord::new(nav, units));
			Self::deposit_event(Event::<T>::SAFTAdded(asset_id, saft_id));
			Ok(())
		}

		/// Removes the SAFT from the registry by purging it from the
		/// `ActiveSAFTs` storage.
		///
		/// The change in NAV will also be reflected in the index.
		///
		/// Parameters:
		///   - `asset_id`: The identifier of the asset of the SAFT
		///   - `saft_id`: The id that was assigned to the SAFT when it was added with `add_saft`
		///
		/// Callable by the governance committee.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::remove_saft())]
		#[transactional]
		pub fn remove_saft(origin: OriginFor<T>, asset_id: T::AssetId, saft_id: SAFTId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin.clone())?;
			let who = ensure_signed(origin)?;

			// remove the SAFT record
			let saft = ActiveSAFTs::<T>::take(asset_id, saft_id).ok_or(Error::<T>::SAFTNotFound)?;

			// reflect the change in NAV
			T::AssetRecorder::remove_saft(who, asset_id, saft.units, saft.nav)?;
			SAFTNetAssetValue::<T>::mutate(asset_id, |nav| *nav = nav.saturating_sub(saft.nav));

			Self::deposit_event(Event::<T>::SAFTRemoved(asset_id, saft_id));

			Ok(())
		}

		/// Called to update the Net Asset Value (NAV) associated with
		/// a SAFT record in the registry.
		///
		/// The NAV of a SAFT is subject to change over time, and will be
		/// updated at regular intervals via governance proposals. Changing the
		/// NAV will also be reflected in the `asset-index`. This is a noop if
		/// the given `latest_nav` is equal to the current nav of the SAFT.
		///
		/// Parameters:
		///   - `asset_id`: The identifier of the SAFT
		///   - `saft_id`: The identifier of the targeted `SaftRecord` whose value should be updated
		///   - `latest_nav`: The NAV for the `SaftRecord` identified by the `index`
		///
		/// Callable by the governance committee.
		///
		/// Weight: `O(1)`
		#[pallet::weight(T::WeightInfo::report_nav())]
		#[transactional]
		pub fn report_nav(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			saft_id: SAFTId,
			latest_nav: T::Balance,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			let old_nav = ActiveSAFTs::<T>::try_mutate(asset_id, saft_id, |maybe_saft| -> Result<_, DispatchError> {
				let saft = maybe_saft.as_mut().ok_or(Error::<T>::SAFTNotFound)?;
				Ok(sp_std::mem::replace(&mut saft.nav, latest_nav))
			})?;

			if old_nav == latest_nav {
				// nothing to update
				return Ok(());
			}

			SAFTNetAssetValue::<T>::try_mutate(asset_id, |nav| -> Result<_, DispatchError> {
				*nav = nav.saturating_sub(old_nav).checked_add(&latest_nav).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::NavUpdated(asset_id, saft_id, old_nav, latest_nav));

			Ok(())
		}

		/// Converts the asset secured by the SAFT into a liquid asset with the given
		/// location
		///
		/// Callable by the governance committee.
		///
		/// Weight: `O(C)` where C is the number of SAFTs for the asset as tracked by the
		/// `SAFTCounter`.
		#[pallet::weight(T::WeightInfo::convert_to_liquid(SAFTCounter::<T>::get(asset_id).saturating_sub(1)))]
		#[transactional]
		pub fn convert_to_liquid(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			location: MultiLocation,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			// update the asset location and ensure it was a SAFT
			let maybe_availability = T::AssetRecorder::insert_asset_availability(asset_id, location.clone().into());
			ensure!(maybe_availability == Some(AssetAvailability::Saft), Error::<T>::ExpectedSAFT);

			// remove all SAFT records
			SAFTNetAssetValue::<T>::take(asset_id);
			let counter = SAFTCounter::<T>::take(asset_id);
			for saft_id in SAFTId::zero()..counter {
				ActiveSAFTs::<T>::take(asset_id, saft_id);
			}

			Self::deposit_event(Event::<T>::ConvertedToLiquid(asset_id, location));
			Ok(())
		}
	}

	impl<T: Config> SaftRegistry<T::AssetId, T::Balance> for Pallet<T> {
		fn net_saft_value(asset: T::AssetId) -> T::Balance {
			SAFTNetAssetValue::<T>::get(asset)
		}
	}

	/// Trait for the asset-index pallet extrinsic weights.
	pub trait WeightInfo {
		fn add_saft() -> Weight;
		fn remove_saft() -> Weight;
		fn report_nav() -> Weight;
		fn convert_to_liquid(_: u32) -> Weight;
	}

	/// For backwards compatibility and tests
	impl WeightInfo for () {
		fn add_saft() -> Weight {
			Default::default()
		}

		fn remove_saft() -> Weight {
			Default::default()
		}

		fn report_nav() -> Weight {
			Default::default()
		}

		fn convert_to_liquid(_: u32) -> Weight {
			Default::default()
		}
	}
}
