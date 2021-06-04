// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Additional types for the remote asset manager pallet
use codec::{Compact, Decode, Encode, EncodeLike, HasCompact};
use xcm::v0::Outcome as XcmOutcome;

use crate::EncodeWith;
use frame_support::dispatch::EncodeAsRef;
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

/// Encodes the type as it is
pub struct PassthroughEncoder<I, T>(PhantomData<(I, T)>);

impl<I: Encode, Context> EncodeWith<I, Context> for PassthroughEncoder<I, Context> {
    fn encode_to_with<T: Output + ?Sized>(input: &I, _: &Context, dest: &mut T) {
        input.encode_to(dest)
    }
}

/// Encodes the type as it is but compact
pub struct PassthroughCompactEncoder<I, T>(PhantomData<(I, T)>);

impl<I: HasCompact, Context> EncodeWith<I, Context> for PassthroughCompactEncoder<I, Context> {
    fn encode_to_with<T: Output + ?Sized>(input: &I, _: &Context, dest: &mut T) {
        <<I as HasCompact>::Type as EncodeAsRef<'_, I>>::RefType::from(input).encode_to(dest)
    }
}

/// Encodes an `AccountId` as `Multiaddress` regardless of the asset id
pub struct MultiAddressLookupSourceEncoder<AccountId, AccountIndex, Context>(
    PhantomData<(AccountId, AccountIndex, Context)>,
);

impl<AccountId, AccountIndex, Context> EncodeWith<AccountId, Context>
    for MultiAddressLookupSourceEncoder<AccountId, AccountIndex, Context>
where
    AccountId: Encode + Clone,
    AccountIndex: HasCompact,
{
    fn encode_to_with<T: Output + ?Sized>(account: &AccountId, _: &Context, dest: &mut T) {
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
