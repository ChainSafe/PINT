// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Price Feed Pallet
//!
//! This pallet is an abstraction over the `chainlink-feed-pallet` which provides oracle data from
//! the chainlink network. This requires some more configurations and provides prices for assets
//! used in the index. For the purpose of the PINT Index all prices will be using a single
//! denominating asset which will be one base currency, which maybe USD, so that the net asset value
//! of the index can be calculated. It is therefore assumed that all the registered chainlink feeds
//! are price pairs with a consisting asset price (e.g. USD as in USD/DOT). **NOTE:** Most
//! `chainlink` price feeds use `USD` as the quote currency to easily calculate how much USD is
//! needed to purchase one units of the `base` currency, or the value of a certain amount of assets
//! by multiplying it with the units the assets. Therefore ths price feed pallet sticks to the same
//! convention, so that the NAV of the index is the sum of all the assets multiplied with their
//! price in form of (Asset/USD) divided by the total supply of index tokens which essentially is
//! the currency price pair of (PINT/USD).

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
pub use mock::FeedBuilder;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
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
	#[cfg(feature = "runtime-benchmarks")]
	pub use crate::traits::PriceFeedBenchmarks;
	#[cfg(feature = "runtime-benchmarks")]
	use frame_benchmarking::Zero;
	#[cfg(feature = "std")]
	use frame_support::traits::GenesisBuild;

	pub use crate::{traits::PriceFeed, types::TimestampedValue};
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::{traits::CheckedDiv, FixedPointNumber, FixedPointOperand},
		sp_std::convert::TryFrom,
		traits::{Get, Time},
	};
	use frame_system::pallet_prelude::*;
	use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData};
	pub use primitives::{AssetPricePair, Price};

	pub type FeedIdFor<T> = <T as pallet_chainlink_feed::Config>::FeedId;
	pub type MomentOf<T> = <<T as Config>::Time as Time>::Moment;
	pub type FeedValueFor<T> = <T as pallet_chainlink_feed::Config>::Value;
	pub type TimestampedFeedValue<T> = TimestampedValue<(FeedValueFor<T>, u8), MomentOf<T>>;

	/// Provides access to all the price feeds
	/// This is used to determine the equivalent amount of PINT for assets
	///
	/// The internal chainlink oracle type `FeedOracle` gives access to the
	/// asset's price feeds.
	///
	/// NOTE: this assumes all the feeds provide data in the same base
	/// currency. When querying the price of an asset
	/// (`quote`/`asset`) from the oracle, its price is given by
	/// means of the asset pair `(base / quote)`. (e.g. DOT/PINT)
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_chainlink_feed::Config {
		/// The origin that is allowed to insert asset -> feed mappings
		type AdminOrigin: EnsureOrigin<Self::Origin>;

		/// The asset identifier for the native asset (PINT).
		#[pallet::constant]
		type SelfAssetId: Get<Self::AssetId>;

		/// Type used to identify the assets.
		type AssetId: Parameter + Member + MaybeSerializeDeserialize + TryFrom<u8>;

		/// Type to keep track of timestamped values
		type Time: Time;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The weight for this pallet's extrinsics.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Store a mapping (AssetId) -> FeedId for all active assets
	#[pallet::storage]
	#[pallet::getter(fn asset_feed)]
	pub type AssetFeeds<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, FeedIdFor<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn latest_answer_timestamp)]
	/// Stores the timestamp of the latest answer of each feed (feed) ->
	/// Timestamp
	pub type LatestAnswerTimestamp<T: Config> = StorageMap<_, Twox64Concat, FeedIdFor<T>, MomentOf<T>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config>
	where
		<T as pallet_chainlink_feed::Config>::FeedId: MaybeSerializeDeserialize,
	{
		/// The mappings to insert at genesis
		pub asset_feeds: Vec<(T::AssetId, FeedIdFor<T>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T>
	where
		<T as pallet_chainlink_feed::Config>::FeedId: MaybeSerializeDeserialize,
	{
		fn default() -> Self {
			Self { asset_feeds: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T>
	where
		<T as pallet_chainlink_feed::Config>::FeedId: MaybeSerializeDeserialize,
	{
		fn build(&self) {
			for (asset, feed) in &self.asset_feeds {
				AssetFeeds::<T>::insert(asset.clone(), *feed)
			}
		}
	}

	#[cfg(feature = "std")]
	impl<T: Config> GenesisConfig<T>
	where
		<T as pallet_chainlink_feed::Config>::FeedId: MaybeSerializeDeserialize,
	{
		/// Direct implementation of `GenesisBuild::build_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn build_storage(&self) -> Result<frame_support::sp_runtime::Storage, String> {
			<Self as GenesisBuild<T>>::build_storage(self)
		}

		/// Direct implementation of `GenesisBuild::assimilate_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn assimilate_storage(&self, storage: &mut frame_support::sp_runtime::Storage) -> Result<(), String> {
			<Self as GenesisBuild<T>>::assimilate_storage(self, storage)
		}
	}

	#[pallet::event]
	#[pallet::metadata(T::AssetId = "AssetId", FeedIdFor<T> = "FeedId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new assetId -> feedId mapping was inserted
		/// \[AssetId, NewFeedId, OldFeedId\]
		UpdateAssetPriceFeed(T::AssetId, FeedIdFor<T>, Option<FeedIdFor<T>>),
		/// An assetId -> feedId was removed
		/// \[AssetId, FeedId\]
		RemoveAssetPriceFeed(T::AssetId, FeedIdFor<T>),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Maps the given asset to an existing price feed.
		/// If the asset was already mapped to a price feed this will update the mapping
		///
		/// Callable by the governance committee.
		#[pallet::weight(<T as Config>::WeightInfo::map_asset_price_feed())]
		pub fn map_asset_price_feed(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			feed_id: FeedIdFor<T>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let old_feed_id = AssetFeeds::<T>::mutate(&asset_id, |maybe_feed_id| maybe_feed_id.replace(feed_id));
			Self::deposit_event(Event::UpdateAssetPriceFeed(asset_id, feed_id, old_feed_id));
			Ok(())
		}

		/// Removes the the `asset` -> `feed` mapping if it exists.
		/// This is a noop if the asset is not tracked.
		///
		/// Callable by the governance committee.
		#[pallet::weight(<T as Config>::WeightInfo::unmap_asset_price_feed())]
		pub fn unmap_asset_price_feed(origin: OriginFor<T>, asset_id: T::AssetId) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			if let Some(feed_id) = AssetFeeds::<T>::take(&asset_id) {
				Self::deposit_event(Event::RemoveAssetPriceFeed(asset_id, feed_id));
			}
			Ok(())
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Thrown if no price feed was found for an asset
		AssetPriceFeedNotFound,
		/// Thrown when the underlying price feed does not yet contain a valid
		/// round.
		InvalidFeedValue,
		/// Thrown if the calculation of the price ratio fails due to exceeding
		/// the accuracy of the configured price.
		ExceededAccuracy,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	impl<T: Config> Pallet<T> {
		/// Returns the corresponding identifier for the asset's price feed
		/// according to the internal mapping
		pub fn asset_feed_id(asset_id: &T::AssetId) -> Option<FeedIdFor<T>> {
			AssetFeeds::<T>::get(asset_id)
		}

		/// Returns the latest value in the feed together with the feed's
		/// decimals (the feed's precision) or an error if no feed was found for the given
		/// or the feed doesn't contain any valid round yet.
		pub fn latest_valid_value(feed_id: FeedIdFor<T>) -> Result<(FeedValueFor<T>, u8), DispatchError> {
			let feed = pallet_chainlink_feed::Pallet::<T>::feed(feed_id).ok_or(Error::<T>::AssetPriceFeedNotFound)?;
			ensure!(feed.first_valid_round().is_some(), Error::<T>::InvalidFeedValue);
			Ok((feed.latest_data().answer, feed.decimals()))
		}

		/// Same as `latest_value` but with the time the answer was emitted
		pub fn latest_timestamped_value(feed_id: FeedIdFor<T>) -> Result<TimestampedFeedValue<T>, DispatchError> {
			let moment = LatestAnswerTimestamp::<T>::get(&feed_id);
			let value = Self::latest_valid_value(feed_id)?;
			Ok(TimestampedValue { value, moment })
		}
	}

	impl<T: Config> PriceFeed<T::AssetId> for Pallet<T>
	where
		FeedValueFor<T>: FixedPointOperand,
	{
		fn get_price(base: T::AssetId) -> Result<Price, DispatchError> {
			let feed = Self::asset_feed_id(&base).ok_or(Error::<T>::AssetPriceFeedNotFound)?;

			let (value, precision) = Self::latest_valid_value(feed)?;
			let multiplier = 10u128.checked_pow(precision.into()).ok_or(Error::<T>::ExceededAccuracy)?;

			Price::checked_from_rational(value, multiplier).ok_or_else(|| Error::<T>::ExceededAccuracy.into())
		}

		fn get_relative_price_pair(
			base: T::AssetId,
			quote: T::AssetId,
		) -> Result<AssetPricePair<T::AssetId>, DispatchError> {
			let base_price = Self::get_price(base.clone())?;
			let quote_price = Self::get_price(quote.clone())?;
			let price = base_price.checked_div(&quote_price).ok_or(Error::<T>::ExceededAccuracy)?;
			Ok(AssetPricePair::new(base, quote, price))
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl<T: Config> PriceFeedBenchmarks<T::AccountId, T::AssetId> for Pallet<T> {
		fn create_feed(
			caller: <T as frame_system::Config>::AccountId,
			asset_id: T::AssetId,
		) -> DispatchResultWithPostInfo {
			use frame_benchmarking::vec;

			pallet_chainlink_feed::Pallet::<T>::set_feed_creator(
				<frame_system::Origin<T>>::Signed(pallet_chainlink_feed::Pallet::<T>::pallet_admin()).into(),
				caller.clone(),
			)?;

			pallet_chainlink_feed::Pallet::<T>::create_feed(
				<frame_system::Origin<T>>::Signed(caller.clone()).into(),
				100u32.into(),
				Zero::zero(),
				(1u8.into(), 100u8.into()),
				1u8.into(),
				8u8,
				vec![1; T::StringLimit::get() as usize],
				Zero::zero(),
				vec![(caller.clone(), caller.clone())],
				None,
				None,
			)?;

			let feed_id = <pallet_chainlink_feed::FeedCounter<T>>::get() - 1.into();
			AssetFeeds::<T>::insert(&asset_id, feed_id);
			pallet_chainlink_feed::Pallet::<T>::submit(
				<frame_system::Origin<T>>::Signed(caller.clone()).into(),
				feed_id,
				1_u32.into(),
				42.into(),
			)?;
			Ok(().into())
		}
	}

	impl<T: Config> pallet_chainlink_feed::traits::OnAnswerHandler<T> for Pallet<T> {
		fn on_answer(feed_id: FeedIdFor<T>, _: RoundData<T::BlockNumber, FeedValueFor<T>>) {
			LatestAnswerTimestamp::<T>::insert(feed_id, T::Time::now());
		}
	}

	/// Trait for the asset-index pallet extrinsic weights.
	pub trait WeightInfo {
		fn map_asset_price_feed() -> Weight;
		fn unmap_asset_price_feed() -> Weight;
	}

	/// For backwards compatibility and tests
	impl WeightInfo for () {
		fn map_asset_price_feed() -> Weight {
			Default::default()
		}

		fn unmap_asset_price_feed() -> Weight {
			Default::default()
		}
	}
}
