// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Shareable PINT types

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		app_crypto::sp_core,
		generic,
		traits::{BlakeTwo256, IdentifyAccount, Verify},
		FixedPointNumber, FixedPointOperand, FixedU128, MultiSignature, OpaqueExtrinsic as UncheckedExtrinsic,
	},
	sp_std::vec::Vec,
};
use xcm::v0::MultiLocation;

/// Some way of identifying an account on the chain. We intentionally make it
/// equivalent to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of
/// them, but you never know...
pub type AccountIndex = u32;

/// Signed version of Balance
pub type Amount = i128;

/// Identifier for an asset.
pub type AssetId = u32;

/// Balance of an account.
pub type Balance = u128;

/// Block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// An index to a block.
pub type BlockNumber = u32;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Identifier for price feeds.
pub type FeedId = u64;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Index of a transaction in the chain.
pub type Index = u32;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Nonce = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on
/// the chain.
pub type Signature = MultiSignature;

pub type AccountPublic = <Signature as Verify>::Signer;

/// Value type for price feeds.
pub type Value = u128;

/// Identifier for a SAFT
pub type SAFTId = u32;

/// The type to represent asset prices
pub type Price = FixedU128;

pub type Ratio = FixedU128;

/// Defines the location of an asset
/// Liquid implies it exists on a chain somewhere in the network and
/// can be moved around
/// SAFT implies the asset is a Simple Agreement for Future Tokens and the
/// promised tokens are not able to be transferred or traded until some time
/// in the future.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum AssetAvailability {
	Liquid(MultiLocation),
	Saft,
}

impl AssetAvailability {
	/// Whether this asset data represents a liquid asset
	pub fn is_liquid(&self) -> bool {
		matches!(self, AssetAvailability::Liquid(_))
	}

	/// Whether this asset data represents a SAFT
	pub fn is_saft(&self) -> bool {
		matches!(self, AssetAvailability::Saft)
	}
}

impl From<MultiLocation> for AssetAvailability {
	fn from(location: MultiLocation) -> Self {
		AssetAvailability::Liquid(location)
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct AssetProportions<AssetId> {
	/// The per token value used to calculate proportions
	pub nav: Price,
	/// All the assets with their proportions
	pub proportions: Vec<AssetProportion<AssetId>>,
}

/// Represents an asset and its proportion in the value of the index
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct AssetProportion<AssetId> {
	/// The identifier for the asset
	pub asset: AssetId,
	/// The the share of all units of the asset held in the index
	pub proportion: Ratio,
}

impl<AssetId> AssetProportion<AssetId> {
	pub fn new(asset: AssetId, proportion: Ratio) -> Self {
		Self { asset, proportion }
	}

	/// Calculates the share of the asset of the units
	pub fn of<N: FixedPointOperand>(&self, units: N) -> Option<N> {
		self.proportion.checked_mul_int(units)
	}
}

/// Defines an asset pair identifier
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct AssetPricePair<AssetId> {
	/// The base asset id of this pair.
	pub base: AssetId,
	/// The quote asset
	pub quote: AssetId,
	/// The price of `base/quote`
	pub price: Price,
}

impl<AssetId: frame_support::sp_std::cmp::PartialEq> AssetPricePair<AssetId> {
	/// Whether this pair involves the `asset`
	pub fn involves_asset(&self, asset: &AssetId) -> bool {
		self.is_base(asset) || self.is_quote(asset)
	}

	/// Whether the provided asset is the `base` asset of this price pair
	pub fn is_base(&self, asset: &AssetId) -> bool {
		self.base == *asset
	}

	/// Whether the provided asset is the `quote` asset of this price pair
	pub fn is_quote(&self, asset: &AssetId) -> bool {
		self.quote == *asset
	}
}

impl<AssetId> AssetPricePair<AssetId> {
	/// Create a new instance
	pub fn new(base: AssetId, quote: AssetId, price: Price) -> Self {
		Self { base, quote, price }
	}

	/// Returns the price fraction `base/quote`
	pub fn price(&self) -> &Price {
		&self.price
	}

	/// Returns the price fraction `quote/base`
	///
	/// Returns `None` if `price = 0`.
	pub fn reciprocal_price(&self) -> Option<Price> {
		self.price.reciprocal()
	}

	/// Calculates the total volume of the provided units of the `quote` assetId
	/// w.r.t. price pair
	pub fn volume<N: FixedPointOperand>(&self, units: N) -> Option<N> {
		self.price.checked_mul_int(units)
	}

	/// Calculates the total volume of the provided units of the `base` assetId
	/// w.r.t. price pair
	pub fn reciprocal_volume<N: FixedPointOperand>(&self, units: N) -> Option<N> {
		self.reciprocal_price()?.checked_mul_int(units)
	}

	/// Turns this price pair of `base/quote` into a price pair of `quote/base`
	///
	/// Returns `None` if `price = 0`.
	pub fn invert(self) -> Option<Self> {
		Some(Self { base: self.quote, quote: self.base, price: self.price.reciprocal()? })
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn mock_pair(base: u128, quote: u128) -> AssetPricePair<u64> {
		AssetPricePair { base: 1, quote: 2, price: Price::checked_from_rational(base, quote).unwrap() }
	}

	#[test]
	fn can_detect_involvement() {
		let pair = mock_pair(600, 200);
		assert!(pair.is_quote(&2));
		assert!(pair.is_base(&1));
	}

	#[test]
	fn can_determine_volume() {
		let pair = mock_pair(600, 200);
		let value_of_units_measured_in_quote = pair.volume(300u128).unwrap();
		assert_eq!(value_of_units_measured_in_quote, 600 / 200 * 300);
	}

	#[test]
	fn can_determine_reciprocal_volume() {
		let pair = mock_pair(800, 200);
		let value_of_units_measured_in_base = pair.reciprocal_volume(500u128).unwrap();
		assert_eq!(value_of_units_measured_in_base, 200 / 8 * 5);
	}

	#[test]
	fn can_invert_pair() {
		let pair = mock_pair(500, 100);
		let reciprocal_price = pair.reciprocal_price().unwrap();
		let inverted = pair.invert().unwrap();
		assert_eq!(reciprocal_price, *inverted.price());
	}
}
