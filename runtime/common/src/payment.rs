// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Multiasset related fungibles adapter to allow payments in multiple assets

use frame_support::{sp_runtime::DispatchError, traits::tokens::BalanceConversion};
use sp_std::marker::PhantomData;

use primitives::{traits::NavProvider, AssetId, Balance};

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
