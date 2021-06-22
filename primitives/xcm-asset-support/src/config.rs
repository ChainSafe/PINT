// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::weights::Weight;
use frame_support::{
    sp_runtime::traits::{Convert, Member},
    traits::Get,
};
use xcm::v0::{ExecuteXcm, MultiLocation};

/// The config trait to parametrize the xcm asset handling.
pub trait Config {
    /// The aggregated `Call` type.
    type Call;

    /// The identifier for an asset
    type AssetId: Member;

    /// Convert a `AssetId` to its relative `MultiLocation` identifier.
    type AssetIdConvert: xcm_executor::traits::Convert<Self::AssetId, MultiLocation>;

    /// The native asset id of this chain (PINT).
    type SelfAssetId: Get<Self::AssetId>;

    /// The account type used locally.
    type AccountId: Member;

    /// The type used to represent amounts of assets.
    type Amount;

    /// Convert the amount type into an `u128` as required by `Xcm`
    type AmountU128Convert: Convert<Self::Amount, u128>;

    /// Convert an `AccountId` to `AccountId32` for cross chain messages
    type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

    /// The location of the chain itself.
    type SelfLocation: Get<MultiLocation>;

    /// Executor for cross chain messages.
    type XcmExecutor: ExecuteXcm<Self::Call>;

    // TODO determine weight
    type WeightLimit: Get<Weight>;
}
