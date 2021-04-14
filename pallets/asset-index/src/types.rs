// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::pallet_prelude::*;
use xcm::v0::MultiLocation;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Defines the location of an asset
/// Liquid implies it exists on a chain somewhere in the network and
/// can be moved around
/// SAFT implies the asset is a Simple Agreement for Future Tokens and the
/// promised tokens are not able to be transferred or traded until some time
/// in the future.
pub enum AssetAvailability {
    Liquid(MultiLocation),
    SAFT,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// A representation of some number of assets that are managed by the index
pub struct IndexAssetData<Balance> {
    pub units: Balance,
    pub availability: AssetAvailability,
}

impl<Balance> IndexAssetData<Balance> {
    pub fn new(units: Balance, availability: AssetAvailability) -> Self {
        Self {
            units,
            availability,
        }
    }
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
    asset: AssetId,
    state: RedemptionState,
    units: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Describes an in progress withdrawal of a collection of assets from the index
pub struct PendingRedemption<AssetId, Balance, BlockNumber> {
    initiated: BlockNumber,
    assets: Vec<AssetWithdrawal<AssetId, Balance>>,
}
