// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! This contains shared traits that are used in multiple pallets to prevent
//! circular dependencies

#[cfg(feature = "runtime-benchmarks")]
use frame_support::dispatch::DispatchResultWithPostInfo;

use crate::{AssetAvailability, AssetPricePair, AssetProportions, Price, Ratio};
use frame_support::{
	dispatch::DispatchError,
	sp_runtime::{app_crypto::sp_core::U256, traits::AtLeast32BitUnsigned, DispatchResult},
	sp_std::result::Result,
};
use xcm::v0::MultiLocation;

/// Type that provides the mapping between `AssetId` and `MultiLocation`.
pub trait MultiAssetRegistry<AssetId> {
	/// Determines the relative location of the consensus system where the given
	/// asset is native from the point of view of the current system
	fn native_asset_location(asset: &AssetId) -> Option<MultiLocation>;

	/// Whether the given identifier is currently supported as a liquid asset
	fn is_liquid_asset(asset: &AssetId) -> bool;
}

/// Facility for remote asset operations.
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
	fn transfer_asset(who: AccountId, asset: AssetId, amount: Balance) -> DispatchResult;

	/// Notification of deposited funds in the index, ready to be `bond` to earn staking rewards.
	///
	/// This is an abstraction over how staking is supported on the `asset`'s native location.
	/// In general, this can be one of
	///     - None, staking is not supported, meaning this asset is idle.
	///     - Staking via the FRAME `pallet_staking`, (e.g. Relay Chain).
	///     - Liquid Staking, with support for early unbonding.
	fn deposit(asset: AssetId, amount: Balance);

	/// Notification of an upcoming withdrawal.
	/// This tells the manager to either reserve the given amount from the free remote balance or
	/// prepare to unbond those funds.
	///
	/// Unbonding an asset will involve:
	///     - Nothing for assets that do not support staking (idle asset).
	///     - Call `pallet_staking::unbond` + `pallet_staking::withdraw` on the asset's native chain
	///       (e.g Relay Chain)
	///     - Execute the unbond mechanism of the liquid staking protocol
	fn announce_withdrawal(asset: AssetId, amount: Balance);
}

// Default implementation that does nothing
impl<AccountId, AssetId, Balance> RemoteAssetManager<AccountId, AssetId, Balance> for () {
	fn transfer_asset(_: AccountId, _: AssetId, _: Balance) -> DispatchResult {
		Ok(())
	}

	fn deposit(_: AssetId, _: Balance) {}

	fn announce_withdrawal(_: AssetId, _: Balance) {}
}

/// Abstracts net asset value (`NAV`) related calculations
pub trait NavProvider<AssetId: Clone, Balance>: SaftRegistry<AssetId, Balance> {
	/// Calculates the amount of index tokens that the given units of the asset
	/// are worth.
	///
	/// This is achieved by dividing the value of the given units by the index' `NAV`.
	/// The value, or volume, of the `units` is determined by `value(units) = units * Price_asset`
	/// (see: `asset_net_value`), and since the `NAV` represents the per token value, the equivalent
	/// number of index token is `vol_asset / NAV`.
	fn index_token_equivalent(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

	/// Calculates the units of the given asset that the given number of
	/// index_tokens are worth.
	///
	/// This is calculated by determining the net value of the given
	/// `index_tokens` and dividing it by the current priceof the `asset`:
	/// `units_asset = (NAV * index_tokens) / Price_asset`
	fn asset_equivalent(index_tokens: Balance, asset: AssetId) -> Result<Balance, DispatchError>;

	/// Returns the price of the asset relative to the `NAV` of the index token.
	///
	/// This is a price pair in the form of `base/quote` whereas `base` is the `NAV` of the index
	/// token and `quote` the current price for the asset:  `NAV / Price_asset`.
	///
	/// *Note:* The price (or value of 1 unit) of an asset secured by SAFTs is determined by the
	/// total asset value secured by all SAFTs divided by the units held in the index, (see:
	/// [`SaftRegistry::net_saft_value`])
	fn relative_asset_price(asset: AssetId) -> Result<AssetPricePair<AssetId>, DispatchError>;

	/// Calculates the net value of the given units of the given asset.
	///
	/// If the asset is liquid then the net value of an asset is determined by
	/// multiplying the share price of the asset by the given amount.: `units *
	/// Price_asset`.
	///
	/// If the asset is secured by SAFTs then the net value is determined by the net value
	/// of the associated `SAFTRecords`, (see: [`SaftRegistry::net_saft_value`]).
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
	/// whether the specified asset is secured by SAFT.
	/// The net value is then determined by  [`SaftRegistry::net_saft_value`]
	fn calculate_net_saft_value(asset: AssetId, units: Balance) -> Result<Balance, DispatchError>;

	/// Calculates the net value of all liquid assets combined.
	///
	/// This is essentially the sum of the value of all liquid assets:
	/// `net_liquid_value(Asset_0) + net_liquid_value(Asset_1) ...`
	fn total_net_liquid_value() -> Result<U256, DispatchError>;

	/// Calculates the net value of all SAFT combined.
	///
	/// This is essentially the sum of the value of all SAFTs:
	/// `net_saft_value(Asset_0) + net_saft_value(Asset_1) ...`
	fn total_net_saft_value() -> Result<U256, DispatchError>;

	/// Calculates the net asset value of all the index tokens which is equal to the
	/// sum of the total value of all assets.
	///
	/// Since the `NAV` represents the per index token value, net value of all
	/// index tokens is the product of the `NAV` and the total supply of index
	/// tokens: `NAV * index_token_issuance`.
	/// Or Simplified:
	/// `total_net_liquid_value + total_net_saft_value`.
	fn total_net_asset_value() -> Result<U256, DispatchError>;

	/// Calculates the net value of the given liquid asset.
	///
	/// In contrast to `net_asset_value`, here it is not checked whether the
	/// specified asset is liquid.
	fn net_liquid_value(asset: AssetId) -> Result<Balance, DispatchError> {
		Self::calculate_net_liquid_value(asset.clone(), Self::asset_balance(asset))
	}

	/// Calculates the net value of the given asset that were contributed to the index.
	///
	/// The net value of an asset is determined by multiplying the share price
	/// of the asset by the amount deposited in the index.: `Price_asset * Index
	/// Deposit`
	fn net_asset_value(asset: AssetId) -> Result<Balance, DispatchError>;

	/// Calculates the `NAV` of the index token, consisting of liquid assets
	/// and SAFT.
	/// This the *per token value* (value of a single unit of index token, or it's
	/// "price")
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
	fn nav() -> Result<Price, DispatchError>;

	/// Returns the per token `NAV` of the index token split to (`liquid`, `saft`).
	/// Summed up, both of them add up to the total nav [`NavProvider::nav`]
	fn navs() -> Result<(Price, Price), DispatchError> {
		Ok((Self::liquid_nav()?, Self::saft_nav()?))
	}

	/// Calculates the NAV of the index token solely for the liquid assets.
	/// This is a *per token value*: the value of a single unit of index token for the funds total
	/// liquid value.
	///
	/// Following the `total_nav` calculation, the `NAV_liquids` is determined
	/// by `NAV_liquids = NAV - (SAFT_net_value / Total Supply)`
	/// Or simplified
	/// `NAV - NAV_saft`, which is  `Liquid_net_value / Total Supply`
	fn liquid_nav() -> Result<Price, DispatchError>;

	/// Calculates the NAV of the index token solely for the SAFT
	/// This is a *per token value*: the value of a single unit of index token for the funds total
	/// SAFT value.
	///
	/// Following `liquid_nav` calculation, this is determined by:
	/// `SAFT_net_value / Total Supply`
	fn saft_nav() -> Result<Price, DispatchError>;

	/// Returns the share of the asset in the total value of the index:
	/// `Asset Value / Total Net Asset Value`
	fn asset_proportion(asset: AssetId) -> Result<Ratio, DispatchError>;

	/// Returns the share of the liquid asset in the total value of all liquid assets in the index:
	/// `Asset Value / Liquid Net Asset Value`
	fn liquid_asset_proportion(asset: AssetId) -> Result<Ratio, DispatchError>;

	/// Returns the share of the asset in the total value of all SAFTs of the asset in the index:
	/// `Asset Value / SAFT Net Asset Value`
	fn saft_asset_proportion(asset: AssetId) -> Result<Ratio, DispatchError>;

	/// Returns the proportions for each asset in the index
	fn asset_proportions() -> Result<AssetProportions<AssetId>, DispatchError>;

	/// Returns the proportions for each liquid asset in total value of liquid assets in the index
	fn liquid_asset_proportions() -> Result<AssetProportions<AssetId>, DispatchError>;

	/// Returns the proportions for each saft asset in total value of SAFTs in the index
	fn saft_asset_proportions() -> Result<AssetProportions<AssetId>, DispatchError>;

	/// The total supply of index tokens currently in circulation.
	fn index_token_issuance() -> Balance;

	/// The amount of the given asset currently held in the index.
	fn asset_balance(asset: AssetId) -> Balance;
}

/// Abstracts SAFT related information
pub trait SaftRegistry<AssetId, Balance> {
	/// Returns the value of the assets currently secured by the SAFTS
	fn net_saft_value(asset: AssetId) -> Balance;
}

/// Abstract core features of the `AssetIndex` shared across pallets.
pub trait AssetRecorder<AccountId, AssetId, Balance> {
	/// Add an liquid asset into the index.
	/// This moves the given units from the caller's balance into the index's
	/// and issues PINT accordingly.
	fn add_liquid(caller: &AccountId, id: AssetId, units: Balance, nav: Balance) -> DispatchResult;

	/// Mints the SAFT into the index and awards the caller with given amount of
	/// PINT token.
	/// If an asset with the given AssetId does not already
	/// exist, it will be registered as SAFT. Fails if the availability of
	/// the asset is liquid.
	fn add_saft(caller: &AccountId, id: AssetId, units: Balance, nav: Balance) -> DispatchResult;

	/// Sets the availability of the given asset.
	/// If the asset was already registered, the old `AssetAvailability` is
	/// returned.
	fn insert_asset_availability(asset_id: AssetId, availability: AssetAvailability) -> Option<AssetAvailability>;

	/// Dispatches transfer to move liquid assets out of the index’s account.
	/// Updates the index by burning the given amount of index token from
	/// the caller's account.
	fn remove_liquid(
		who: AccountId,
		id: AssetId,
		units: Balance,
		nav: Balance,
		recipient: Option<AccountId>,
	) -> DispatchResult;

	/// Burns the given amount of SAFT token from the index and
	/// the nav from the caller's account
	fn remove_saft(who: AccountId, id: AssetId, units: Balance, nav: Balance) -> DispatchResult;
}

#[cfg(feature = "runtime-benchmarks")]
pub trait AssetRecorderBenchmarks<AssetId, Balance> {
	fn add_asset(
		asset_id: AssetId,
		units: Balance,
		localtion: MultiLocation,
		amount: Balance,
	) -> DispatchResultWithPostInfo;
}

/// Determines the fee upon index token redemptions
pub trait RedemptionFee<BlockNumber, Balance: AtLeast32BitUnsigned> {
	/// Determines the redemption fee based on how long the given amount were held in the index
	///
	/// Parameters:
	///     - `time_spent`: The number of blocks the amount were held in the index. This is `current
	///       block -  deposit`.
	///     - `amount`: The amount of index tokens withdrawn
	fn redemption_fee(time_spent: BlockNumber, amount: Balance) -> Balance;
}

impl<BlockNumber, Balance: AtLeast32BitUnsigned> RedemptionFee<BlockNumber, Balance> for () {
	fn redemption_fee(_: BlockNumber, _: Balance) -> Balance {
		Balance::zero()
	}
}

/// This is a helper trait only used for constructing `AssetId` types in Runtime Benchmarks
pub trait MaybeAssetIdConvert<A, B>: Sized {
	#[cfg(feature = "runtime-benchmarks")]
	fn try_convert(value: A) -> Option<B>;
}

#[cfg(feature = "runtime-benchmarks")]
impl<T> MaybeAssetIdConvert<u8, crate::types::AssetId> for T {
	fn try_convert(value: u8) -> Option<crate::types::AssetId> {
		frame_support::sp_std::convert::TryFrom::try_from(value).ok()
	}
}

#[cfg(not(feature = "runtime-benchmarks"))]
impl<T, A, B> MaybeAssetIdConvert<A, B> for T {}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::AssetId;

	fn assert_maybe_from<T: MaybeAssetIdConvert<u8, AssetId>>() {}

	#[test]
	fn maybe_from_works() {
		assert_maybe_from::<()>();
	}
}
