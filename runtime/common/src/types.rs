// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use frame_support::sp_runtime::{
    app_crypto::sp_core,
    generic,
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on
/// the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it
/// equivalent to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of
/// them, but you never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Signed version of Balance
pub type Amount = i128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Identifier for an asset.
pub type AssetId = u32;

/// Identifier for price feeds.
pub type FeedId = u64;

/// Value type for price feeds.
pub type Value = u128;
