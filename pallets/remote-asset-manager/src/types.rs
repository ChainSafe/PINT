// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Additional types for the remote asset manager pallet
use codec::{Compact, Decode, Encode, EncodeLike, HasCompact};
use xcm::v0::Outcome as XcmOutcome;

use crate::EncodeWith;
use frame_support::{
    dispatch::Output,
    sp_runtime::{MultiAddress, RuntimeDebug},
    sp_std::{marker::PhantomData, prelude::*},
    weights::constants::RocksDbWeight,
    weights::Weight,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// A Wrapper around an already encoded item that does not include the item's length when encoded
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub struct WrappedEncoded(pub Vec<u8>);

impl Encode for WrappedEncoded {
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        for item in &self.0 {
            item.encode_to(dest);
        }
    }
}

impl From<Vec<u8>> for WrappedEncoded {
    fn from(encoded: Vec<u8>) -> Self {
        WrappedEncoded(encoded)
    }
}

/// Encodes a `u128` using `CompactRef` regardless of the asset id
pub struct CompactU128BalanceEncoder<T>(PhantomData<T>);

impl<AssetId> EncodeWith<u128, AssetId> for CompactU128BalanceEncoder<AssetId> {
    fn encode_to_with<T: Output + ?Sized>(balance: &u128, _: &AssetId, dest: &mut T) {
        Compact(*balance).encode_to(dest)
    }
}

/// Encodes an `AccountId` as `Multiaddress` regardless of the asset id
pub struct MultiAddressLookupSourceEncoder<AssetId, AccountId, AccountIndex>(
    PhantomData<(AssetId, AccountId, AccountIndex)>,
);

impl<AssetId, AccountId, AccountIndex> EncodeWith<AccountId, AssetId>
    for MultiAddressLookupSourceEncoder<AssetId, AccountId, AccountIndex>
where
    AccountId: Encode + Clone,
    AccountIndex: HasCompact,
{
    fn encode_to_with<T: Output + ?Sized>(account: &AccountId, _: &AssetId, dest: &mut T) {
        MultiAddress::<AccountId, AccountIndex>::from(account.clone()).encode_to(dest)
    }
}

/// The index of `pallet_staking` in the polkadot runtime
pub const POLKADOT_PALLET_PROXY_INDEX: u8 = 29u8;

/// The identifier the `ProxyType::Staking` variant encodes to
pub const POLKADOT_PALLET_PROXY_TYPE_STAKING_INDEX: u8 = 3u8;

/// Represents dispatchable calls of the FRAME `pallet_proxy` pallet.
#[derive(Encode)]
pub enum ProxyCall<AccountId, ProxyType, BlockNumber> {
    /// The [`add_proxy`](https://crates.parity.io/pallet_proxy/pallet/enum.Call.html#variant.add_proxy) extrinsic.
    ///
    /// Register a proxy account for the sender that is able to make calls on its behalf.
    #[codec(index = 1)]
    AddProxy {
        /// The account that the `caller` would like to make a proxy.
        proxy: AccountId,
        /// The permissions allowed for this proxy account.
        proxy_type: ProxyType,
        /// The announcement period required of the initial proxy. Will generally be zero
        delay: BlockNumber,
    },
}
