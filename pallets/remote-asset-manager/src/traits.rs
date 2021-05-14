// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::dispatch::DispatchResult;

/// Facility for remote asset transactions.
pub trait RemoteAssetManager<AccountId, AssetId, Balance> {
    /// Withdraws the given amount from
    /// - ReserveAssetDeposit
    /// - InitiateReserveWithdraw
    /// - Deposit
    /// - Remote deposit on PINT again
    fn reserve_withdraw_and_deposit(
        who: AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> DispatchResult;

    /// Dispatch XCM to bound assets
    fn bond(asset: AssetId, amount: Balance) -> DispatchResult;

    /// Dispatch XCM to unbound assets
    fn unbond(asset: AssetId, amount: Balance) -> DispatchResult;
}
