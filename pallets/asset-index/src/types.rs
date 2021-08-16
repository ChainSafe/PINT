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

/// State of a single asset withdrawal on some parachain
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum RedemptionState {
	/// This marks the state in which a withdrawal was initiated but the requested unbonding failed,
	/// either because the corresponding xcm unbonding failed to execute or because there is nothing
	/// to unbond and the minimum remote stash balance is exhausted.
	/// This indicates that the redemption in progress needs get confirmation that the remote asset
	/// manager followed up on the failed unbonding procedure.
	Initiated,
	/// Unbonding was successful due to:
	///   - the asset does not support staking.
	///   - the current parachain's stash account is liquid enough to cover the withdrawal after the
	///     redemption period without falling below the configured minimum stash balance threshold.
	///   - xcm unbonding call was sent successfully.
	///
	/// This state represents a waiting state until the redemption period is over.
	Unbonding,
	/// This is a intermediary state in which it will be attempted to transfer the
	/// units to the LP's account.
	Transferring,
	/// Successfully transferred the units to LP's account, the
	/// `AssetWithdrawal` has thus been completed.
	Withdrawn,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Represents a single asset being withdrawn
pub struct AssetWithdrawal<AssetId, Balance> {
	/// The identifier of the asset
	pub asset: AssetId,
	/// The state in which the redemption process currently is.
	pub state: RedemptionState,
	/// The amount of asset units about to be transferred to the LP.
	pub units: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Describes an in progress withdrawal of a collection of assets from the index
pub struct PendingRedemption<AssetId, Balance, BlockNumber> {
	/// When the redemption process is over
	pub end_block: BlockNumber,
	/// All the withdrawals resulted from the redemption
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
