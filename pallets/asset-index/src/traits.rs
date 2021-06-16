// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pub use crate::types::{AssetAvailability, AssetMetadata};
use frame_support::{dispatch::DispatchResult, sp_runtime::traits::AtLeast32BitUnsigned};
use xcm::v0::MultiLocation;

pub trait AssetRecorder<AssetId, Balance> {
    /// Add an asset to the recorder. If an asset with the given AssetId already exists
    /// then the added asset units will be combined.
    /// The provided NAV parameter is the Net Asset Value of the total units provided
    /// given in units of some stable asset. In the case of an AssetId that already exists the
    /// newly provided NAV will be used to re-value the existing units and compute a total NAV
    fn add_asset(id: &AssetId, units: &Balance, availability: &AssetAvailability)
        -> DispatchResult;

    fn remove_asset(id: &AssetId) -> DispatchResult;
}

/// Type that provides the mapping between `AssetId` and `MultiLocation`.
pub trait MultiAssetRegistry<AssetId> {
    /// Determines the relative location of the consensus system where the given asset is native from the point of view of the current system
    fn native_asset_location(asset: &AssetId) -> Option<MultiLocation>;

    /// Whether the given identifier is currently supported as a liquid asset
    fn is_liquid_asset(asset: &AssetId) -> bool;
}

/// Type that calculations any fees to be deducted for every withdrawal.
pub trait WithdrawalFee<Balance> {
    /// Calculates the fee to be deducted from the PINT being redeemed
    // TODO specify parameters
    fn withdrawal_fee(amount: Balance) -> Balance;
}

impl<Balance: AtLeast32BitUnsigned> WithdrawalFee<Balance> for () {
    fn withdrawal_fee(_: Balance) -> Balance {
        Balance::zero()
    }
}
