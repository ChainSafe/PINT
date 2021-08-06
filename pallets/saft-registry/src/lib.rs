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
        sp_runtime::traits::{AtLeast32BitUnsigned, Zero},
        sp_std::prelude::*,
        transactional,
    };
    use frame_system::pallet_prelude::*;
    use pallet_asset_index::{traits::AssetRecorder, types::AssetAvailability};
    use xcm::v0::MultiLocation;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        // Origin that is allowed to manage the SAFTs
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        type AssetRecorder: AssetRecorder<Self::AccountId, Self::AssetId, Self::Balance>;
        type Balance: Parameter + AtLeast32BitUnsigned + Default + Copy;
        type AssetId: Parameter + From<u32> + Copy;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The weight for this pallet's extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct SAFTRecord<Balance, NAV> {
        nav: NAV,
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

    #[pallet::storage]
    /// Store a mapping (AssetId) -> NAV for all active SAFTs
    ///
    /// NAV for the assets being secured by the SAFT at time of submission
    pub type ActiveSAFTs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        Vec<SAFTRecord<T::Balance, T::Balance>>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::metadata(T::AssetId = "AssetId", T::Balance = "Balance")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new SAFT was added
        /// \[AssetId, AssetIndex\]
        SAFTAdded(T::AssetId, u32),
        /// A SAFT was removed
        /// \[AssetId, AssetIndex\]
        SAFTRemoved(T::AssetId, u32),
        /// The NAV for a SAFT was updated
        /// \[AssetId, AssetIndex, OldNav, NewNav\]
        NavUpdated(T::AssetId, u32, T::Balance, T::Balance),
        /// A SAFT was converted into a liquid asset
        /// \[AssetId, MultiLocation\]
        ConvertedToLiquid(T::AssetId, MultiLocation),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// No SAFT with the given index exists for the given AssetId
        AssetIndexOutOfBounds,
        /// Thrown if the given asset was not a known SAFT.
        ExpectedSAFT,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Callable by the governance committee to add new SAFT to the index
        /// and mint the given amount of IndexToken. The amount of PINT
        /// minted and awarded to the LP is specified as part of the
        /// associated proposal
        /// If the asset does not exist yet, it will get created.
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
            <T as Config>::AssetRecorder::add_saft(&caller, asset_id, units, nav)?;

            let index = ActiveSAFTs::<T>::mutate(asset_id, |records| {
                let index = records.len() as u32;
                records.push(SAFTRecord::new(nav, units));
                index
            });

            Self::deposit_event(Event::<T>::SAFTAdded(asset_id, index));

            Ok(())
        }

        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn remove_saft(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            index: u32,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin.clone())?;
            let who = ensure_signed(origin)?;

            let index_usize: usize = index as usize;

            ActiveSAFTs::<T>::try_mutate(asset_id, |safts| -> Result<(), DispatchError> {
                if index_usize >= safts.len() {
                    Err(Error::<T>::AssetIndexOutOfBounds.into())
                } else {
                    let record = safts.remove(index_usize);
                    T::AssetRecorder::remove_saft(who, asset_id, record.units, record.nav)?;
                    Self::deposit_event(Event::<T>::SAFTRemoved(asset_id, index));

                    Ok(())
                }
            })?;

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::report_nav())]
        /// Called to update the Net Asset Value (NAV) associated with
        /// a SAFT record in the registry
        pub fn report_nav(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            index: u32,
            latest_nav: T::Balance,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            let index_usize: usize = index as usize;
            ActiveSAFTs::<T>::try_mutate(asset_id, |safts| -> Result<(), DispatchError> {
                if let Some(mut nav_record) = safts.get_mut(index_usize) {
                    let old_nav = nav_record.nav;
                    nav_record.nav = latest_nav;
                    Self::deposit_event(Event::<T>::NavUpdated(
                        asset_id, index, old_nav, latest_nav,
                    ));
                    Ok(())
                } else {
                    // get_mut will return None if index out of bounds
                    Err(Error::<T>::AssetIndexOutOfBounds.into())
                }
            })?;
            Ok(())
        }

        /// Converts the given SAFT asset into a liquid asset with the given
        /// location
        #[pallet::weight(T::WeightInfo::convert_to_liquid())]
        #[transactional]
        pub fn convert_to_liquid(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            location: MultiLocation,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            // update the asset location and ensure it was a SAFT
            let maybe_availability =
                T::AssetRecorder::insert_asset_availability(asset_id, location.clone().into());
            ensure!(
                maybe_availability == Some(AssetAvailability::Saft),
                Error::<T>::ExpectedSAFT
            );

            // remove all SAFT records, balances are already tracked for each deposit
            ActiveSAFTs::<T>::remove(&asset_id);

            Self::deposit_event(Event::<T>::ConvertedToLiquid(asset_id, location));
            Ok(())
        }
    }

    /// Trait for the asset-index pallet extrinsic weights.
    pub trait WeightInfo {
        fn add_saft() -> Weight;
        // TODO: (incompleted)
        //
        // https://github.com/ChainSafe/PINT/pull/73
        //
        // fn remove_saft() -> Weight;
        fn report_nav() -> Weight;
        fn convert_to_liquid() -> Weight;
    }

    /// For backwards compatibility and tests
    impl WeightInfo for () {
        fn add_saft() -> Weight {
            Default::default()
        }

        fn report_nav() -> Weight {
            Default::default()
        }

        fn convert_to_liquid() -> Weight {
            Default::default()
        }
    }
}
