// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Multiasset related fungibles adapter to allow payments in multiple assets

use frame_support::{
	sp_runtime::DispatchError,
	traits::tokens::BalanceConversion,
	weights::constants::{ExtrinsicBaseWeight, WEIGHT_PER_SECOND},
};
use primitives::{traits::NavProvider, AssetId, Balance};
use sp_std::marker::PhantomData;

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

fn base_tx_in_pint() -> Balance {
	1 / 1000
}

pub fn basic_per_second() -> u128 {
	let base_weight = Balance::from(ExtrinsicBaseWeight::get());
	let base_tx_per_second = (WEIGHT_PER_SECOND as u128) / base_weight;
	base_tx_per_second * base_tx_in_pint()
}

pub fn ksm_per_second() -> u128 {
	basic_per_second() / 50
}

pub fn dot_per_second() -> u128 {
	basic_per_second() / 50
}
