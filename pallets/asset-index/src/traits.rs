// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pub use crate::types::{AssetAvailability, AssetMetadata};
use frame_support::{dispatch::DispatchResult, sp_runtime::traits::AtLeast32BitUnsigned};

pub trait AssetRecorder<AccountId, AssetId, Balance> {
    /// Add an asset into the index.
    /// If an asset with the given AssetId does not already exist, it will be registered.
    /// This moves the given units from the caller's balance into the index's and issues PINT accordingly.
    fn add_asset(
        caller: &AccountId,
        id: AssetId,
        units: Balance,
        nav: Balance,
        availability: AssetAvailability,
    ) -> DispatchResult;

    /// Mints the SAFT into the index and awards the caller with given amount of PINT token.
    /// If an asset with the given AssetId does not already exist, it will be registered as SAFT.
    /// Fails if the availability of the asset is liquid.
    fn add_saft(caller: &AccountId, id: AssetId, units: Balance, nav: Balance) -> DispatchResult;

    /// Sets the availability of the given asset.
    /// If the asset was already registered, the old `AssetAvailability` is returned.
    fn insert_asset_availability(
        asset_id: AssetId,
        availability: AssetAvailability,
    ) -> Option<AssetAvailability>;

    /// Dispatches transfer to move assets out of the index’s account, if a liquid asset is specified, burns the SAFT otherwise.
    /// Updates the index by burning the given amount of index token.
    fn remove_asset(
        who: AccountId,
        id: AssetId,
        units: Balance,
        nav: Balance,
        recipient: Option<AccountId>,
    ) -> DispatchResult;
}

/// Type that calculations any fees to be deducted for every withdrawal.
pub trait WithdrawalFee<Balance> {
    /// Calculates the fee to be deducted from the PINT being redeemed
    // TODO specify parameters
    fn withdrawal_fee(amount: Balance) -> Balance;
}

impl<Balance: AtLeast32BitUnsigned> WithdrawalFee<Balance> for () {
    fn withdrawal_fee(_: Balance) -> Balance {
        Balance::zero()
    }
}
