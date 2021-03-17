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
        traits::OriginTrait,
        traits::{Currency, Get},
    };
    use frame_system::pallet_prelude::*;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    type BalanceFor<T> = <<T as Config>::Currency as Currency<AccountIdFor<T>>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type AdminOrigin: OriginTrait;
        /// This must be an unique 8 character string.
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
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        SomethingStored(u32, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Error names should be descriptive.
        NoneValue,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Can add helper functions on the config here
    impl<T: Config> Module<T> {
        pub fn treasury_account_id() -> T::AccountId {
            T::ModuleId::get().into_account()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn withdraw(
            origin: OriginFor<T>,
            _amount: BalanceFor<T>,
            _recipient: AccountIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            let _caller = ensure_signed(origin)?;
            // STUB
            Ok(().into())
        }
    }
}
