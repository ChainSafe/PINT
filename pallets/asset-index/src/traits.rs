// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pub use crate::types::AssetAvailability;
use frame_support::sp_runtime::DispatchError;

pub trait AssetRecorder<AssetId, Balance> {
    /// Add an asset to the recorder. If an asset with the given AssetId already exists
    /// then the added asset units will be combined.
    /// The provided NAV parameter is the Net Asset Value of the total units provided
    /// given in units of some stable asset. In the case of an AssetId that already exists the
    /// newly provided NAV will be used to re-value the existing units and compute a total NAV
    fn add_asset(
        id: &AssetId,
        units: &Balance,
        availability: &AssetAvailability,
        nav: &Balance,
    ) -> Result<(), DispatchError>;

    fn remove_asset(id: &AssetId) -> Result<(), DispatchError>;

    fn update_nav(id: &AssetId, nav: &Balance) -> Result<(), DispatchError>;
}
