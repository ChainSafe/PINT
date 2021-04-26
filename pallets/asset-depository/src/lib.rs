// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Asset Depository Pallet
//!
//! Provides support for storing the balances of multiple assets

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod traits;
mod types;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    use crate::traits::MultiAssetDepository;
    use crate::types::AccountBalance;
    use frame_support::sp_runtime::traits::{CheckedAdd, CheckedSub};
    use frame_support::{
        pallet_prelude::*,
        sp_runtime::traits::{AtLeast32BitUnsigned, Zero},
    };
    use frame_system::pallet_prelude::*;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The balance type for cross chain transfers
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize;

        /// Asset Id that is used to identify different kinds of assets.
        type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    /// The balances the assets stored for an account.
    ///
    /// This is used to temporarily store balances for assets.
    #[pallet::storage]
    pub type Accounts<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AccountIdFor<T>,
        Twox64Concat,
        T::AssetId,
        AccountBalance<T::Balance>,
        ValueQuery,
    >;

    /// The aggregated balance of an asset.
    #[pallet::storage]
    pub type TotalBalance<T: Config> =
        StorageMap<_, Twox64Concat, T::AssetId, T::Balance, ValueQuery>;

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {
        /// Thrown when depositing an amount of an asset has caused an overflow of the aggregated balance.
        TotalBalanceOverflow,
        /// Thrown when depositing amount into an user account caused an overflow.
        BalanceOverflow,
        /// Thrown when withdrawing would cause an underflow
        NotEnoughBalance,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    impl<T: Config> Pallet<T> {
        /// The total amount of the given asset currently held
        pub fn aggregated_balance(asset_id: &T::AssetId) -> T::Balance {
            TotalBalance::<T>::get(asset_id)
        }

        /// The total balance of an asset of a user
        pub fn total_balance(asset_id: &T::AssetId, who: &AccountIdFor<T>) -> T::Balance {
            Accounts::<T>::get(who, asset_id).total_balance()
        }

        /// The current available balance of an asset of a user
        pub fn available_balance(asset_id: &T::AssetId, who: &AccountIdFor<T>) -> T::Balance {
            Accounts::<T>::get(who, asset_id).available
        }

        /// Set the available balance of the given account the given value.
        pub(crate) fn set_available_balance(
            asset_id: &T::AssetId,
            who: &AccountIdFor<T>,
            amount: T::Balance,
        ) {
            Accounts::<T>::mutate(who, asset_id, |account| {
                account.available = amount;
            });
        }
    }

    impl<T: Config> MultiAssetDepository<T::AssetId, AccountIdFor<T>, T::Balance> for Pallet<T> {
        /// Deposit the `amount` of the given asset into the available balance of the given account `who`.
        fn deposit(
            asset_id: &T::AssetId,
            who: &AccountIdFor<T>,
            amount: T::Balance,
        ) -> DispatchResult {
            if amount.is_zero() {
                return Ok(());
            }

            TotalBalance::<T>::try_mutate(asset_id, |total| -> DispatchResult {
                *total = total
                    .checked_add(&amount)
                    .ok_or(Error::<T>::TotalBalanceOverflow)?;

                // SAFETY: this can't overflow because the balance for an account is
                //  at most equal to the total balance and adding to it already succeeded
                Self::set_available_balance(
                    &asset_id,
                    who,
                    Self::available_balance(&asset_id, who) + amount,
                );

                Ok(())
            })
        }

        /// Withdraw the `amount` of the given asset from the available balance of the given account `who`.
        fn withdraw(
            asset_id: &T::AssetId,
            who: &AccountIdFor<T>,
            amount: T::Balance,
        ) -> DispatchResult {
            if amount.is_zero() {
                return Ok(());
            }

            Accounts::<T>::try_mutate(who, &asset_id, |balance| -> DispatchResult {
                balance.available = balance
                    .available
                    .checked_sub(&amount)
                    .ok_or(Error::<T>::NotEnoughBalance)?;
                Ok(())
            })?;

            // SAFETY: this can't underflow because the total balance for an asset is at least
            //  equal to the total balance in the given account and subtracting already succeeded
            TotalBalance::<T>::mutate(asset_id, |total| *total -= amount);

            Ok(())
        }
    }
}
