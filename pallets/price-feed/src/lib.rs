// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Additional type used in this pallet
mod traits;
/// Additional structures used in this pallet
mod types;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    pub use crate::traits::PriceFeed;
    use crate::types::AssetPricePair;
    use frame_support::sp_runtime::PerThing;
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use pallet_chainlink_feed::FeedOracle;

    /// Provides access to all the price feeds
    /// This is used to determine the equivalent amount of PINT for assets
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The unit used to measure the value of assets
        type BaseCurrency: Parameter + Member;

        /// The asset identifier for the native asset (PINT) price feed in the oracle.
        #[pallet::constant]
        type SelfAssetFeedId: Get<Self::AssetFeedId>;

        /// Used to define the decimal precision when calculating prices
        // TODO should this be a property of the FeedOracle::Feed type instead, so that feeds can have different precisions?
        //  however when the base currency is the same for all feeds the precision should also be
        type Precision: PerThing + Encode;

        /// Type used to identify the asset's feed.
        type AssetFeedId: Parameter + Member;

        /// The internal oracle that gives access to the asset's price feeds.
        ///
        /// NOTE: this assumes all the feed's provide data in the same base currency.
        /// When querying the price of an asset (`quote` asset) from the, its price is given by means of the asset pair `(base / quote)`. (e.g. DOT/PINT)
        type Oracle: FeedOracle<Self>;

        // TODO probably also make this IsType<<Self as pallet_chainlink_feed::Config>::Event
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // TODO here we could track historical price pairs

    #[pallet::extra_constants]
    impl<T: Config> Pallet<T> {
        /// The decimal precision to use when calculating price fractions
        pub fn precision() -> T::Precision {
            T::Precision::one()
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    impl<T: Config> Pallet<T> {}

    impl<T: Config> PriceFeed<T::AssetFeedId, T::Precision> for Pallet<T> {
        fn get_price(
            quote: T::AssetFeedId,
        ) -> Result<AssetPricePair<T::AssetFeedId, T::Precision>, DispatchError> {
            Self::get_price_pair(T::SelfAssetFeedId::get(), quote)
        }

        fn get_price_pair(
            _base: T::AssetFeedId,
            _quote: T::AssetFeedId,
        ) -> Result<AssetPricePair<T::AssetFeedId, T::Precision>, DispatchError> {
            todo!()
        }
    }
}
