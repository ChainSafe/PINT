// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::sp_runtime::DispatchResult;

/// An abstraction over the multiple balances for different assets
pub trait MultiAssetDepository<AssetId, AccountId, Balance> {
    /// The total amount of the given asset currently held
    fn aggregated_balance(asset_id: &AssetId) -> Balance;

    /// The total balance of an asset of a user
    fn total_balance(asset_id: &AssetId, who: &AccountId) -> Balance;

    /// The current available balance of an asset of a user
    fn available_balance(asset_id: &AssetId, who: &AccountId) -> Balance;

    /// Add `amount` to the balance of `who` under `asset_id`.
    fn deposit(asset_id: &AssetId, who: &AccountId, amount: Balance) -> DispatchResult;

    /// Remove `amount` from the balance of `who` under `asset_id`.
    fn withdraw(asset_id: &AssetId, who: &AccountId, amount: Balance) -> DispatchResult;
}
