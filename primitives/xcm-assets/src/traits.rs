// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Supporting traits for handling cross-chain messages (XCM) in PINT

use xcm::v0::Outcome;

pub enum Error {
    /// Thrown when conversion from accountId to MultiLocation failed
    BadLocation,
    /// Can't transfer to the provided location.
    InvalidDestination,
    /// Thrown when the destination of a requested cross-chain transfer is the location of
    /// the local chain itself
    NoCrossChainTransfer,
    /// Failed to convert the provided currency into a location
    NotCrossChainTransferableAsset,
}

pub trait XcmAssetHandler<AccountId, Balance, AssetId> {
    /// Execute an XCM to remove the given amount from the PINT sovereign account on the target
    /// system and deposit them into the account of the sender
    fn execute_xcm_transfer(
        who: AccountId,
        asset_id: AssetId,
        amount: Balance,
    ) -> frame_support::sp_std::result::Result<Outcome, Error>;
}
