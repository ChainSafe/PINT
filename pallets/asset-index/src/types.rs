// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::pallet_prelude::*;
use xcm::opaque::v0::MultiLocation;

/// Abstraction over the lock of minted index token that are locked up for
/// `LockupPeriod`
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct IndexTokenLock<BlockNumber, Balance> {
    /// Locked amount of index token.
    pub locked: Balance,
    /// The block when the locked index token can be unlocked.
    pub end_block: BlockNumber,
}

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

/// Metadata for an asset
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
pub struct AssetMetadata<BoundedString> {
    pub name: BoundedString,
    pub symbol: BoundedString,
    pub decimals: u8,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// State of a single asset withdrawal on some parachain
pub enum RedemptionState {
    Initiated,
    Unbonding,
    Transferred,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Represents a single asset being withdrawn
pub struct AssetWithdrawal<AssetId, Balance> {
    pub asset: AssetId,
    pub state: RedemptionState,
    pub units: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Describes an in progress withdrawal of a collection of assets from the index
pub struct PendingRedemption<AssetId, Balance, BlockNumber> {
    pub initiated: BlockNumber,
    pub assets: Vec<AssetWithdrawal<AssetId, Balance>>,
}
