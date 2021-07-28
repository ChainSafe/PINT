// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use xcm::v0::{Junction, MultiLocation, Outcome};
use xcm_calls::assets::AssetsConfig;

/// Represents the config for the statemint parachain
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct StatemintConfig<AssetId> {
    /// Dedicated config for the internal `pallet_assets`
    pub assets_config: AssetsConfig,
    /// The id of the `statemint` parachain
    ///
    /// *NOTE* using `u32` here instead of location, since `MultiLocation` has
    /// no serde support
    pub parachain_id: u32,
    /// Whether interacting with the parachain is currently active
    pub enabled: bool,
    /// The `pallet_assets` asset identifier of the pint token on statemint
    pub pint_asset_id: AssetId,
}

impl<AssetId> StatemintConfig<AssetId> {
    /// The path to the `statemint` parachain
    pub fn location(&self) -> MultiLocation {
        (Junction::Parent, Junction::Parachain(self.parachain_id)).into()
    }
}

/// Outcome of an XCM unbonding api call
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug)]
pub enum UnBondingOutcome {
    /// Staking is not supported, therefore nothing to unbond
    NotSupported,
    /// Staking is supported, but the parachain's reserve account currently
    /// holds enough units as stash so that no unbonding procedure is necessary
    SufficientReserve,
    /// The outcome of the XCM unbond call
    Outcome(Outcome),
}
