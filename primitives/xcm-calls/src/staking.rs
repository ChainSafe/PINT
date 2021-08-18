// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Xcm support for `pallet_staking` calls.
//!
//! This module provides support for calling into the FRAME `pallet_staking` pallet of a remote
//! chain via XCM.
//!
//! Staking involves bonding funds for a certain amount of blocks.
//! The `pallet_staking` pallet is configured with [`Config::BondingDuration`] (in number of eras)
//! must pass until the funds can actually be removed (`withdraw_unbonded`), after they were
//! `unbonded`.
//! - An **Era** is defined as  (whole) number of sessions, which is the period that the validator
//!   set (and each validator's active nominator set) is recalculated and where rewards are paid
//!   out. An era is ~7 days (relay chain) and the `BondingDuration` on polkadot is 28 Eras and 7
//!   Eras on Kusama.
//! - A **Session** is a period of time that has a constant set of validators and is measured in
//!   block numbers. Sessions are handled by the FRAME `pallet_session` pallet which implement the
//!   `ShouldEndSession` trait which determines when a session has ended, and new started. This is
//!   used to determine the overall session length.
//!
//! Kusama and polkadot rely on **BABE** (`pallet_babe`) to determine when a session has ended. the
//! BABE pallet implements the `ShouldEndSession` trait and determines whether a session should end
//! by determine whether the epoch should change. An epoch should change if more than
//! `EpochDuration` time has passed. `EpochDuration` measures the amount of time in slots, that each
//! epoch should last. **An epoch length cannot be changed after the chain has started.** Meaning
//! this is chain specific constant. An Epoch on kusama is 1 hour, and 4 hours
//!
//! Knowledge of the `EpochDuration` and the `BondingDuration` and the `MILLISECS_PER_BLOCK` or
//! rather the `SLOT_DURATION` is required to determine when we call `withdraw_unbonded` after we
//! initiate the `unbond`.

use codec::{Compact, Decode, Encode, Output};
use frame_support::{sp_std::vec::Vec, weights::Weight, RuntimeDebug};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use crate::{CallEncoder, EncodeWith, PalletCall, PalletCallEncoder};
use frame_support::sp_runtime::traits::AtLeast32BitUnsigned;

/// The index of `pallet_staking` in the polkadot runtime
pub const POLKADOT_PALLET_STAKING_INDEX: u8 = 7u8;

/// Provides encoder types to encode the associated types of the
/// `pallet_staking::Config` trait depending on the configured Context.
pub trait StakingCallEncoder<Source, Balance, AccountId>: PalletCallEncoder {
	/// Encodes the `<pallet_staking::Config>::Balance` depending on the context
	type CompactBalanceEncoder: EncodeWith<Balance, Self::Context>;

	/// Encodes the `<pallet_staking::Config>::Source` depending on the context
	type SourceEncoder: EncodeWith<Source, Self::Context>;

	/// Encodes the `<pallet_staking::Config>::AccountId` depending on the
	/// context
	type AccountIdEncoder: EncodeWith<AccountId, Self::Context>;
}

impl<'a, 'b, Source, Balance, AccountId, Config> Encode
	for CallEncoder<'a, 'b, StakingCall<Source, Balance, AccountId>, Config>
where
	Config: StakingCallEncoder<Source, Balance, AccountId>,
{
	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		// include the pallet identifier
		dest.push_byte(self.call.pallet_call_index());
		match self.call {
			StakingCall::Bond(bond) => {
				Config::SourceEncoder::encode_to_with(&bond.controller, self.ctx, dest);
				Config::CompactBalanceEncoder::encode_to_with(&bond.value, self.ctx, dest);

				match &bond.payee {
					RewardDestination::Staked => {
						dest.push_byte(0);
					}
					RewardDestination::Stash => {
						dest.push_byte(1);
					}
					RewardDestination::Controller => {
						dest.push_byte(2);
					}
					RewardDestination::Account(ref account) => {
						dest.push_byte(3);
						Config::AccountIdEncoder::encode_to_with(account, self.ctx, dest);
					}
					_ => {}
				}
			}
			StakingCall::BondExtra(val) => {
				Config::CompactBalanceEncoder::encode_to_with(val, self.ctx, dest);
			}
			StakingCall::Unbond(val) => {
				Config::CompactBalanceEncoder::encode_to_with(val, self.ctx, dest);
			}
			StakingCall::WithdrawUnbonded(val) => val.encode_to(dest),
			StakingCall::Nominate(sources) => {
				Compact(sources.len() as u32).encode_to(dest);
				for source in sources {
					Config::SourceEncoder::encode_to_with(source, self.ctx, dest);
				}
			}
		}
	}
}

/// Represents dispatchable calls of the FRAME `pallet_staking` pallet.
///
/// *NOTE*: `Balance` is expected to encode with `HasCompact`
pub enum StakingCall<Source, Balance, AccountId> {
	/// The [`bond`](https://crates.parity.io/pallet_staking/pallet/enum.Call.html#variant.bond) extrinsic.
	///
	/// The dispatch origin for this call must be _Signed_ by the stash account.
	// #[codec(index = 0)]
	Bond(Bond<Source, Balance, AccountId>),

	/// The [`bond_extra`](https://crates.parity.io/pallet_staking/pallet/enum.Call.html#variant.bond_extra) extrinsic.
	///
	/// The dispatch origin for this call must be _Signed_ by the stash, not the
	/// controller.
	// #[codec(index = 1)]
	BondExtra(Balance),
	/// The [`unbond`](https://crates.parity.io/pallet_staking/pallet/enum.Call.html#variant.unbond) extrinsic.
	///
	/// The dispatch origin for this call must be _Signed_ by the controller,
	/// not the stash.
	// #[codec(index = 2)]
	Unbond(Balance),
	/// The [`withdraw_unbonded`](https://crates.parity.io/pallet_staking/pallet/enum.Call.html#variant.withdraw_unbonded) extrinsic.
	///
	/// The dispatch origin for this call must be _Signed_ by the controller,
	/// not the stash.
	/// `num_slashing_spans` the number of slashing spans to remove.
	// #[codec(index = 3)]
	WithdrawUnbonded(u32),
	/// The [`nominate`](https://crates.parity.io/pallet_staking/pallet/enum.Call.html#variant.nominate) extrinsic.
	///
	/// The dispatch origin for this call must be _Signed_ by the controller,
	/// not the stash.
	// #[codec(index = 5)]
	Nominate(Vec<Source>),
}

impl<Source, Balance, AccountId> PalletCall for StakingCall<Source, Balance, AccountId> {
	/// the indices of the corresponding calls within the `pallet_staking`
	fn pallet_call_index(&self) -> u8 {
		match self {
			StakingCall::Bond(_) => 0,
			StakingCall::BondExtra(_) => 1,
			StakingCall::Unbond(_) => 2,
			StakingCall::WithdrawUnbonded(_) => 3,
			StakingCall::Nominate(_) => 5,
		}
	}
}

/// The [`bond_extra`](https://crates.parity.io/pallet_staking/pallet/enum.Call.html#variant.bond_extra) extrinsic.
///
/// The dispatch origin for this call must be _Signed_ by the stash account.
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub struct Bond<Source, Balance, AccountId> {
	/// The lookup type of the controller,
	pub controller: Source,
	/// The amount to bond.
	pub value: Balance,
	/// How to payout staking rewards
	pub payee: RewardDestination<AccountId>,
}

/// A destination account for payment. mirrored from `pallet_staking`
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
pub struct StakingConfig<AccountId, Balance, BlockNumber> {
	/// The index of `pallet_index` within the parachain's runtime
	pub pallet_index: u8,
	/// The configured reward destination
	pub reward_destination: RewardDestination<AccountId>,
	/// The specified `minimum_balance` specified the parachain's `T::Currency`
	pub minimum_balance: Balance,
	/// The configured weights for `pallet_staking`
	pub weights: StakingWeights,
	/// This measures the time that must pass until `unbonded` funds can be withdrawn measured in
	/// **BlockNumber**s
	///
	/// *NOTE:* It is expected that this is already the duration measured in PINT parachain blocks
	/// and *NOT* in remote chain blocks. This involves converting the block time of the other chain
	/// to the block time of PINT. The `bonding_duration` as it's expected here is
	/// ```nocompile
	///    BondDuration * EPOCH_DURATION_IN_BLOCKS * SessionsPerEra * (MILLISEC_PER_BLOCK_other / MILLISEC_PER_BLOCK_pint)
	/// ```
	pub bonding_duration: BlockNumber,
}

// Counter for the number of eras that have passed
pub type EraIndex = u32;

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct UnlockChunk<Balance, BlockNumber> {
	/// Amount of funds to be unlocked.
	pub value: Balance,
	/// The block number at which point it'll be unlocked.
	///
	/// *NOTE:* this is expected to be PINT block number
	pub end: BlockNumber,
}

/// Represents the state of staking of the PINT's sovereign account on another chain
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct StakingLedger<Source, Balance, BlockNumber> {
	/// The controller account
	pub controller: Source,
	/// The total amount of the stash's balance that will be at stake
	pub active: Balance,
	/// The total amount of the stash's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	pub total: Balance,
	/// Any balance that is becoming free, which may eventually be transferred out
	/// of the stash (assuming it doesn't get slashed first).
	///
	/// *NOTE:* No more than a limited number of unlocking chunks can co-exists at the same time.
	///  See `pallet_staking::MAX_UNLOCKING_CHUNKS`.
	pub unlocking: Vec<UnlockChunk<Balance, BlockNumber>>,
}

impl<Source, Balance, BlockNumber> StakingLedger<Source, Balance, BlockNumber>
where
	Balance: AtLeast32BitUnsigned + Copy,
	BlockNumber: AtLeast32BitUnsigned,
{
	/// Mirror an `bond` or `bond_extra` that increased the bonded amount
	pub fn bond_extra(&mut self, amount: Balance) {
		self.active = self.active.saturating_add(amount);
	}

	/// The amount currently unbonding
	pub fn unlocking(&self) -> Balance {
		self.total.saturating_sub(self.active)
	}

	/// Remove entries from `unlocking` that are sufficiently old and reduce the
	/// total by the sum of their balances.
	/// Returns `true` if at least one chunk was consolidated
	pub fn consolidate_unlocked(&mut self, current_block: BlockNumber) -> bool {
		let chunks = self.unlocking.len();
		let mut total = self.total;
		self.unlocking.retain(|chunk| {
			if chunk.end > current_block {
				true
			} else {
				total = total.saturating_sub(chunk.value);
				false
			}
		});
		self.total = total;

		chunks > self.unlocking.len()
	}
}

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
	/// Weight for `withdraw_unbonded` extrinsic
	pub withdraw_unbonded: Weight,
}

/// Represents all staking related durations required to determine the correct chain-specific
/// bonding duration.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StakingDurations {
	/// This determines the average expected block time that the chain is targeting.
	///
	/// This is essentially the block time in milliseconds.
	pub slot_duration: u64,
	/// The amount of time in slots, that each epoch should last
	///
	/// This is essentially the duration of an Era
	pub epoch_duration: u64,
	/// The number of eras the bonding duration is long
	pub bonding_duration: u64,
}
