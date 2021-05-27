// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::{dispatch::DispatchResult, sp_std::vec::Vec};

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

    /// Ensures that the unbonding process succeeded
    fn withdraw_unbonded(caller: AccountId, asset: AssetId, amount: Balance) -> DispatchResult;
}

/// Helper trait to encode the local Balance into the expected format on the target chain
pub trait BalanceEncoder<AssetId, Balance> {
    /// Convert the balance based on the given asset and append it to the destination.
    fn encoded_balance(asset: &AssetId, balance: Balance) -> Option<Vec<u8>>;
}
