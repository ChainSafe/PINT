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
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::AccountIdConversion,
        traits::{Currency, ExistenceRequirement::AllowDeath, Get},
        PalletId,
    };
    use frame_system::pallet_prelude::*;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    type BalanceFor<T> = <<T as Config>::Currency as Currency<AccountIdFor<T>>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin that is allowed to manage the treasury balance and initiate withdrawals
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        /// PalletId must be an unique 8 character string.
        /// It is used to generate the account ID which holds the balance of the treasury.
        #[pallet::constant]
        type PalletId: Get<PalletId>;
        /// The pallet to use as the base currency for this treasury
        type Currency: Currency<Self::AccountId>;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::metadata(AccountIdFor<T> = "AccountId", BalanceFor<T> = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Admin successfully transferred some funds from the treasury to another account
        /// parameters. \[initiator, recipient, amount\]
        Withdrawl(AccountIdFor<T>, BalanceFor<T>),
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::extra_constants]
    impl<T: Config> Pallet<T> {
        /// Returns the accountID for the treasury balance
        /// Transferring balance to this account funds the treasury
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer balance from the treasury to another account. Only callable by the AdminOrigin.
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn withdraw(
            origin: OriginFor<T>,
            amount: BalanceFor<T>,
            recipient: AccountIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;

            T::Currency::transfer(&Self::account_id(), &recipient, amount, AllowDeath)?;

            Self::deposit_event(Event::Withdrawl(recipient, amount));

            Ok(().into())
        }
    }
}
