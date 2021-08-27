// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use codec::{Decode, Encode};
use frame_support::{sp_runtime::traits::AtLeast32BitUnsigned, RuntimeDebug};
use xcm::v0::{Junction, MultiLocation};
use xcm_calls::assets::AssetsConfig;

/// Represents all XCM calls of the `pallet_staking` pallet transacted on a parachain
#[derive(Default, Encode, Decode, Clone, PartialEq, RuntimeDebug)]
pub struct XcmStakingMessageCount {
	/// Total number of all `pallet_staking::Pallet::bond_extra` calls transacted
	pub bond_extra: u32,
	/// Total number of all `pallet_staking::Pallet::unbond` calls transacted
	pub unbond: u32,
	/// Total number of all `pallet_staking::Pallet::withdraw_unbonded` calls transacted
	pub withdraw_unbonded: u32,
}

/// Represents the different balances of an asset
#[derive(Default, Encode, Decode, Clone, PartialEq, RuntimeDebug)]
pub struct AssetLedger<Balance> {
	/// The real deposits contributed to the index
	pub deposited: Balance,
	/// the amount of the asset about to be withdrawn
	pub pending_redemption: Balance,
}

impl<Balance> AssetLedger<Balance>
where
	Balance: AtLeast32BitUnsigned + Copy,
{
	/// Cancel each balance out, after which at least 1 balance is zero.
	pub fn consolidate(&mut self) {
		let deposited = self.deposited;
		self.deposited = self.deposited.saturating_sub(self.pending_redemption);
		self.pending_redemption = self.pending_redemption.saturating_sub(deposited);
	}
}

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
