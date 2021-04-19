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
    use frame_support::{pallet_prelude::*, sp_runtime::PerThing, traits::Get};
    use frame_system::pallet_prelude::*;
    use pallet_chainlink_feed::FeedOracle;

    type FeedIdFor<T> =  <<T as Config>::Oracle as FeedOracle<T>>::FeedId;

    /// Provides access to all the price feeds
    /// This is used to determine the equivalent amount of PINT for assets
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The unit used to measure the value of assets
        type BaseCurrency: Parameter + Member;

        /// The origin that is allowed to insert asset -> feed mappings
        type AdminOrigin: EnsureOrigin<Self::Origin>;

        /// The asset identifier for the native asset (PINT).
        #[pallet::constant]
        type SelfAssetId: Get<Self::AssetId>;

        /// Used to define the decimal precision when calculating prices
        // TODO this needs to be factored in when converting the feed prices with their decimals
        type Precision: PerThing + Encode;

        /// Type used to identify the assets.
        type AssetId: Parameter + Member;

        /// The internal oracle that gives access to the asset's price feeds.
        ///
        /// NOTE: this assumes all the feeds provide data in the same base currency.
        /// When querying the price of an asset (`quote`/`asset`) from the oracle,
        /// its price is given by means of the asset pair `(base / quote)`. (e.g. DOT/PINT)
        type Oracle: FeedOracle<Self>;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    /// Store a mapping (AssetId) -> FeedId for all active assets
    pub type AssetFeeds<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        FeedIdFor<T>,
        OptionQuery,
    >;

    #[pallet::extra_constants]
    impl<T: Config> Pallet<T> {
        /// The decimal precision to use when calculating price fractions
        pub fn precision() -> T::Precision {
            T::Precision::one()
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new assetId -> feedId mapping was inserted
        /// \[AssetId, NewFeedId, OldFeedId\]
        UpdateAssetPriceFeed(T::AssetId, FeedIdFor<T>, Option<FeedIdFor<T>>),
        /// An assetId -> feedId was removed
        /// \[AssetId, FeedId\]
        RemoveAssetPriceFeed(T::AssetId, Option<FeedIdFor<T>>),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        /// Callable by an admin to track a price feed identifier for the asset
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn track_asset_price_feed(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            feed_id : FeedIdFor<T>
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;
            let old_feed_id = AssetFeeds::<T>::mutate(&asset_id, |maybe_feed_id | {
                maybe_feed_id.replace(feed_id.clone())
            } );
            Self::deposit_event(Event::UpdateAssetPriceFeed(asset_id, feed_id, old_feed_id));
            Ok(().into())
        }

        /// Callable by an admin to untrack the asset's price feed.
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn untrack_asset_price_feed(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;
            let feed_id =  AssetFeeds::<T>::take(&asset_id);
            Self::deposit_event(Event::RemoveAssetPriceFeed(asset_id, feed_id));
            Ok(().into())
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Thrown if no price feed was found for an asset
        AssetPriceFeedNotFound
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}


    impl<T: Config> Pallet<T> {

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
