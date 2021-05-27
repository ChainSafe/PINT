// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Additional types for the remote asset manager pallet
use codec::{Decode, Encode, EncodeLike};
use frame_support::{
    dispatch::{CallableCallFor, Output},
    sp_runtime::RuntimeDebug,
    sp_std::prelude::*,
    traits::Currency,
};

pub struct TransactCall<Call: Encode> {
    /// The index of the call's pallet within the runtime.
    ///
    /// This must be equivalent with the `#[codec(index = <pallet_index>)]` annotation.
    pub pallet_index: u8,

    /// The actual that should be dispatched
    pub call: Call,
}

impl<Call: EncodeLike> EncodeLike for TransactCall<Call> {}

impl<Call: EncodeLike> Encode for TransactCall<Call> {
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        dest.push_byte(self.pallet_index);
        self.call.encode_to(dest)
    }
}

// TODO: MAX_UNLOCKING_CHUNKS in pallet_staking

/// Represents dispatchable calls of the FRAME `pallet_staking` pallet.
#[derive(Encode)]
pub enum StakingCall<AccountId: Encode, Balance: Encode, Source: Encode> {
    /// The [`bond_extra`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond_extra) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the stash account.
    #[codec(index = 0)]
    Bond(
        Source,
        #[codec(compact)] Balance,
        RewardDestination<AccountId>,
    ),

    /// The [`bond_extra`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond_extra) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
    #[codec(index = 1)]
    BondExtra(#[codec(compact)] Balance),
    /// The [`unbond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.unbond) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
    #[codec(index = 2)]
    Unbond(#[codec(compact)] Balance),
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

/// A destination account for payment.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
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

pub enum StakingSupport {
    None,
}
