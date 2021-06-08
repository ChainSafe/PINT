// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Xcm support for `pallet_assets` calls
use codec::{Decode, Encode, Output};
use frame_support::{sp_std::vec::Vec, weights::Weight, RuntimeDebug};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use crate::{EncodeWith, PalletCallEncoder};

/// The index of `pallet_assets` in the statemint runtime
pub const STATEMINT_PALLET_ASSETS_INDEX: u8 = 50u8;

/// Provides encoder types to encode the associated types of the  `pallet_assets::Config` trait depending on the configured Context.
pub trait AssetsCallEncoder<AssetId, Source, Balance>: PalletCallEncoder {
    /// Encodes the `<pallet_assets::Config>::AssetId` depending on the context
    type CompactAssetIdIdEncoder: EncodeWith<AssetId, Self::Context>;

    /// Encodes the `<pallet_assets::Config>::Source` depending on the context
    type SourceEncoder: EncodeWith<Source, Self::Context>;

    /// Encodes the `<pallet_assets::Config>::Balance` depending on the context
    type CompactBalanceEncoder: EncodeWith<Balance, Self::Context>;
}

/// Represents dispatchable calls of the FRAME `pallet_assets` pallet.
///
/// *NOTE*: `Balance` and `AssetId` are expected to encode with `HasCompact`
#[derive(Clone, PartialEq, RuntimeDebug)]
pub enum AssetsCall<AssetId, Source, Balance> {
    /// The [`mint`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.mint) extrinsic.
    ///
    /// Mint assets of a particular class.
    ///
    /// The origin must be Signed and the sender must be the Issuer of the asset id.
    // #[codec(index = 3)]
    Mint(AssetParam<AssetId, Source, Balance>),
    /// The [`burn`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.burn) extrinsic.
    ///
    /// Reduce the balance of who by as much as possible up to amount assets of id.
    ///
    /// Origin must be Signed and the sender should be the Manager of the asset id.
    // #[codec(index = 4)]
    Burn(AssetParam<AssetId, Source, Balance>),
    /// The [`transfer`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.transfer) extrinsic.
    ///
    /// Move some assets from the sender account to another.
    // #[codec(index = 5)]
    Transfer(AssetParam<AssetId, Source, Balance>),
    /// The [`force_transfer`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.force_transfer) extrinsic.
    ///
    /// Same as `Transfer` but debit the source instead of the origin
    // #[codec(index = 7)]
    ForceTransfer(Source, AssetParam<AssetId, Source, Balance>),
    /// The [`freeze`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.freeze) extrinsic.
    ///
    /// Disallow further unprivileged transfers from an account.
    // #[codec(index = 8)]
    Freeze(AssetId, Source),
    /// The [`thaw`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.thaw) extrinsic.
    ///
    /// Allow unprivileged transfers from an account again.
    // #[codec(index = 9)]
    Thaw(AssetId, Source),
    /// The [`freeze_asset`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.freeze_asset) extrinsic.
    ///
    /// Disallow further unprivileged transfers for the asset class.
    // #[codec(index = 10)]
    FreezeAsset(AssetId),
    /// The [`freeze_asset`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.freeze_asset) extrinsic.
    ///
    /// Allow unprivileged transfers for the asset again.
    // #[codec(index = 11)]
    ThawAsset(AssetId),
    /// The [`approve_transfer`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.approve_transfer) extrinsic.
    ///
    /// Approve an amount of asset for transfer by a delegated third-party account.
    /// \[id, delegate, amount \]
    // #[codec(index = 19)]
    ApproveTransfer(AssetParam<AssetId, Source, Balance>),
    /// The [`cancel_approval`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.cancel_approval) extrinsic.
    ///
    /// Cancel all of some asset approved for delegated transfer by a third-party account.
    /// \[id, delegate \]
    // #[codec(index = 20)]
    CancelApproval(AssetId, Source),
    /// The [`transfer_approved`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.transfer_approved) extrinsic.
    ///
    /// Transfer some asset balance from a previously delegated account to some third-party account.
    /// \[id, owner, destination, amount \]
    // #[codec(index = 21)]
    TransferApproved(AssetId, Source, Source, Balance),
}

/// Represents the parameters for common `AssetsCall`
#[derive(Clone, PartialEq, RuntimeDebug)]
pub struct AssetParam<AssetId, Source, Balance> {
    /// The identifier for the asset.
    pub id: AssetId,
    /// The lookup type of the targeted account,
    pub beneficiary: Source,
    /// The amount of assets
    pub amount: Balance,
}
