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
// #[cfg(test)]
// pub use mock::FeedBuilder;

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


	pub use crate::{traits::PriceFeed, types::TimestampedValue};
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::{traits::CheckedDiv},
		traits::{Get, Time},
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{DataProvider};
	pub use primitives::{AssetPricePair, Price};
	pub use primitives::traits::{MaybeAssetIdConvert};

	pub type MomentOf<T> = <<T as Config>::Time as Time>::Moment;

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
	pub trait Config:
		frame_system::Config + MaybeAssetIdConvert<u8, Self::AssetId>
	{
		/// The origin that is allowed to insert asset -> feed mappings
		type AdminOrigin: EnsureOrigin<Self::Origin>;

		/// The asset identifier for the native asset (PINT).
		#[pallet::constant]
		type SelfAssetId: Get<Self::AssetId>;

		/// Type used to identify the assets.
		type AssetId: Parameter + Member + MaybeSerializeDeserialize;

		/// Type to keep track of timestamped values
		type Time: Time;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The weight for this pallet's extrinsics.
		type WeightInfo: WeightInfo;

		type DataProvider: DataProvider<Self::AssetId, Price>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);


	// #[cfg(feature = "std")]
	// impl<T: Config> GenesisConfig<T>
	// where
	// 	<T as pallet_chainlink_feed::Config>::FeedId: MaybeSerializeDeserialize,
	// {
	// 	/// Direct implementation of `GenesisBuild::build_storage`.
	// 	///
	// 	/// Kept in order not to break dependency.
	// 	pub fn build_storage(&self) -> Result<frame_support::sp_runtime::Storage, String> {
	// 		<Self as GenesisBuild<T>>::build_storage(self)
	// 	}

	// 	/// Direct implementation of `GenesisBuild::assimilate_storage`.
	// 	///
	// 	/// Kept in order not to break dependency.
	// 	pub fn assimilate_storage(&self, storage: &mut frame_support::sp_runtime::Storage) -> Result<(), String> {
	// 		<Self as GenesisBuild<T>>::assimilate_storage(self, storage)
	// 	}
	// }

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// 
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

	impl<T: Config> PriceFeed<T::AssetId> for Pallet<T> {
		fn get_price(base: T::AssetId) -> Result<Price, DispatchError> {
			// let feed = Self::asset_feed_id(&base).ok_or(Error::<T>::AssetPriceFeedNotFound)?;

			// let (value, precision) = Self::latest_valid_value(feed)?;
			// let multiplier = 10u128.checked_pow(precision.into()).ok_or(Error::<T>::ExceededAccuracy)?;

			// Price::checked_from_rational(value, multiplier).ok_or_else(|| Error::<T>::ExceededAccuracy.into())
			T::DataProvider::get(&base).ok_or_else(|| Error::<T>::ExceededAccuracy.into())
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
