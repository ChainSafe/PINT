// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Local Treasury Pallet
//!
//! Manages PINT exclusively. The treasury is a single account which is derived from the configured
//! `PalletId`. It maintains ownership of various assets and is controlled by the Governance
//! Committee. Deposits to the Treasury can be done by simply transferring funds to its AccountId.
//! The committee can execute proposals to withdraw funds from the Treasury.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod tests;

// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResult,
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
		/// Origin that is allowed to manage the treasury balance and initiate
		/// withdrawals
		type AdminOrigin: EnsureOrigin<Self::Origin>;
		/// PalletId used to generate the `AccountId` which holds the balance of the
		/// treasury.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The pallet to use as the base currency for this treasury
		type Currency: Currency<Self::AccountId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The weight for this pallet's extrinsics.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Admin successfully transferred some funds from the treasury to
		/// another account parameters. \[recipient, amount\]
		Withdrawn(AccountIdFor<T>, BalanceFor<T>),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::extra_constants]
	impl<T: Config> Pallet<T> {
		/// Returns the `AccountId` of the treasury account.
		pub fn treasury_account() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer balance from the treasury to another account.
		///
		/// Only callable by the AdminOrigin.
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(origin: OriginFor<T>, amount: BalanceFor<T>, recipient: AccountIdFor<T>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			T::Currency::transfer(&Self::treasury_account(), &recipient, amount, AllowDeath)?;

			Self::deposit_event(Event::Withdrawn(recipient, amount));

			Ok(())
		}
	}

	/// Trait for the treasury pallet extrinsic weights.
	pub trait WeightInfo {
		fn withdraw() -> Weight;
	}

	/// For backwards compatibility and tests
	impl WeightInfo for () {
		fn withdraw() -> Weight {
			Default::default()
		}
	}
}
