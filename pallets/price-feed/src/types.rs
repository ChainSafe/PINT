// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::{FixedPointNumber, FixedPointOperand, FixedU128};

/// The type to represent asset prices
pub type Price = FixedU128;

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

impl<AssetId> AssetPricePair<AssetId>
where
    AssetId: Member,
{
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

    /// Calculates the total volume of the provided units of the `quote` assetId w.r.t. price pair
    pub fn volume<N: FixedPointOperand>(&self, units: N) -> Option<N> {
        self.price.checked_mul_int(units)
    }

    /// Calculates the total volume of the provided units of the `base` assetId w.r.t. price pair
    pub fn reciprocal_volume<N: FixedPointOperand>(&self, units: N) -> Option<N> {
        self.reciprocal_price()?.checked_mul_int(units)
    }

    /// Turns this price pair of `base/quote` into a price pair of `quote/base`
    ///
    /// Returns `None` if `price = 0`.
    pub fn invert(self) -> Option<Self> {
        Some(Self {
            base: self.quote,
            quote: self.base,
            price: self.price.reciprocal()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_pair(base: u128, quote: u128) -> AssetPricePair<u64> {
        AssetPricePair {
            base: 1,
            quote: 2,
            price: Price::checked_from_rational(base, quote).unwrap(),
        }
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
