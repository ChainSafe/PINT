// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate::Config;
use frame_support::{
    pallet_prelude::*,
    sp_std::{self, prelude::Vec},
};
use sp_core::u32_trait::Value as U32;
use sp_runtime::traits::Hash;

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

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, RuntimeDebug, Encode, Decode)]
/// Defines what sub-type a member belongs to.
/// Council members are fixed in number and can vote on proposals
/// Constituent members are unbounded in number but can only veto council proposals
pub enum MemberType {
    Council,
    Constituent,
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
/// Assignment of a member type to an accountId
pub struct CommitteeMember<AccountId> {
    pub account_id: AccountId,
    pub member_type: MemberType,
}

impl<AccountId> CommitteeMember<AccountId> {
    pub fn new(account_id: AccountId, member_type: MemberType) -> Self {
        Self {
            account_id,
            member_type,
        }
    }

    pub fn into_vote(self, vote: Vote) -> MemberVote<AccountId> {
        MemberVote { member: self, vote }
    }
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
/// A committee member together with their cast vote.
pub struct MemberVote<AccountId> {
    pub member: CommitteeMember<AccountId>,
    pub vote: Vote,
}

impl<AccountId> MemberVote<AccountId> {
    pub fn new(member: CommitteeMember<AccountId>, vote: Vote) -> Self {
        Self { member, vote }
    }
}

/// Origin for the committee pallet.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
pub enum CommitteeOrigin<AccountId, BlockNumber> {
    /// Action is executed by the committee. Contains the closer account and the members that voted Aye
    ApprovedByCommittee(AccountId, VoteAggregate<AccountId, BlockNumber>),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
/// Info for keeping track of a motion being voted on.
/// Default is empty vectors for all votes
pub struct VoteAggregate<AccountId, BlockNumber> {
    /// The current set of votes.
    pub votes: Vec<MemberVote<AccountId>>,
    /// The hard end time of this vote.
    pub end: BlockNumber,
}

impl<I, B> VoteAggregate<I, B> {
    /// Get the votes of I
    pub fn count_votes(&self) -> (u32, u32, u32) {
        let (mut aye, mut nay, mut abstain) = (0, 0, 0);
        self.votes.iter().for_each(|v| match v.vote {
            Vote::Aye => aye += 1,
            Vote::Nay => nay += 1,
            Vote::Abstain => abstain += 1,
        });

        (aye, nay, abstain)
    }
}

pub enum VoteRejectionReason {
    InsuffientVotes,
    ConstituentVeto,
    CouncilDeny,
}

impl<AccountId: Default + PartialEq, BlockNumber: Default> VoteAggregate<AccountId, BlockNumber> {
    pub fn new(
        ayes: Vec<CommitteeMember<AccountId>>,
        nays: Vec<CommitteeMember<AccountId>>,
        abstentions: Vec<CommitteeMember<AccountId>>,
        end: BlockNumber,
    ) -> Self {
        let votes = sp_std::iter::empty()
            .chain(ayes.into_iter().map(|x| x.into_vote(Vote::Aye)))
            .chain(nays.into_iter().map(|x| x.into_vote(Vote::Nay)))
            .chain(abstentions.into_iter().map(|x| x.into_vote(Vote::Abstain)))
            .collect();
        Self { votes, end }
    }

    pub fn new_with_end(end: BlockNumber) -> Self {
        Self {
            end,
            ..Default::default()
        }
    }

    // This does not check if a vote is a duplicate, This must be done before calling this function
    pub fn cast_vote(&mut self, vote: MemberVote<AccountId>) {
        self.votes.push(vote)
    }

    pub fn remove_voters(&mut self, voters: &[AccountId]) {
        self.votes
            .retain(|x| !voters.contains(&x.member.account_id));
    }

    pub fn has_voted(&self, voter: &AccountId) -> bool {
        self.votes.iter().any(|x| &x.member.account_id == voter)
    }

    /// produce a tuple of the vote totals: (ayes, nays, abstentions)
    /// Can optionally filter by membership type to only tally council or constituent votes
    pub fn tally(&self, member_type: Option<&MemberType>) -> (usize, usize, usize) {
        self.votes
            .iter()
            .filter(|x| {
                if let Some(m) = member_type {
                    &x.member.member_type == m
                } else {
                    true
                }
            })
            .fold((0, 0, 0), |(ayes, nays, abs), x| match x.vote {
                Vote::Aye => (ayes + 1, nays, abs),
                Vote::Nay => (ayes, nays + 1, abs),
                Vote::Abstain => (ayes, nays, abs + 1),
            })
    }

    /// For a vote to be accepted all of the following must be true:
    ///  - At least min_council_votes must be cast by the council
    ///  - A simple majority of council Ayes vs Nays (e.g. count(ayes) > count(nays))
    ///  - There is NOT a majority of Nay votes by the constituent members
    pub fn is_accepted(&self, min_council_votes: usize) -> Result<(), VoteRejectionReason> {
        // council votes
        let (ayes, nays, abs) = self.tally(Some(&MemberType::Council));
        let participants = ayes + nays + abs;
        // constituent votes
        let (cons_ayes, cons_nays, _) = self.tally(Some(&MemberType::Constituent));

        ensure!(
            participants >= min_council_votes,
            VoteRejectionReason::InsuffientVotes
        );
        ensure!(ayes > nays, VoteRejectionReason::CouncilDeny);
        ensure!(cons_nays <= cons_ayes, VoteRejectionReason::ConstituentVeto);

        Ok(())
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Possible votes a member can cast
pub enum Vote {
    Aye,
    Nay,
    Abstain,
}

pub struct EnsureProportionMoreThan<N: U32, AccountId, BlockNumber>(
    sp_std::marker::PhantomData<(N, AccountId, BlockNumber)>,
);
impl<
        O: Into<Result<CommitteeOrigin<AccountId, BlockNumber>, O>>
            + From<CommitteeOrigin<AccountId, BlockNumber>>,
        N: U32,
        AccountId,
        BlockNumber,
    > EnsureOrigin<O> for EnsureProportionMoreThan<N, AccountId, BlockNumber>
{
    type Success = ();
    fn try_origin(o: O) -> Result<Self::Success, O> {
        o.into().and_then(|o| match o {
            CommitteeOrigin::ApprovedByCommittee(_, _) => Ok(()),
        })
    }
}
