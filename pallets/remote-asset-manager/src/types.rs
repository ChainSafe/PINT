// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use codec::{Decode, Encode};
use frame_support::{sp_runtime::traits::AtLeast32BitUnsigned, RuntimeDebug};
use xcm::v0::{Junction, MultiAsset, MultiLocation};

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
	///
	/// *NOTE:* this is not the full path to the asset on the statemint chain
	pub fn parahain_location(&self) -> MultiLocation {
		(Junction::Parent, Junction::Parachain(self.parachain_id)).into()
	}
}
impl<AssetId: Into<u128> + Copy> StatemintConfig<AssetId> {
	/// The XCM `MultiAsset` the statemint parachain expects in order to convert it correctly to the
	/// pint asset
	pub fn multi_asset(&self, amount: u128) -> MultiAsset {
		// TODO simplify when on polkadot-v0.9.9 (xcm-latest) with the correct asset id converter:
		// AsPrefixedGeneralIndex<Local, AssetId, JustTry>::reverse_ref(&self.pint_asset_id.into())
		// where Local is MultiLocation = Junctions::Here.into()
		MultiAsset::ConcreteFungible { id: Junction::GeneralIndex { id: self.pint_asset_id.into() }.into(), amount }
	}
}
