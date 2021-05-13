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
    use crate::traits::WithdrawalFee;
    pub use crate::traits::{AssetRecorder, MultiAssetRegistry};
    pub use crate::types::MultiAssetAdapter;
    use crate::types::{AssetAvailability, IndexAssetData, PendingRedemption};
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::{AtLeast32BitUnsigned, CheckedAdd, CheckedDiv, CheckedSub, Zero},
        sp_std::{convert::TryInto, prelude::*, result::Result},
        traits::{Currency, LockableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use pallet_asset_depository::MultiAssetDepository;
    use pallet_price_feed::PriceFeed;
    use pallet_remote_asset_manager::RemoteAssetManager;
    use xcm::opaque::v0::MultiLocation;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin that is allowed to administer the index
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        /// Currency implementation to use as the index token
        type IndexToken: LockableCurrency<Self::AccountId, Balance = Self::Balance>;
        /// The balance type used within this pallet
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Into<u128>;
        /// Period after the minting of the index token for which 100% is locked up.
        /// Only applies to users contributing assets directly to index
        #[pallet::constant]
        type LockupPeriod: Get<Self::BlockNumber>;
        /// The minimum amount of the index token that can be redeemed for the underlying asset in the index
        #[pallet::constant]
        type MinimumRedemption: Get<Self::Balance>;
        /// Minimum amount of time between redeeming index tokens
        /// and being able to withdraw the awarded assets
        #[pallet::constant]
        type WithdrawalPeriod: Get<Self::BlockNumber>;
        /// The maximum amount of DOT that can exist in the index
        #[pallet::constant]
        type DOTContributionLimit: Get<Self::Balance>;
        /// Type that handles cross chain transfers
        type RemoteAssetManager: RemoteAssetManager<
            AccountIdFor<Self>,
            Self::AssetId,
            Self::Balance,
        >;
        /// Type used to identify assets
        type AssetId: Parameter + Member;
        /// Handles asset depositing and withdrawing from sovereign user accounts
        type MultiAssetDepository: MultiAssetDepository<
            Self::AssetId,
            AccountIdFor<Self>,
            Self::Balance,
        >;
        /// The types that provides the necessary asset price pairs
        type PriceFeed: PriceFeed<Self::AssetId>;
        /// The type that calculates the withdrawal fee
        type WithdrawalFee: WithdrawalFee<Self::Balance>;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    /// (AssetId) -> IndexAssetData
    pub type Holdings<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, IndexAssetData<T::Balance>, OptionQuery>;

    #[pallet::storage]
    /// (AccountId) -> Balance. Tracks how much each LP has contributed in PINT.
    pub type Depositors<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance, OptionQuery>;

    #[pallet::storage]
    ///  (AccountId) -> Vec<PendingRedemption>
    pub type PendingWithdrawals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vec<PendingRedemption<T::AssetId, T::Balance, BlockNumberFor<T>>>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::metadata(T::AssetId = "AccountId", AccountIdFor<T> = "AccountId", T::Balance = "Balance")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // A new asset was added to the index and some index token paid out
        // \[AssetIndex, AssetUnits, IndexTokenRecipient, IndexTokenPayout\]
        AssetAdded(T::AssetId, T::Balance, AccountIdFor<T>, T::Balance),
        // A new deposit of an asset into the index has been performed
        // \[AssetId, AssetUnits, Account, PINTPayout\]
        Deposited(T::AssetId, T::Balance, AccountIdFor<T>, T::Balance),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Thrown if adding units to an asset holding causes its numerical type to overflow
        AssetUnitsOverflow,
        /// Thrown if no index could be found for an asset identifier.
        UnsupportedAsset,
        /// Thrown if calculating the volume of units of an asset with it's price overflows.
        AssetVolumeOverflow,
        /// Thrown if the given amount of PINT to redeem is too low
        MinimumRedemption,
        /// Thrown when the redeemer does not have enough PINT as is requested for withdrawal.
        InsufficientDeposit,
        /// Thrown when calculating the NAV resulted in a overflow
        NAVOverflow,
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
            units: T::Balance,
            availability: AssetAvailability,
            value: T::Balance,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin.clone())?;
            let caller = ensure_signed(origin)?;
            <Self as AssetRecorder<T::AssetId, T::Balance>>::add_asset(
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
            amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let caller = ensure_signed(origin)?;

            let mut holding = Holdings::<T>::get(&asset_id)
                .filter(|holding| matches!(holding.availability, AssetAvailability::Liquid(_)))
                .ok_or(Error::<T>::UnsupportedAsset)?;

            let pint_amount = Self::asset_nav(asset_id.clone())?;

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

        /// Starts the withdraw process for the given amount of PINT to redeem for a distribution
        /// of underlying assets.
        ///
        /// All withdrawals undergo an unlocking period after which the assets can be withdrawn.
        /// A withdrawal fee will be deducted from the PINT being redeemed by the LP depending on
        /// how long the assets remained in the index.
        /// The remaining PINT will be burned to match the new NAV after this withdrawal.
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn withdraw(origin: OriginFor<T>, amount: T::Balance) -> DispatchResultWithPostInfo {
            let caller = ensure_signed(origin)?;
            ensure!(
                amount >= T::MinimumRedemption::get(),
                Error::<T>::MinimumRedemption
            );

            let deposit = Depositors::<T>::get(&caller).ok_or(Error::<T>::InsufficientDeposit)?;
            ensure!(deposit >= amount, Error::<T>::InsufficientDeposit);

            let fee = T::WithdrawalFee::withdrawal_fee(amount);
            let redeem = amount
                .checked_sub(&fee)
                .ok_or(Error::<T>::InsufficientDeposit)?;

            // calculate the distribution of the underlying assets

            // if total supply of any asset drops to 0 it gets removed from the index.

            todo!();
        }

        /// Completes the unbonding process on other parachains and
        /// transfers the redeemed assets into the sovereign account of the owner.
        ///
        /// All pending withdrawals need to have completed their lockup period
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn complete_withdraw(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            todo!();
        }
    }

    impl<T: Config> Pallet<T> {
        /// The amount of index tokens held by the given user
        pub fn index_token_balance(account: &T::AccountId) -> T::Balance {
            T::IndexToken::total_balance(account)
        }

        /// Calculates the total NAV of the Index token
        pub fn total_nav() -> Result<T::Balance, DispatchError> {
            Self::calculate_nav(Holdings::<T>::iter())
        }

        /// Calculates the NAV of all liquid assets the Index token
        pub fn liquid_nav() -> Result<T::Balance, DispatchError> {
            Self::calculate_nav(Holdings::<T>::iter().filter(|(_, holding)| holding.is_liquid()))
        }

        fn calculate_nav(
            iter: impl Iterator<Item = (T::AssetId, IndexAssetData<T::Balance>)>,
        ) -> Result<T::Balance, DispatchError> {
            let total_issuance = T::IndexToken::total_issuance();
            if total_issuance.is_zero() {
                return Ok(T::Balance::zero());
            }
            let mut nav = T::Balance::zero();
            for (asset, holding) in iter {
                nav = nav
                    .checked_add(&Self::calculate_asset_nav(asset, holding.units)?)
                    .ok_or(Error::<T>::NAVOverflow)?;
            }
            Ok(nav
                .checked_div(&total_issuance)
                .ok_or(Error::<T>::NAVOverflow)?)
        }

        /// Calculates the NAV for the given amount of the asset
        fn calculate_asset_nav(
            asset: T::AssetId,
            amount: T::Balance,
        ) -> Result<T::Balance, DispatchError> {
            let price = T::PriceFeed::get_price(asset)?;
            let units: u128 = amount.into();
            let pint_amount: T::Balance = price
                .volume(units)
                .ok_or(Error::<T>::AssetVolumeOverflow)
                .and_then(|units| units.try_into().map_err(|_| Error::<T>::AssetUnitsOverflow))?;
            Ok(pint_amount)
        }

        /// Calculates the NAV of a single asset
        pub fn asset_nav(asset: T::AssetId) -> Result<T::Balance, DispatchError> {
            let holding = Holdings::<T>::get(&asset).ok_or(Error::<T>::UnsupportedAsset)?;
            Self::calculate_asset_nav(asset, holding.units)
        }

        /// Calculates the distribution of assets equal to the value being redeemed and equivalent
        /// to the ration of the assets in the index is awarded to the redeemer.
        fn distribution(total_nav: T::Balance, nav: T::Balance) -> Vec<(T::AssetId, T::Balance)> {
            // total nav
            // partial for each asset -> nav
            todo!()
        }
    }

    impl<T: Config> AssetRecorder<T::AssetId, T::Balance> for Pallet<T> {
        /// Creates IndexAssetData if entry with given assetID does not exist.
        /// Otherwise adds the units to the existing holding
        fn add_asset(
            asset_id: &T::AssetId,
            units: &T::Balance,
            availability: &AssetAvailability,
        ) -> DispatchResult {
            Holdings::<T>::try_mutate(asset_id, |value| -> Result<_, Error<T>> {
                let index_asset_data = value.get_or_insert_with(|| {
                    IndexAssetData::<T::Balance>::new(T::Balance::zero(), availability.clone())
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
                .map(|holding| holding.is_liquid())
                .unwrap_or_default()
        }
    }
}
