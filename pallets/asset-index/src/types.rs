// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use codec::{Decode, Encode};
use frame_support::{
	sp_runtime::{traits::Zero, RuntimeDebug},
	sp_std::vec::Vec,
};

/// Abstraction over the lock of minted index token that are locked up for
/// `LockupPeriod`
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct IndexTokenLock<BlockNumber, Balance> {
	/// Locked amount of index token.
	pub locked: Balance,
	/// The block when the locked index token can be unlocked.
	pub end_block: BlockNumber,
}

/// Metadata for an asset
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
pub struct AssetMetadata<BoundedString> {
	pub name: BoundedString,
	pub symbol: BoundedString,
	pub decimals: u8,
}

/// Represents a single asset being withdrawn
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct AssetWithdrawal<AssetId, Balance> {
	/// The identifier of the asset
	pub asset: AssetId,
	/// The amount of asset units about to be transferred to the LP.
	pub units: Balance,
	/// The amount still reserved for this withdrawal.
	pub reserved: Balance,
	/// Whether this withdrawal was already been closed.
	pub withdrawn: bool,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Describes an in progress withdrawal of a collection of assets from the index
pub struct PendingRedemption<AssetId, Balance, BlockNumber> {
	/// The block after which the redemption process is over.
	pub end_block: BlockNumber,
	/// All the withdrawals resulted from the redemption.
	pub assets: Vec<AssetWithdrawal<AssetId, Balance>>,
}

/// Represents the redemption of a given pint amount based on the
/// `AssetDistribution`.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct AssetRedemption<AssetId, Balance> {
	/// All the assets together with their redeemed amount
	pub asset_amounts: Vec<(AssetId, Balance)>,
	/// The total amount of redeemed pint
	pub redeemed_index_tokens: Balance,
}

impl<AssetId, Balance: Zero> Default for AssetRedemption<AssetId, Balance> {
	fn default() -> Self {
		Self { asset_amounts: Vec::new(), redeemed_index_tokens: Balance::zero() }
	}
}
