// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::sp_runtime::DispatchResult;

/// An abstraction over the multiple balances for different assets
pub trait MultiAssetDepository<AssetId, AccountId, Balance> {
    /// Add `amount` to the balance of `who` under `asset_id`.
    fn deposit(asset_id: &AssetId, who: &AccountId, amount: Balance) -> DispatchResult;

    /// Remove `amount` from the balance of `who` under `asset_id`.
    fn withdraw(asset_id: &AssetId, who: &AccountId, amount: Balance) -> DispatchResult;
}
