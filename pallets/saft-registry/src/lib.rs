// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod traits;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    use crate::traits::{AssetAvailability, AssetRecorder};
    use frame_support::{
        dispatch::DispatchResultWithPostInfo, pallet_prelude::*,
        sp_runtime::traits::AtLeast32BitUnsigned,
    };
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        // Origin that is allowed to manage the SAFTs
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        type AssetRecorder: AssetRecorder<Self::AssetId, Self::Balance>;
        type Balance: Parameter + AtLeast32BitUnsigned;
        type AssetId: Parameter;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
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
    /// Store a mapping (AssetId) -> Vec<SaftRecord> for all active SAFTs
    pub type ActiveSAFTs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        Vec<SAFTRecord<T::Balance, T::Balance>>,
        ValueQuery,
    >;

    #[pallet::event]
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
    }

    #[pallet::error]
    pub enum Error<T> {
        // No SAFT with the given index exists for the given AssetId
        AssetIndexOutOfBounds,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn add_saft(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            nav: T::Balance,
            units: T::Balance,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;

            <T as Config>::AssetRecorder::add_asset(
                &asset_id,
                &units,
                &AssetAvailability::SAFT,
                &nav,
            )?;
            ActiveSAFTs::<T>::append(
                asset_id.clone(),
                SAFTRecord::new(nav, units),
            );
            Self::deposit_event(Event::<T>::SAFTAdded(asset_id, 0));

            Ok(().into())
        }

        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn remove_saft(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;
            let index_usize: usize = index as usize;
            ActiveSAFTs::<T>::try_mutate(asset_id.clone(), |safts| -> Result<(), DispatchError> {
                if index_usize >= safts.len() {
                    Err(Error::<T>::AssetIndexOutOfBounds.into())
                } else {
                    <T as Config>::AssetRecorder::remove_asset(&asset_id)?;
                    safts.remove(index_usize);
                    Self::deposit_event(Event::<T>::SAFTRemoved(asset_id, index));

                    Ok(())
                }
            })?;

            Ok(().into())
        }

        #[pallet::weight(10_000)] // TODO: Set weights
        /// Called to update the Net Asset Value (NAV) associated with
        /// a SAFT record in the registry
        pub fn report_nav(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            index: u32,
            latest_nav: T::Balance,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;
            let index_usize: usize = index as usize;
            ActiveSAFTs::<T>::try_mutate(asset_id.clone(), |safts| -> Result<(), DispatchError> {
                if let Some(mut nav_record) = safts.get_mut(index_usize) {
                    let prev_nav = nav_record.nav.clone();
                    nav_record.nav = latest_nav.clone();
                    <T as Config>::AssetRecorder::update_nav(&asset_id, &latest_nav)?;
                    Self::deposit_event(Event::<T>::NavUpdated(
                        asset_id, index, prev_nav, latest_nav,
                    ));
                    Ok(())
                } else {
                    // get_mut will return None if index out of bounds
                    Err(Error::<T>::AssetIndexOutOfBounds.into())
                }
            })?;
            Ok(().into())
        }
    }
}
