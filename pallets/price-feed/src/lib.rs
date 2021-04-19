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
    use frame_support::{pallet_prelude::*, sp_runtime::PerThing, traits::{Get}, };
    use frame_system::pallet_prelude::*;
    use pallet_chainlink_feed::{FeedOracle, FeedInterface, Feed};
    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;
    use frame_support::sp_runtime::{FixedU128};

    type FeedIdFor<T> =  <<T as Config>::Oracle as FeedOracle<T>>::FeedId;

    type FeedValueFor<T> =  <<<T as Config>::Oracle as FeedOracle<T>>::Feed as FeedInterface<T>>::Value;

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

        // /// The price type to represent the different asset price pairs
        // type Price: FixedPointNumber + CheckedMul + One;

        /// Type used to identify the assets.
        type AssetId: Parameter + Member + MaybeSerializeDeserialize;

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

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config>  where <<T as Config>::Oracle as FeedOracle<T>>::FeedId: MaybeSerializeDeserialize {
        /// The mappings to insert at genesis
        pub asset_feeds: Vec<(T::AssetId, FeedIdFor<T>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> where <<T as Config>::Oracle as FeedOracle<T>>::FeedId: MaybeSerializeDeserialize{
        fn default() -> Self {
            Self {
                asset_feeds: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> where <<T as Config>::Oracle as FeedOracle<T>>::FeedId: MaybeSerializeDeserialize {
        fn build(&self) {
           for (asset, feed) in &self.asset_feeds {
               AssetFeeds::<T>::insert(asset.clone(),feed.clone())
           }
        }
    }

    #[cfg(feature = "std")]
    impl<T: Config> GenesisConfig<T> where <<T as Config>::Oracle as FeedOracle<T>>::FeedId: MaybeSerializeDeserialize {
        /// Direct implementation of `GenesisBuild::build_storage`.
        ///
        /// Kept in order not to break dependency.
        pub fn build_storage(&self) -> Result<frame_support::sp_runtime::Storage, String> {
            <Self as GenesisBuild<T>>::build_storage(self)
        }

        /// Direct implementation of `GenesisBuild::assimilate_storage`.
        ///
        /// Kept in order not to break dependency.
        pub fn assimilate_storage(
            &self,
            storage: &mut frame_support::sp_runtime::Storage,
        ) -> Result<(), String> {
            <Self as GenesisBuild<T>>::assimilate_storage(self, storage)
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
        AssetPriceFeedNotFound,
        /// Thrown when the underlying price feed does not yet contain a valid round.
        InvalidFeedValue,
        /// Thrown if the calculation of the price ratio fails due to exceeding the
        /// accuracy of the configured price.
        ExceededAccuracy
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}


    impl<T: Config> Pallet<T> {

        /// Returns the corresponding identifier for the asset's price feed
        pub fn get_asset_feed_id(asset_id : &T::AssetId) -> Option<FeedIdFor<T>> {
            AssetFeeds::<T>::get(asset_id)
        }

        /// Returns the latest value in the feed together with the feed's decimals
        /// or an error if no feed was found for the given
        /// or the feed doesn't contain any valid round yet.
        fn get_latest_valid_value(feed_id: FeedIdFor<T>) -> Result<(FeedValueFor<T>, u8), DispatchError> {
            let feed = T::Oracle::feed(feed_id).ok_or_else(||Error::<T>::AssetPriceFeedNotFound)?;
            ensure!(feed.first_valid_round().is_some(), Error::<T>::InvalidFeedValue);
            Ok((feed.latest_data().answer, feed.decimals()))
        }
    }

    impl<T: Config> PriceFeed<T::AssetId> for Pallet<T> {

        /// Returns a `AssetPricePair` where `base` is the configured `SelfAssetId`.
        fn get_price(
            quote: T::AssetId,
        ) -> Result<AssetPricePair<T::AssetId>, DispatchError> {
            Self::get_price_pair(T::SelfAssetId::get(), quote)
        }

        fn get_price_pair(
            base: T::AssetId,
            quote: T::AssetId,
        ) -> Result<AssetPricePair<T::AssetId>, DispatchError> {
            let base_feed_id = Self::get_asset_feed_id(&base).ok_or_else(||Error::<T>::AssetPriceFeedNotFound)?;
            let quote_feed_id = Self::get_asset_feed_id(&quote).ok_or_else(||Error::<T>::AssetPriceFeedNotFound)?;

            let (mut last_base_value, base_decimals) = Self::get_latest_valid_value(base_feed_id)?;
            let (mut last_quote_value, quote_decimals) = Self::get_latest_valid_value(quote_feed_id)?;

            if base_decimals > quote_decimals {

            } else if quote_decimals > base_decimals {

            }

            let maybe_adjustment_multiplier = 10u128.checked_pow(10);


            // T::Price::checked_from_rational()

            todo!()
        }
    }


    fn adjust_with_multiplier() {

    }
}
