// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::dispatch::DispatchResult;

/// Facility for remote asset transactions.
pub trait RemoteAssetManager<AccountId, AssetId, Balance> {
    /// Transfers the given amount of asset from the account's sovereign account
    /// on PINT into the account on the asset's destination.
    ///
    /// This performs the following steps:
    /// - Ensure the account has enough free balance of the given asset
    /// - Depending on the asset's location this will execute
    ///     - an XCM InitiateReserveWithdraw followed by XCM Deposit order,
    ///       if the location of the asset is a reserve location of PINT (Relay Chain)
    ///     - an XCM InitiateReserveWithdraw followed by XCM DepositReserveAsset order will be
    ///       dispatched as XCM ReserveAssetDeposit with an Xcm Deposit order
    fn transfer_asset(who: AccountId, asset: AssetId, amount: Balance) -> DispatchResult;

    /// Dispatch XCM to bond assets
    fn bond(asset: AssetId, amount: Balance) -> DispatchResult;

    /// Dispatch XCM to unbond assets
    fn unbond(asset: AssetId, amount: Balance) -> DispatchResult;
}
