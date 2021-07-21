// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use codec::{Decode, Encode};
use frame_support::pallet_prelude::RuntimeDebug;
use xcm::v0::{Junction, MultiLocation};
use xcm_calls::assets::AssetsConfig;

/// Represents the config for the statemint parachain
#[derive(Encode, Decode, Clone, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct StatemintConfig {
    /// Dedicated config for the internal `pallet_assets`
    pub assets_config: AssetsConfig,
    /// The id of the `statemint` parachain
    ///
    /// *NOTE* using `u32` here instead of location, since `MultiLocation` has
    /// no serde support
    pub parachain_id: u32,
    /// Whether interacting with the parachain is currently active
    pub enabled: bool,
    /// The `pallet_assets` asset id of the pint token on statemint
    pub pint_asset_id: u32,
}

impl StatemintConfig {
    /// The direct path to the `statemint` parachain
    pub fn location(&self) -> MultiLocation {
        Junction::Parachain(self.parachain_id).into()
    }
}
