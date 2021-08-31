// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::dispatch::{DispatchError, DispatchResultWithPostInfo};
use primitives::{AssetPricePair, Price};

/// An interface to access price data
pub trait PriceFeed<AssetId> {
	/// Returns the current price for the given asset measured in the constant denominating asset
	/// which is used as the quote currency, whereas the price of the `base` Asset will be the base
	/// currency for the price pair. *Note*: this returns the price for 1 basic unit
	fn get_price(base: AssetId) -> Result<Price, DispatchError>;

	/// Returns the current price pair for the prices of the base and quote asset in the form of
	/// `base/quote`
	fn get_relative_price_pair(base: AssetId, quote: AssetId) -> Result<AssetPricePair<AssetId>, DispatchError>;
}

#[cfg(feature = "runtime-benchmarks")]
pub trait PriceFeedBenchmarks<AccountId, AssetId> {
	fn create_feed(caller: AccountId, asset_id: AssetId) -> DispatchResultWithPostInfo;
}
