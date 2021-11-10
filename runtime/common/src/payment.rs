// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Multiasset related fungibles adapter to allow payments in multiple assets

use codec::{Decode, Encode};
use frame_support::{
	sp_runtime::{DispatchError, RuntimeDebug},
	traits::tokens::BalanceConversion,
};
use sp_std::marker::PhantomData;

use primitives::{traits::NavProvider, AssetId, Balance};

/// Possible errors when converting between external and asset balances.
#[derive(Eq, PartialEq, Copy, Clone, RuntimeDebug, Encode, Decode, scale_info::TypeInfo)]
pub enum ConversionError {
	/// The external minimum balance must not be zero.
	MinBalanceZero,
	/// The asset is not present in storage.
	AssetMissing,
	/// The asset is not sufficient and thus does not have a reliable `min_balance` so it cannot be
	/// converted.
	AssetNotSufficient,
}

/// Converts a balance value into an asset balance based on the current index token NAV.
pub struct BalanceToAssetBalance<NAV>(PhantomData<NAV>);

impl<NAV> BalanceConversion<Balance, AssetId, Balance> for BalanceToAssetBalance<NAV>
where
	NAV: NavProvider<AssetId, Balance>,
{
	type Error = DispatchError;

	/// Convert the given balance value into an asset balance based on current index token NAV.
	///
	/// Will return `Err` if theNAVconversion failed
	fn to_asset_balance(balance: Balance, asset_id: AssetId) -> Result<Balance, Self::Error> {
		NAV::asset_equivalent(balance, asset_id)
	}
}
