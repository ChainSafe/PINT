// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::pallet_prelude::*;
use xcm::v0::MultiLocation;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum AssetAvailability {
    Liquid(MultiLocation),
    SAFT,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
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
pub enum RedemptionState {
    Initiated,
    Unbonding,
    Transferred,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct AssetWithdrawal<AssetId, Balance> {
    asset: AssetId,
    state: RedemptionState,
    units: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct PendingRedemption<AssetId, Balance, BlockNumber> {
    initiated: BlockNumber,
    assets: Vec<AssetWithdrawal<AssetId, Balance>>,
}
