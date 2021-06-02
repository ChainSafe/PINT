// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use codec::Encode;
use frame_support::dispatch::Output;
use frame_support::{dispatch::DispatchResult, sp_runtime::RuntimeDebug, sp_std::vec::Vec};

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

/// A helper to encode an item using the provided context
pub trait EncodeWith<T, I> {
    /// Encodes the given info if possible
    fn encoded_with(ctx: &T, input: I) -> Option<Vec<u8>>;
}
