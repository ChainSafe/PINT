// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Xcm support for `pallet_proxy` calls
use crate::{CallEncoder, EncodeWith, PalletCall, PalletCallEncoder};
use codec::{Decode, Encode, Output};
use frame_support::{weights::Weight, RuntimeDebug};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// The index of `pallet_proxy` in the polkadot runtime
pub const POLKADOT_PALLET_PROXY_INDEX: u8 = 29u8;

/// The identifier the `ProxyType::Staking` variant encodes to
pub const POLKADOT_PALLET_PROXY_TYPE_STAKING_INDEX: u8 = 3u8;

/// Denotes an enum based (identified by an `u8`) proxy type
#[derive(Encode, Decode, Copy, Clone, PartialEq, RuntimeDebug)]
pub struct ProxyType(pub u8);

impl ProxyType {
    /// Represents the `Staking` variant of the polkadot `ProxyType` enum
    pub const fn polkadot_staking() -> Self {
        ProxyType(POLKADOT_PALLET_PROXY_TYPE_STAKING_INDEX)
    }
}

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
    pub delegate: AccountId,
    /// The permissions to add/remove for this proxy account.
    pub proxy_type: ProxyType,
    /// The announcement period required of the initial proxy. Will generally be zero
    pub delay: BlockNumber,
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

/// Denotes the current state of proxies for the PINT chain's account
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, Default)]
pub struct ProxyState {
    /// All the added Proxy types
    pub added: Vec<ProxyType>,
}

impl ProxyState {
    /// Whether the given proxy is already set
    pub fn contains(&self, proxy: &ProxyType) -> bool {
        self.added.contains(&proxy)
    }

    /// Adds the proxy to the list
    ///
    /// *NOTE:* the caller must check `contains` first
    pub fn add(&mut self, proxy: ProxyType) {
        self.added.push(proxy)
    }
}

/// The `pallet_proxy` configuration for a particular chain
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ProxyConfig {
    /// The index of `pallet_index` within the parachain's runtime
    pub pallet_index: u8,
    /// The configured weights for `pallet_proxy`
    pub weights: ProxyWeights,
}

/// Represents an excerpt from the `pallet_proxy` weights
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ProxyWeights {
    /// Weight for `add_proxy` extrinsic
    pub add_proxy: Weight,
    /// Weight for `remove_proxy` extrinsic
    pub remove_proxy: Weight,
}
