// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use xcm::v0::MultiLocation;

/// Type that provides the mapping between `AssetId` and `MultiLocation`.
pub trait MultiAssetRegistry<AssetId> {
    /// Determines the relative location of the consensus system where the given
    /// asset is native from the point of view of the current system
    fn native_asset_location(asset: &AssetId) -> Option<MultiLocation>;

    /// Whether the given identifier is currently supported as a liquid asset
    fn is_liquid_asset(asset: &AssetId) -> bool;
}
