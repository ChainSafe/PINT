// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Xcm support for [`pallet_assets`](https://crates.parity.io/pallet_assets/pallet/index.html) calls.
use codec::{Decode, Encode, Output};
use frame_support::{weights::Weight, RuntimeDebug};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use crate::{CallEncoder, EncodeWith, PalletCall, PalletCallEncoder};

/// The index of `pallet_assets` in the statemint runtime
pub const STATEMINT_PALLET_ASSETS_INDEX: u8 = 50u8;

/// Provides encoder types to encode the associated types of the
/// `pallet_assets::Config` trait depending on the configured Context.
pub trait AssetsCallEncoder<AssetId, Source, Balance>: PalletCallEncoder {
	/// Encodes the `<pallet_assets::Config>::AssetId` depending on the context
	type CompactAssetIdEncoder: EncodeWith<AssetId, Self::Context>;

	/// Encodes the `<pallet_assets::Config>::Source` depending on the context
	type SourceEncoder: EncodeWith<Source, Self::Context>;

	/// Encodes the `<pallet_assets::Config>::Balance` depending on the context
	type CompactBalanceEncoder: EncodeWith<Balance, Self::Context>;
}

impl<'a, 'b, AssetId, Source, Balance, Config> Encode
	for CallEncoder<'a, 'b, AssetsCall<AssetId, Source, Balance>, Config>
where
	Config: AssetsCallEncoder<AssetId, Source, Balance>,
{
	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		// include the pallet identifier
		dest.push_byte(self.call.pallet_call_index());
		match self.call {
			AssetsCall::Mint(params) => params.encode_with::<Config, _>(self.ctx, dest),
			AssetsCall::Burn(params) => params.encode_with::<Config, _>(self.ctx, dest),
			AssetsCall::Transfer(params) => params.encode_with::<Config, _>(self.ctx, dest),
			AssetsCall::ForceTransfer(id, source, destination, amount) => {
				Config::CompactAssetIdEncoder::encode_to_with(id, self.ctx, dest);
				Config::SourceEncoder::encode_to_with(source, self.ctx, dest);
				Config::SourceEncoder::encode_to_with(destination, self.ctx, dest);
				Config::CompactBalanceEncoder::encode_to_with(amount, self.ctx, dest);
			}
			AssetsCall::Freeze(asset, source) => {
				Config::CompactAssetIdEncoder::encode_to_with(asset, self.ctx, dest);
				Config::SourceEncoder::encode_to_with(source, self.ctx, dest);
			}
			AssetsCall::Thaw(asset, source) => {
				Config::CompactAssetIdEncoder::encode_to_with(asset, self.ctx, dest);
				Config::SourceEncoder::encode_to_with(source, self.ctx, dest);
			}
			AssetsCall::FreezeAsset(asset) => {
				Config::CompactAssetIdEncoder::encode_to_with(asset, self.ctx, dest);
			}
			AssetsCall::ThawAsset(asset) => {
				Config::CompactAssetIdEncoder::encode_to_with(asset, self.ctx, dest);
			}
			AssetsCall::ApproveTransfer(params) => params.encode_with::<Config, _>(self.ctx, dest),
			AssetsCall::CancelApproval(asset, source) => {
				Config::CompactAssetIdEncoder::encode_to_with(asset, self.ctx, dest);
				Config::SourceEncoder::encode_to_with(source, self.ctx, dest);
			}
			AssetsCall::TransferApproved(id, owner, destination, amount) => {
				Config::CompactAssetIdEncoder::encode_to_with(id, self.ctx, dest);
				Config::SourceEncoder::encode_to_with(owner, self.ctx, dest);
				Config::SourceEncoder::encode_to_with(destination, self.ctx, dest);
				Config::CompactBalanceEncoder::encode_to_with(amount, self.ctx, dest);
			}
		}
	}
}

/// Represents dispatchable calls of the FRAME `pallet_assets` pallet.
///
/// *NOTE*: `Balance` and `AssetId` are expected to encode with `HasCompact`
#[derive(Clone, PartialEq, RuntimeDebug, scale_info::TypeInfo)]
pub enum AssetsCall<AssetId, Source, Balance> {
	/// The [`mint`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.mint) extrinsic.
	///
	/// Mint assets of a particular class.
	///
	/// The origin must be Signed and the sender must be the Issuer of the asset
	/// id.
	// #[codec(index = 3)]
	Mint(AssetParams<AssetId, Source, Balance>),
	/// The [`burn`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.burn) extrinsic.
	///
	/// Reduce the balance of who by as much as possible up to amount assets of
	/// id.
	///
	/// Origin must be Signed and the sender should be the Manager of the asset
	/// id.
	// #[codec(index = 4)]
	Burn(AssetParams<AssetId, Source, Balance>),
	/// The [`transfer`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.transfer) extrinsic.
	///
	/// Move some assets from the sender account to another.
	// #[codec(index = 5)]
	Transfer(AssetParams<AssetId, Source, Balance>),
	/// The [`force_transfer`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.force_transfer) extrinsic.
	///
	/// Same as `Transfer` but debit the source instead of the origin
	/// \[id, source, dest, amount \]
	// #[codec(index = 7)]
	ForceTransfer(AssetId, Source, Source, Balance),
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
	/// Approve an amount of asset for transfer by a delegated third-party
	/// account. \[id, delegate, amount \]
	// #[codec(index = 19)]
	ApproveTransfer(AssetParams<AssetId, Source, Balance>),
	/// The [`cancel_approval`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.cancel_approval) extrinsic.
	///
	/// Cancel all of some asset approved for delegated transfer by a
	/// third-party account. \[id, delegate \]
	// #[codec(index = 20)]
	CancelApproval(AssetId, Source),
	/// The [`transfer_approved`](https://crates.parity.io/pallet_assets/pallet/enum.Call.html#variant.transfer_approved) extrinsic.
	///
	/// Transfer some asset balance from a previously delegated account to some
	/// third-party account. \[id, owner, destination, amount \]
	// #[codec(index = 22)]
	TransferApproved(AssetId, Source, Source, Balance),
}

impl<AssetId, Source, Balance> PalletCall for AssetsCall<AssetId, Source, Balance> {
	/// the indices of the corresponding calls within the `pallet_staking`
	fn pallet_call_index(&self) -> u8 {
		match self {
			AssetsCall::Mint(_) => 3,
			AssetsCall::Burn(_) => 4,
			AssetsCall::Transfer(_) => 5,
			AssetsCall::ForceTransfer(_, _, _, _) => 7,
			AssetsCall::Freeze(_, _) => 8,
			AssetsCall::Thaw(_, _) => 9,
			AssetsCall::FreezeAsset(_) => 10,
			AssetsCall::ThawAsset(_) => 11,
			AssetsCall::ApproveTransfer(_) => 19,
			AssetsCall::CancelApproval(_, _) => 20,
			AssetsCall::TransferApproved(_, _, _, _) => 22,
		}
	}
}

/// Represents common parameters the `AssetsCall` enum
#[derive(Clone, PartialEq, RuntimeDebug, scale_info::TypeInfo)]
pub struct AssetParams<AssetId, Source, Balance> {
	/// The identifier for the asset.
	pub id: AssetId,
	/// The lookup type of the targeted account,
	pub beneficiary: Source,
	/// The amount of assets
	pub amount: Balance,
}

impl<AssetId, Source, Balance> AssetParams<AssetId, Source, Balance> {
	/// encode the parameters with the given encoder set
	fn encode_with<Config, T>(&self, ctx: &Config::Context, dest: &mut T)
	where
		Config: AssetsCallEncoder<AssetId, Source, Balance>,
		T: Output + ?Sized,
	{
		Config::CompactAssetIdEncoder::encode_to_with(&self.id, ctx, dest);
		Config::SourceEncoder::encode_to_with(&self.beneficiary, ctx, dest);
		Config::CompactBalanceEncoder::encode_to_with(&self.amount, ctx, dest);
	}
}

/// The `pallet_assets` configuration for a particular chain
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AssetsConfig {
	/// The index of `pallet_index` within the parachain's runtime
	pub pallet_index: u8,
	/// The configured weights for `pallet_staking`
	pub weights: AssetsWeights,
}

/// Represents an excerpt from the `pallet_asset` weights
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AssetsWeights {
	/// Weight for `mint` extrinsic
	pub mint: Weight,
	/// Weight for `burn` extrinsic
	pub burn: Weight,
	/// Weight for `transfer` extrinsic
	pub transfer: Weight,
	/// Weight for `force_transfer` extrinsic
	pub force_transfer: Weight,
	/// Weight for `freeze` extrinsic
	pub freeze: Weight,
	/// Weight for `thaw` extrinsic
	pub thaw: Weight,
	/// Weight for `freeze_asset` extrinsic
	pub freeze_asset: Weight,
	/// Weight for `thaw_asset` extrinsic
	pub thaw_asset: Weight,
	/// Weight for `approve_transfer` extrinsic
	pub approve_transfer: Weight,
	/// Weight for `cancel_approval` extrinsic
	pub cancel_approval: Weight,
	/// Weight for `transfer_approved` extrinsic
	pub transfer_approved: Weight,
}
