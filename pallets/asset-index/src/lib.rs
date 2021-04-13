// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod traits;
mod types;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    use crate::traits::AssetRecorder;
    use crate::types::{AssetAvailability, IndexAssetData, PendingRedemption};
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        traits::{Currency, LockableCurrency},
    };
    use frame_system::pallet_prelude::*;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    type BalanceFor<T> = <<T as Config>::IndexToken as Currency<AccountIdFor<T>>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        // Origin that is allowed to administer the index
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        // Currency implementation to use as the index token
        type IndexToken: LockableCurrency<Self::AccountId>;
        type LockupPeriod: Get<Self::BlockNumber>;
        type MinimumRedemption: Get<BalanceFor<Self>>;
        type WithdrawalPeriod: Get<Self::BlockNumber>;
        type DOTContributionLimit: Get<BalanceFor<Self>>;
        type AssetId: Parameter + Encode + Decode;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    /// (AssetId) -> IndexAssetData
    pub type Holdings<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, IndexAssetData<BalanceFor<T>>, OptionQuery>;

    #[pallet::storage]
    /// (AccountId) -> Balance. Tracks how much each LP has contributed
    pub type Depositors<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BalanceFor<T>, OptionQuery>;

    #[pallet::storage]
    ///  (AccountId) -> Vec<PendingRedemption>
    pub type PendingWithdrawals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vec<PendingRedemption<T::AssetId, BalanceFor<T>, BlockNumberFor<T>>>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AssetAdded(AccountIdFor<T>, BalanceFor<T>),
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)] // TODO: Set weights
        /// Index updated to include added assets
        /// Caller balance is updated to allocate the correct amount of the IndexToken
        /// Creates IndexAssetData if it doesn’t exist, otherwise adds to list of deposits
        pub fn add_asset(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            units: BalanceFor<T>,
            availaility: AssetAvailability,
            value: BalanceFor<T>,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin.clone())?;
            let caller = ensure_signed(origin)?;
            <Self as AssetRecorder<T::AssetId, BalanceFor<T>>>::add_asset(
                &asset_id,
                &units,
                &availaility,
            )?;
            T::IndexToken::deposit_into_existing(&caller, value)?;
            Self::deposit_event(Event::AssetAdded(caller, value));
            Ok(().into())
        }
    }

    impl<T: Config> AssetRecorder<T::AssetId, BalanceFor<T>> for Pallet<T> {
        /// Creates IndexAssetData if entry with given assetID does not exist.
        /// Otherwise adds the units to existing
        fn add_asset(
            asset_id: &T::AssetId,
            units: &BalanceFor<T>,
            availability: &AssetAvailability,
        ) -> Result<(), DispatchError> {
            Holdings::<T>::mutate(asset_id, |query| {
                if let Some(index_asset_data) = query {
                    // mutate if exists
                    index_asset_data.units += *units;
                } else {
                    // otherwise write a new entry
                    let index_asset_data =
                        IndexAssetData::<BalanceFor<T>>::new(*units, availability.clone());
                    Holdings::<T>::insert(asset_id, index_asset_data);
                }
            });
            Ok(())
        }

        fn remove_asset(asset_id: &T::AssetId) -> Result<(), DispatchError> {
            Holdings::<T>::remove(asset_id);
            Ok(())
        }
    }
}
