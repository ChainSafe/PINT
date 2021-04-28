// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::dispatch::DispatchResult;
use xcm::v0::Xcm;

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
}

/// A XCM handler wrapper type for the cumulus XCM Handler to execute xcm locally.
pub trait XcmHandler<AccountId, Call> {
    fn execute_xcm(origin: AccountId, xcm: Xcm<Call>) -> DispatchResult;
}
