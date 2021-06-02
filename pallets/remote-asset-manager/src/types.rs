// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Additional types for the remote asset manager pallet
use codec::{Decode, Encode, EncodeLike};
use frame_support::{dispatch::Output, sp_runtime::RuntimeDebug, sp_std::prelude::*};
use xcm::v0::Outcome as XcmOutcome;

use crate::traits::BalanceEncoder;
use frame_support::sp_std::marker::PhantomData;
use frame_support::weights::constants::RocksDbWeight;
use frame_support::weights::Weight;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use xcm::opaque::v0::Outcome;

/// Represents an extrinsic of a pallet configured inside a runtime
pub struct RuntimeCall<Call: Encode> {
    /// The index of the call's pallet within the runtime.
    ///
    /// This must be equivalent with the `#[codec(index = <pallet_index>)]` annotation.
    pub pallet_index: u8,

    /// The actual that should be dispatched
    pub call: Call,
}

impl<Call: EncodeLike> EncodeLike for RuntimeCall<Call> {}

impl<Call: EncodeLike> Encode for RuntimeCall<Call> {
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        dest.push_byte(self.pallet_index);
        self.call.encode_to(dest)
    }
}

/// Encodes a `u128` using `CompactRef` regardless of the asset id
pub struct CompactU128BalanceEncoder<T>(PhantomData<T>);

impl<AssetId> BalanceEncoder<AssetId, u128> for CompactU128BalanceEncoder<AssetId> {
    fn encoded_balance(_: &AssetId, balance: u128) -> Option<Vec<u8>> {
        let encoded =
            <<u128 as codec::HasCompact>::Type as codec::EncodeAsRef<'_, u128>>::RefType::from(
                &balance,
            )
            .encode();
        Some(encoded)
    }
}

/// Represents dispatchable calls of the FRAME `pallet_staking` pallet.
///
/// *NOTE*: `CompactBalance` is expected to encode with `HasCompact`
#[derive(Encode)]
pub enum StakingCall<AccountId: Encode, CompactBalance: Encode, Source: Encode> {
    /// The [`bond_extra`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond_extra) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the stash account.
    #[codec(index = 0)]
    Bond(
        // The controller to use
        // on polkadot this is of type `MultiAddress<AcountId, AccountIndex>`
        Source,
        CompactBalance,
        RewardDestination<AccountId>,
    ),

    /// The [`bond_extra`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond_extra) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
    #[codec(index = 1)]
    BondExtra(CompactBalance),
    /// The [`unbond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.unbond) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
    #[codec(index = 2)]
    Unbond(CompactBalance),
    /// The [`withdraw_unbonded`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.withdraw_unbonded) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
    #[codec(index = 3)]
    WithdrawUnbonded(u32),
    /// The [`nominate`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.nominate) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
    #[codec(index = 5)]
    Nominate(Vec<Source>),
}

impl<AccountId: Encode, CompactBalance: Encode, Source: Encode>
    StakingCall<AccountId, CompactBalance, Source>
{
    /// Wraps the staking pallet call into a `RuntimeCall`
    pub fn into_runtime_call(self, pallet_index: u8) -> RuntimeCall<Self> {
        RuntimeCall {
            pallet_index,
            call: self,
        }
    }
}

/// A destination account for payment.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum RewardDestination<AccountId> {
    /// Pay into the stash account, increasing the amount at stake accordingly.
    Staked,
    /// Pay into the stash account, not increasing the amount at stake.
    Stash,
    /// Pay into the controller account.
    Controller,
    /// Pay into a specified account.
    Account(AccountId),
    /// Receive no reward.
    None,
}

/// The `pallet_staking` configuration for a particular chain
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StakingConfig<AccountId, Balance> {
    /// The index of `pallet_index` within the parachain's runtime
    pub pallet_index: u32,
    /// The limitation to the number of fund-chunks that can be scheduled to be unlocked via `unbond`.
    ///
    /// If this is reached, the bonded account _must_ first wait until successful call to
    /// `withdraw_unbonded` to remove some of the chunks.
    pub max_unlocking_chunks: u32,
    /// Counter for the sent `unbond` calls.
    pub pending_unbond_calls: u32,
    /// The configured reward destination
    pub reward_destination: RewardDestination<AccountId>,
    /// The specified `minimum_balance` specified the parachain's `T::Currency`
    pub minimum_balance: Balance,
    /// The configured weights for `pallet_staking`
    pub weights: StakingWeights,
    // TODO add minumum (un)bond  that has to be met for executing XCM (un)bonding calls
}

/// The index of `pallet_staking` in the polkadot runtime
pub const POLKADOT_PALLET_STAKING_INDEX: u8 = 7u8;

/// Represents an excerpt from the `pallet_staking` weights
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StakingWeights {
    /// Weight for `bond` extrinsic
    pub bond: Weight,
    /// Weight for `bond_extra` extrinsic
    pub bond_extra: Weight,
    /// Weight for `unbond` extrinsic
    pub unbond: Weight,
}

impl StakingWeights {
    /// The weights as defined in `pallet_staking` on polkadot
    // TODO: import pallet_staking weights directly?
    pub fn polkadot() -> Self {
        let weight = RocksDbWeight::get();
        Self {
            bond: (75_102_000 as Weight)
                .saturating_add(weight.reads(5 as Weight))
                .saturating_add(weight.writes(4 as Weight)),
            bond_extra: (57_637_000 as Weight)
                .saturating_add(weight.reads(3 as Weight))
                .saturating_add(weight.writes(2 as Weight)),
            unbond: (52_115_000 as Weight)
                .saturating_add(weight.reads(4 as Weight))
                .saturating_add(weight.writes(3 as Weight)),
        }
    }
}

/// Outcome of an XCM staking execution.
#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug)]
pub enum StakingOutcome {
    /// Staking is not supported for the given asset
    ///
    /// No `StakingConfig` found
    NotSupported,
    /// Outcome of the executed staking xcm routine
    XcmOutcome(XcmOutcome),
}

impl From<XcmOutcome> for StakingOutcome {
    fn from(outcome: XcmOutcome) -> Self {
        StakingOutcome::XcmOutcome(outcome)
    }
}
