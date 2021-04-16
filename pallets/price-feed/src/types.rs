// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::AtLeast32BitUnsigned;
use frame_support::sp_runtime::{PerThing, SaturatedConversion};

/// Defines an asset pair identifier
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct AssetPricePair<AssetId, Price> {
    /// The base asset id of this pair.
    pub base: AssetId,
    /// The quote asset
    pub quote: AssetId,
    /// The price for `base/quote`
    price: Price,
}

impl<AssetId, Price> AssetPricePair<AssetId, Price>
where
    AssetId: Member,
    Price: PerThing,
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

    /// Calculates the total volume of the provided units of the `quote` assetId w.r.t. price pair
    pub fn volume<N>(&self, units: N) -> Price
    where
        N: Into<Price>,
    {
        self.price * Price::saturated_from(units)
    }

    /// Calculates the total volume of the provided units of the `base` assetId w.r.t. price pair
    pub fn reciprocal_volume<N>(&self, units: N) -> u128
    where
        N: Into<u128>,
    {
        self.price.saturating_reciprocal_mul(units.into())
    }
}

impl<AssetId, Price> AssetPricePair<AssetId, Price>
where
    AssetId: Member,
    Price: PerThing + From<u128>,
{
    /// Turns this price pair of `base/quote` into a price pair of `quote/base`
    pub fn invert(self) -> Self {
        Self {
            base: self.quote,
            quote: self.base,
            price: self.price.saturating_reciprocal_mul(1).into(),
        }
    }
}
