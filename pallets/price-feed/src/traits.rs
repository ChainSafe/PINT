// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate::types::{AssetPricePair, Price};
use frame_support::dispatch::DispatchError;

/// An interface to access price data
pub trait PriceFeed<AssetId> {
	/// Returns the current price pair for `base/quote` where `base` is the
	/// native token
	fn get_price(quote: AssetId) -> Result<AssetPricePair<AssetId>, DispatchError>;

	/// Returns the current price pair for `base/quote`
	fn get_price_pair(base: AssetId, quote: AssetId) -> Result<AssetPricePair<AssetId>, DispatchError>;

	/// Set initial price pair if feed not exists
	fn ensure_price(quote: AssetId, units: Price) -> Result<AssetPricePair<AssetId>, DispatchError>;
}
