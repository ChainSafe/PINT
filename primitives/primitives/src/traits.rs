// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! This contains shared traits that are used in multiple pallets to prevent
//! circular dependencies

use codec::{Decode, Encode};
use frame_support::{
    dispatch::DispatchError,
    sp_runtime::DispatchResult,
    sp_std::{boxed::Box, result::Result},
    RuntimeDebug,
};
use xcm::v0::{MultiLocation, Outcome};

/// Type that provides the mapping between `AssetId` and `MultiLocation`.
pub trait MultiAssetRegistry<AssetId> {
    /// Determines the relative location of the consensus system where the given
    /// asset is native from the point of view of the current system
    fn native_asset_location(asset: &AssetId) -> Option<MultiLocation>;

    /// Whether the given identifier is currently supported as a liquid asset
    fn is_liquid_asset(asset: &AssetId) -> bool;
}

/// Facility for remote asset transactions.
pub trait RemoteAssetManager<AccountId, AssetId, Balance> {
    /// Transfers the given amount of asset from the account's sovereign account
    /// on PINT into the account on the asset's destination.
    ///
    /// This performs the following steps:
    /// - Ensure the account has enough free balance of the given asset
    /// - Depending on the asset's location this will execute
    ///     - an XCM InitiateReserveWithdraw followed by XCM Deposit order, if
    ///       the location of the asset is a reserve location of PINT (Relay
    ///       Chain)
    ///     - an XCM InitiateReserveWithdraw followed by XCM DepositReserveAsset
    ///       order will be dispatched as XCM ReserveAssetDeposit with an Xcm
    ///       Deposit order
    fn transfer_asset(
        who: AccountId,
        asset: AssetId,
        amount: Balance,
    ) -> frame_support::sp_std::result::Result<Outcome, DispatchError>;

    /// Dispatch XCM to bond assets
    fn bond(asset: AssetId, amount: Balance) -> DispatchResult;

    /// Dispatch XCM to unbond assets
    fn unbond(asset: AssetId, amount: Balance) -> UnbondingOutcome;
}

/// Abstracts net asset value (NAV) related calculations
pub trait NavProvider<AssetId: Clone, Balance> {
    /// Calculates the amount of index tokens that the given amount of the asset
    /// are worth.
    ///
    /// This is achieved by dividing the worth of the given units by the NAV.
    /// The worth, or volume, is determined by `vol_asset = units * Price_asset`
    /// (`asset_net_worth`), and since the `NAV` represents the per token value,
    /// the equivalent number of index token is `vol_asset / NAV`.
    fn index_token_equivalent(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

    /// Calculates the net worth of the given units of the given asset.
    /// .
    /// If the asset is liquid then the net worth of an asset is determined by multiplying the share price of the asset by the given amount.: `units * Price_asset`.
    ///
    /// If the asset is liquid then the net worth is determined by the net worth of the associated `SAFTRecords`.
    fn calculate_asset_net_worth(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

    /// Calculates the net worth of the given units of the given *liquid* asset.
    ///
    /// In contrast to `calculate_asset_net_worth`, here it is not checked whether the specified asset is liquid, but it is expected that this is the case and it attempts to determine the net worth using the asset's price feed.
    fn calculate_liquid_asset_net_worth(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

    /// Calculates the net worth of the given units of the given SAFT.
    ///
    /// In contrast to `calculate_asset_net_worth`, here it is not checked whether the specified asset is SAFT. The net worth is then determined by the tracked `SAFTRecords`
    fn calculate_saft_net_worth(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

    /// Calculates the net worth of the given asset contributed to the index.
    ///
    /// The net worth of an asset is determined by multiplying the share price
    /// of the asset by the amount deposited in the index.: `Price_asset * Index
    /// Deposit`
    fn asset_net_worth(asset: AssetId) -> Result<Balance, DispatchError> {
        Self::calculate_asset_net_worth(asset.clone(), Self::asset_balance(asset))
    }

    /// Calculates the net worth of all assets combined.
    fn total_asset_net_worth() -> Result<Balance, DispatchError>;

    /// Calculates the net worth of all liquid assets combined.
    fn liquid_net_worth() -> Result<Balance, DispatchError>;

    /// Calculates the net worth of all SAFT combined.
    fn saft_net_worth() -> Result<Balance, DispatchError>;

    /// Calculates the total NAV of the index token, consiting of liquid assets and SAFT
    ///
    /// The NAV represents the fund's per token value.
    /// The the NAV is calculated by dividing the total value of all the
    /// contributed assets by the total supply of index token:
    /// `NAV = (Asset_0 + Asset_1+ ... + Asset_n) / Total Supply`. where
    /// `Asset_n` is the net worth of all shares of the specific asset that were
    /// contributed to the index. And the sum of all of them is the `total_asset_net_worth`
    ///
    /// To determine the net value of the assets, and then ultimately the `NAV`,
    /// the prices of the assets are required.
    fn total_nav() -> Result<Balance, DispatchError>;

    /// Calculates the NAV of the index token solely based on the liquid assets
    fn liquid_nav() -> Result<Balance, DispatchError>;

    /// Calculates the NAV of the index token solely based on the SAFT
    fn saft_nav() -> Result<Balance, DispatchError>;

    /// The total supply of index tokens currently in circulation.
    fn index_token_issuance() -> Balance;

    /// The amount of the specified asset currently held in the index.
    fn asset_balance(asset: AssetId) -> Balance;
}

/// Outcome of an XCM unbonding api call
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug)]
pub enum UnbondingOutcome {
    /// Staking is not supported, therefore nothing to unbond
    NotSupported,
    /// Staking is supported, but the parachain's reserve account currently
    /// holds enough units as stash so that no unbonding procedure is necessary
    SufficientReserve,
    /// The outcome of the XCM unbond call
    Outcome(Box<Outcome>),
}
