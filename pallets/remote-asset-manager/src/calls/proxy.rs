// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Xcm support for `pallet_proxy` calls
use crate::{CallEncoder, EncodeWith, PalletCall, PalletCallEncoder};
use codec::{Compact, Decode, Encode, Output};
use frame_support::{sp_std::marker::PhantomData, RuntimeDebug};

/// The index of `pallet_proxy` in the polkadot runtime
pub const POLKADOT_PALLET_PROXY_INDEX: u8 = 29u8;

/// The identifier the `ProxyType::Staking` variant encodes to
pub const POLKADOT_PALLET_PROXY_TYPE_STAKING_INDEX: u8 = 3u8;

/// Provides encoder types to encode the associated types of the  `pallet_proxy::Config` trait depending on the configured Context.
pub trait ProxyCallEncoder<AccountId, ProxyType, BlockNumber>: PalletCallEncoder {
    /// Encodes the `<pallet_proxy::Config>::AccountId` depending on the context
    type AccountIdEncoder: EncodeWith<AccountId, Self::Context>;

    /// Encodes the `<pallet_proxy::Config>::ProxyType` depending on the context
    type ProxyTypeEncoder: EncodeWith<ProxyType, Self::Context>;

    /// Encodes the `<pallet_proxy::Config>::BlockNumber` depending on the context
    type BlockNumberEncoder: EncodeWith<BlockNumber, Self::Context>;
}

impl<'a, 'b, AccountId, ProxyType, BlockNumber, Config> Encode
    for CallEncoder<'a, 'b, ProxyCall<AccountId, ProxyType, BlockNumber>, Config>
where
    Config: ProxyCallEncoder<AccountId, ProxyType, BlockNumber>,
{
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        // include the pallet identifier
        dest.push_byte(self.call.pallet_call_index());
        match self.call {
            ProxyCall::AddProxy(params) | ProxyCall::RemoveProxy(params) => {
                Config::AccountIdEncoder::encode_to_with(&params.delegate, self.ctx, dest);
                Config::ProxyTypeEncoder::encode_to_with(&params.proxy_type, self.ctx, dest);
                Config::BlockNumberEncoder::encode_to_with(&params.delay, self.ctx, dest);
            }
        }
    }
}

/// Represents dispatchable calls of the FRAME `pallet_proxy` pallet.
#[derive(Clone, PartialEq, RuntimeDebug)]
pub enum ProxyCall<AccountId, ProxyType, BlockNumber> {
    /// The [`add_proxy`](https://crates.parity.io/pallet_proxy/pallet/enum.Call.html#variant.add_proxy) extrinsic.
    ///
    /// Register a proxy account for the sender that is able to make calls on its behalf.
    AddProxy(ProxyParams<AccountId, ProxyType, BlockNumber>),
    /// The [`remove_proxy`](https://crates.parity.io/pallet_proxy/pallet/enum.Call.html#variant.remove_proxy) extrinsic.
    ///
    /// Unregister a proxy account for the sender..
    RemoveProxy(ProxyParams<AccountId, ProxyType, BlockNumber>),
}

#[derive(Clone, PartialEq, RuntimeDebug)]
pub struct ProxyParams<AccountId, ProxyType, BlockNumber> {
    /// The account that the `caller` would like to make a proxy.
    delegate: AccountId,
    /// The permissions to add/remove for this proxy account.
    proxy_type: ProxyType,
    /// The announcement period required of the initial proxy. Will generally be zero
    delay: BlockNumber,
}

impl<AccountId, ProxyType, BlockNumber> PalletCall
    for ProxyCall<AccountId, ProxyType, BlockNumber>
{
    /// the indices of the corresponding calls within the `pallet_proxy`
    fn pallet_call_index(&self) -> u8 {
        match self {
            ProxyCall::AddProxy(_) => 1,
            ProxyCall::RemoveProxy(_) => 2,
        }
    }
}
