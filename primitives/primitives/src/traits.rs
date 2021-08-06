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
	///     - an XCM InitiateReserveWithdraw followed by XCM Deposit order, if the location of the
	///       asset is a reserve location of PINT (Relay Chain)
	///     - an XCM InitiateReserveWithdraw followed by XCM DepositReserveAsset order will be
	///       dispatched as XCM ReserveAssetDeposit with an Xcm Deposit order
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

/// Abstracts net asset value (`NAV`) related calculations
pub trait NavProvider<AssetId: Clone, Balance> {
	/// Calculates the amount of index tokens that the given units of the asset
	/// are value.
	///
	/// This is achieved by dividing the value of the given units by the NAV.
	/// The value, or volume, is determined by `vol_asset = units * Price_asset`
	/// (`asset_net_value`), and since the `NAV` represents the per token value,
	/// the equivalent number of index token is `vol_asset / NAV`.
	fn index_token_equivalent(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

	/// Calculates the units of the given asset that the given number of
	/// index_tokens are value.
	///
	/// This is calculated by determine the net value of the given
	/// `index_tokens` and dividing it by the price of the `asset`.
	/// (`NAV * index_tokens) / Price_asset`
	fn asset_equivalent(index_tokens: Balance, asset: AssetId) -> Result<Balance, DispatchError>;

	/// Calculates the net value of the given units of the given asset.
	/// .
	/// If the asset is liquid then the net value of an asset is determined by
	/// multiplying the share price of the asset by the given amount.: `units *
	/// Price_asset`.
	///
	/// If the asset is liquid then the net value is determined by the net value
	/// of the associated `SAFTRecords`.
	fn calculate_net_asset_value(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

	/// Calculates the net value of the given units of the given *liquid* asset.
	///
	/// In contrast to `calculate_asset_net_value`, here it is not checked
	/// whether the specified asset is liquid, but it is expected that this is
	/// the case and it attempts to determine the net value using the asset's
	/// price feed.
	fn calculate_net_liquid_value(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

	/// Calculates the net value of the given units of the given SAFT.
	///
	/// In contrast to `calculate_asset_net_value`, here it is not checked
	/// whether the specified asset is SAFT. The net value is then determined by
	/// the tracked `SAFTRecords`
	fn calculate_net_saft_value(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

	/// Calculates the net value of the given asset that were contributed to the index.
	///
	/// The net value of an asset is determined by multiplying the share price
	/// of the asset by the amount deposited in the index.: `Price_asset * Index
	/// Deposit`
	fn net_asset_value(asset: AssetId) -> Result<Balance, DispatchError> {
		Self::calculate_net_asset_value(asset.clone(), Self::asset_balance(asset))
	}

	/// Calculates the net value of all liquid assets combined.
	///
	/// This is essentially the sum of the value of all liquid assets:
	/// `net_liquid_value(Asset_0) + net_liquid_value(Asset_1) ...`
	fn total_net_liquid_value() -> Result<Balance, DispatchError>;

	/// Calculates the net value of all SAFT combined.
	///
	/// This is essentially the sum of the value of all SAFTs:
	/// `net_saft_value(Asset_0) + net_saft_value(Asset_1) ...`
	fn total_net_saft_value() -> Result<Balance, DispatchError>;

	/// Calculates the net asset value of all the index tokens which is equal to the
	/// sum of the total value of all assets.
	///
	/// Since the `NAV` represents the per index token value, net value of all
	/// index tokens is the product of the `NAV` and the total supply of index
	/// tokens: `NAV * index_token_issuance`.
	/// Or Simplified:
	/// `total_net_liquid_value + total_net_saft_value`.
	fn total_net_asset_value() -> Result<Balance, DispatchError>;

	/// Calculates the net value of the given liquid asset.
	///
	/// In contrast to `net_asset_value`, here it is not checked whether the
	/// specified asset is liquid.
	fn net_liquid_value(asset: AssetId) -> Result<Balance, DispatchError> {
		Self::calculate_net_liquid_value(asset.clone(), Self::asset_balance(asset))
	}

	/// Calculates the net value of the given SAFT.
	///
	/// In contrast to `net_asset_value`, here it is not checked whether the
	/// specified asset is a SAFT.
	fn net_saft_value(asset: AssetId) -> Result<Balance, DispatchError> {
		Self::calculate_net_saft_value(asset.clone(), Self::asset_balance(asset))
	}

	/// Calculates the `NAV` of the index token, consisting of liquid assets
	/// and SAFT. This the *per token value* (value of a single unit of index token)
	///
	/// The the NAV is calculated by dividing the total value of all the
	/// contributed assets by the total supply of index token:
	/// `NAV = (NAV_0 + NAV_1+ ... + NAV_n) / Total Supply`. where
	/// `Asset_n` is the net value of all shares of the specific asset that were
	/// contributed to the index. And the sum of all of them is the
	/// `total_asset_net_value`
	///
	/// This can be simplified to
	/// `NAV = (Liquid_net_value + SAFT_net_value) / Total Supply`,
	/// which is also `NAV = NAV_liquids + NAV_saft`.
	fn nav() -> Result<Balance, DispatchError>;

	/// Calculates the NAV of the index token solely for the liquid assets.
	/// This is a *per token value*: the value of a single unit of index token for the funds total
	/// liquid value.
	///
	/// Following the `total_nav` calculation, the `NAV_liquids` is determined
	/// by `NAV_liquids = NAV - (SAFT_net_value / Total Supply)`
	/// Or simplified
	/// `NAV - NAV_saft`, which is  `Liquid_net_value / Total Supply`
	fn liquid_nav() -> Result<Balance, DispatchError>;

	/// Calculates the NAV of the index token solely for the SAFT
	/// This is a *per token value*: the value of a single unit of index token for the funds total
	/// SAFT value.
	///
	/// Following `liquid_nav` calculation, this is determined by:
	/// `SAFT_net_value / Total Supply`
	fn saft_nav() -> Result<Balance, DispatchError>;

	/// The total supply of index tokens currently in circulation.
	fn index_token_issuance() -> Balance;

	/// The amount of the given asset currently held in the index.
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
