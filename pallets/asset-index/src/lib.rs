// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # AssetIndex Pallet
//!
//! Tracks all the assets in the PINT index, composed of multiple assets

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
#[allow(clippy::unused_unit, clippy::large_enum_variant)]
pub mod pallet {
    pub use crate::traits::{AssetRecorder, MultiAssetRegistry};
    pub use crate::types::MultiAssetAdapter;
    use crate::types::{AssetAvailability, IndexAssetData, PendingRedemption};
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::{CheckedAdd, Zero},
        sp_std::{convert::TryInto, prelude::*},
        traits::{Currency, LockableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use pallet_asset_depository::MultiAssetDepository;
    use pallet_price_feed::PriceFeed;
    use pallet_remote_asset_manager::RemoteAssetManager;
    use xcm::opaque::v0::MultiLocation;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    type BalanceFor<T> = <<T as Config>::IndexToken as Currency<AccountIdFor<T>>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin that is allowed to administer the index
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        /// Currency implementation to use as the index token
        type IndexToken: LockableCurrency<Self::AccountId>;
        /// Period after the minting of the index token for which 100% is locked up.
        /// Only applies to users contributing assets directly to index
        #[pallet::constant]
        type LockupPeriod: Get<Self::BlockNumber>;
        /// The minimum amount of the index token that can be redeemed for the underlying asset in the index
        #[pallet::constant]
        type MinimumRedemption: Get<BalanceFor<Self>>;
        /// Minimum amount of time between redeeming index tokens
        /// and being able to withdraw the awarded assets
        #[pallet::constant]
        type WithdrawalPeriod: Get<Self::BlockNumber>;
        /// The maximum amount of DOT that can exist in the index
        #[pallet::constant]
        type DOTContributionLimit: Get<BalanceFor<Self>>;
        /// Type that handles cross chain transfers
        type RemoteAssetManager: RemoteAssetManager<
            AccountIdFor<Self>,
            Self::AssetId,
            BalanceFor<Self>,
        >;
        /// Type used to identify assets
        type AssetId: Parameter + Member;
        /// Handles asset depositing and withdrawing from sovereign user accounts
        type MultiAssetDepository: MultiAssetDepository<
            Self::AssetId,
            AccountIdFor<Self>,
            BalanceFor<Self>,
        >;
        /// The types that provides the necessary asset price pairs
        type PriceFeed: PriceFeed<Self::AssetId>;
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
    #[pallet::metadata(T::AssetId = "AccountId", AccountIdFor<T> = "AccountId", BalanceFor<T> = "Balance")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // A new asset was added to the index and some index token paid out
        // \[AssetIndex, AssetUnits, IndexTokenRecipient, IndexTokenPayout\]
        AssetAdded(T::AssetId, BalanceFor<T>, AccountIdFor<T>, BalanceFor<T>),
        // A new deposit of an asset into the index has been performed
        // \[AssetId, AssetUnits, Account, PINTPayout\]
        Deposited(T::AssetId, BalanceFor<T>, AccountIdFor<T>, BalanceFor<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Thrown if adding units to an asset holding causes its numerical type to overflow
        AssetUnitsOverflow,
        /// Thrown if no index could be found for an asset identifier.
        UnsupportedAsset,
        /// Thrown if calculating the volume of units of an asset with it's price overflows.
        AssetVolumeOverflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)] // TODO: Set weights
        /// Callable by an admin to add new assets to the index and mint some IndexToken
        /// Caller balance is updated to allocate the correct amount of the IndexToken
        /// Creates IndexAssetData if it doesn’t exist, otherwise adds to list of deposits
        pub fn add_asset(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            units: BalanceFor<T>,
            availability: AssetAvailability,
            value: BalanceFor<T>,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin.clone())?;
            let caller = ensure_signed(origin)?;
            <Self as AssetRecorder<T::AssetId, BalanceFor<T>>>::add_asset(
                &asset_id,
                &units,
                &availability,
            )?;
            T::IndexToken::deposit_into_existing(&caller, value)?;
            Self::deposit_event(Event::AssetAdded(asset_id, units, caller, value));
            Ok(().into())
        }

        /// Initiate a transfer from the user's sovereign account into the index.
        ///
        /// This will withdraw the given amount from the user's sovereign account and mints PINT proportionally using the latest available price pairs
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn deposit(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            amount: BalanceFor<T>,
        ) -> DispatchResultWithPostInfo {
            let caller = ensure_signed(origin)?;

            let mut holding = Holdings::<T>::get(&asset_id)
                .filter(|holding| matches!(holding.availability, AssetAvailability::Liquid(_)))
                .ok_or(Error::<T>::UnsupportedAsset)?;

            let price = T::PriceFeed::get_price(asset_id.clone())?;
            let units: u128 = amount
                .try_into()
                .map_err(|_| Error::<T>::AssetUnitsOverflow)?;
            let pint_amount: BalanceFor<T> = price
                .volume(units)
                .ok_or(Error::<T>::AssetVolumeOverflow)
                .and_then(|units| units.try_into().map_err(|_| Error::<T>::AssetUnitsOverflow))?;

            // make sure we can store the additional deposit
            holding.units = holding
                .units
                .checked_add(&amount)
                .ok_or(Error::<T>::AssetUnitsOverflow)?;

            // withdraw from the caller's sovereign account
            T::MultiAssetDepository::withdraw(&asset_id, &caller, amount)?;
            // update the holding
            Holdings::<T>::insert(asset_id.clone(), holding);
            // add minted PINT to user's balance
            T::IndexToken::deposit_creating(&caller, pint_amount);
            Self::deposit_event(Event::Deposited(asset_id, amount, caller, pint_amount));
            Ok(().into())
        }

        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn withdraw(
            origin: OriginFor<T>,
            _amount: BalanceFor<T>,
        ) -> DispatchResultWithPostInfo {
            let _caller = ensure_signed(origin)?;
            todo!();
        }
    }

    impl<T: Config> Pallet<T> {
        /// The amount of index tokens held by the given user
        pub fn index_token_balance(account: &T::AccountId) -> BalanceFor<T> {
            T::IndexToken::total_balance(account)
        }
    }

    impl<T: Config> AssetRecorder<T::AssetId, BalanceFor<T>> for Pallet<T> {
        /// Creates IndexAssetData if entry with given assetID does not exist.
        /// Otherwise adds the units to the existing holding
        fn add_asset(
            asset_id: &T::AssetId,
            units: &BalanceFor<T>,
            availability: &AssetAvailability,
        ) -> DispatchResult {
            Holdings::<T>::try_mutate(asset_id, |value| -> Result<_, Error<T>> {
                let index_asset_data = value.get_or_insert_with(|| {
                    IndexAssetData::<BalanceFor<T>>::new(
                        BalanceFor::<T>::zero(),
                        availability.clone(),
                    )
                });
                index_asset_data.units = index_asset_data
                    .units
                    .checked_add(units)
                    .ok_or(Error::AssetUnitsOverflow)?;
                Ok(())
            })?;
            Ok(())
        }

        fn remove_asset(_: &T::AssetId) -> DispatchResult {
            todo!();
        }
    }

    impl<T: Config> MultiAssetRegistry<T::AssetId> for Pallet<T> {
        fn native_asset_location(asset: &T::AssetId) -> Option<MultiLocation> {
            Holdings::<T>::get(asset).and_then(|holding| {
                if let AssetAvailability::Liquid(location) = holding.availability {
                    Some(location)
                } else {
                    None
                }
            })
        }

        fn is_liquid_asset(asset: &T::AssetId) -> bool {
            Holdings::<T>::get(asset)
                .map(|holding| matches!(holding.availability, AssetAvailability::Liquid(_)))
                .unwrap_or_default()
        }
    }
}
