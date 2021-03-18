#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::{traits::AccountIdConversion, ModuleId},
        traits::{Currency, ExistenceRequirement::AllowDeath, Get},
    };
    use frame_system::pallet_prelude::*;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    type BalanceFor<T> = <<T as Config>::Currency as Currency<AccountIdFor<T>>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        /// ModuleId must be an unique 8 character string.
        /// it is used to generate the account to hold the balances in this pallet
        type ModuleId: Get<ModuleId>;
        type Currency: Currency<Self::AccountId>;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn something)]
    pub type Something<T> = StorageValue<_, u32>;

    #[pallet::event]
    // #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Some admin origin successfully transferred some funds from the treasury to another account
        /// parameters. [initiator, recipient, amount]
        WithdrawlMadeFromTreasury(AccountIdFor<T>, BalanceFor<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        NoneValue,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Can add helper functions on the config here
    impl<T: Config> Module<T> {
        fn treasury_account_id() -> T::AccountId {
            T::ModuleId::get().into_account()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn withdraw(
            origin: OriginFor<T>,
            amount: BalanceFor<T>,
            recipient: AccountIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;

            T::Currency::transfer(&Self::treasury_account_id(), &recipient, amount, AllowDeath)?;

            Self::deposit_event(Event::WithdrawlMadeFromTreasury(recipient, amount));

            Ok(().into())
        }
    }
}
