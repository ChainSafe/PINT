// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::{
    pallet_prelude::*,
};
use sp_runtime::traits::Hash;
use crate::Config;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// This represents an instance of a proposal that can be voted on.
/// It has been proposed and has an assigned nonce.
/// This extra abstraction is required since it may be desirable construct multiple
/// proposal instances out of a single proposal
pub struct Proposal<T: Config>(pub T::ProposalNonce, pub T::Action);

impl<T: Config> Proposal<T> {
    pub fn new(nonce: T::ProposalNonce, action: T::Action) -> Self {
        Self(nonce, action)
    }

    pub fn hash(&self) -> <T as frame_system::Config>::Hash {
        T::Hashing::hash_of(self)
    }
}

/// Origin for the committee module.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
pub enum CommitteeOrigin<AccountId> {
    /// Action is executed by the committee. Contains the closer account and the members that voted Aye
    ApprovedByCommittee(AccountId, Vec<AccountId>),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
/// Info for keeping track of a motion being voted on.
/// Default is empty vectors for all votes
pub struct VoteAggregate<AccountId, BlockNumber> {
    /// The current set of voters that approved it.
    pub ayes: Vec<AccountId>,
    /// The current set of voters that rejected it.
    pub nays: Vec<AccountId>,
    /// The current set of votes abstaining.
    pub abstentions: Vec<AccountId>,
    /// The hard end time of this vote.
    pub end: BlockNumber,
}

impl<AccountId: Default + PartialEq, BlockNumber: Default> VoteAggregate<AccountId, BlockNumber> {
    pub fn new(
        ayes: Vec<AccountId>,
        nays: Vec<AccountId>,
        abstentions: Vec<AccountId>,
        end: BlockNumber,
    ) -> Self {
        Self {
            ayes,
            nays,
            abstentions,
            end,
        }
    }

    pub fn new_with_end(end: BlockNumber) -> Self {
        Self {
            end,
            ..Default::default()
        }
    }

    // This does not check if a vote is a duplicate, This must be done before calling this function
    pub fn cast_vote(&mut self, voter: AccountId, vote: &Vote) {
        match vote {
            Vote::Aye => self.ayes.push(voter),
            Vote::Nay => self.nays.push(voter),
            Vote::Abstain => self.abstentions.push(voter),
        }
    }

    pub fn remove_voters(&mut self, voters: &[AccountId]) {
        self.ayes.retain(|x| !voters.contains(x));
        self.nays.retain(|x| !voters.contains(x));
        self.abstentions.retain(|x| !voters.contains(x));
    }

    pub fn has_voted(&self, voter: &AccountId) -> bool {
        self.ayes.contains(voter) | self.nays.contains(voter) | self.abstentions.contains(voter)
    }

    // to be accepted a proposal must have a majority of non-abstainig members vote Aye
    // TODO: Check how non-voting memnbers should be handled
    // TODO: Check how ties should be broken
    pub fn is_accepted(&self) -> bool {
        self.ayes.len() > self.nays.len()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Possible votes a member can cast
pub enum Vote {
    Aye,
    Nay,
    Abstain,
}
