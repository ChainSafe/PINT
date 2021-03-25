// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod utils;

#[frame_support::pallet]
// requires unused_unit exception as the #[pallet::event] proc macro generates code that violates this lint
// requires boxed_local exception as extrincis must accept boxed calls but clippy only sees the local function
#[allow(clippy::unused_unit, clippy::boxed_local)]
pub mod pallet {
    use crate::utils;
    use frame_support::{
        dispatch::{Codec, DispatchResultWithPostInfo},
        pallet_prelude::*,
        sp_runtime::traits::Dispatchable,
        traits::{ChangeMembers, InitializeMembers},
        weights::{GetDispatchInfo, PostDispatchInfo},
    };
    use frame_system::pallet_prelude::*;
    use frame_system::RawOrigin;
    use sp_runtime::traits::{CheckedAdd, Hash, One, Zero};

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    type HashFor<T> = <T as frame_system::Config>::Hash;
    type BlockNumberFor<T> = <T as frame_system::Config>::BlockNumber;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The outer origin type.
        type Origin: From<RawOrigin<Self::AccountId>>;
        /// The outer call dispatch type.
        type Action: Parameter
            + Dispatchable<Origin = <Self as Config>::Origin, PostInfo = PostDispatchInfo>
            + From<frame_system::Call<Self>>
            + GetDispatchInfo;

        /// A unique number assigned to each new instance of a proposal
        type ProposalNonce: Parameter
            + Member
            + One
            + Zero
            + Codec
            + Default
            + MaybeSerializeDeserialize
            + CheckedAdd;

        /// Duration (in blocks) of te proposal submission period
        type ProposalSubmissionPeriod: Get<Self::BlockNumber>;

        /// Duration (in blocks) of the voting period
        type VotingPeriod: Get<Self::BlockNumber>;

        /// Origin that is permitted to create proposals
        type ProposalSubmissionOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

        /// Origin that is permitted to execute approved proposals
        type ProposalExecutionOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    /// This represents an instance of a proposal that can be voted on.
    /// It has been proposed and has an assigned nonce.
    /// This extra abstraction is required since it may be desirable construct multiple
    /// proposal instances out of a single proposal
    pub struct Proposal<T: Config>(T::ProposalNonce, T::Action);

    impl<T: Config> Proposal<T> {
        pub fn new(nonce: T::ProposalNonce, action: T::Action) -> Self {
            Self(nonce, action)
        }

        pub fn hash(&self) -> <T as frame_system::Config>::Hash {
            T::Hashing::hash_of(self)
        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    /// Info for keeping track of a motion being voted on.
    /// This implements Default which is
    pub struct VoteAggregate<AccountId, BlockNumber> {
        /// The current set of voters that approved it.
        ayes: Vec<AccountId>,
        /// The current set of voters that rejected it.
        nays: Vec<AccountId>,
        /// The current set of votes abstaining.
        abstenations: Vec<AccountId>,
        /// The hard end time of this vote.
        end: BlockNumber,
    }

    impl<AccountId: Default + PartialEq, BlockNumber: Default> VoteAggregate<AccountId, BlockNumber> {
        pub fn new_with_end(end: BlockNumber) -> Self {
            Self {
                end,
                ..Default::default()
            }
        }

        // This does not check if a vote is a duplicate, This must be done before calling this function
        pub fn cast_vote(&mut self, voter: AccountId, vote: Vote) {
            match vote {
                Vote::Aye => self.ayes.push(voter),
                Vote::Nay => self.nays.push(voter),
                Vote::Abstain => self.abstenations.push(voter),
            }
        }

        pub fn remove_voters(&mut self, voters: &[AccountId]) {
            self.ayes.retain(|x| !voters.contains(x));
            self.nays.retain(|x| !voters.contains(x));
            self.abstenations.retain(|x| !voters.contains(x));
        }

        pub fn has_voted(&self, voter: &AccountId) -> bool {
            self.ayes.contains(voter)
                | self.nays.contains(voter)
                | self.abstenations.contains(voter)
        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    /// Possible votes a member can cast
    pub enum Vote {
        Aye,
        Nay,
        Abstain,
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // Storage defs

    #[pallet::storage]
    /// Stores a vector of the hashes of currently active proposals for iteration
    pub type ActiveProposals<T: Config> = StorageValue<_, Vec<HashFor<T>>, ValueQuery>;

    #[pallet::storage]
    /// Increments with each new proposal. Used to produce a unique nonce per proposal instance
    pub type ProposalCount<T: Config> = StorageValue<_, T::ProposalNonce, ValueQuery>;

    #[pallet::storage]
    /// Store a mapping (hash) -> Proposal for all existing proposals.
    pub type Proposals<T: Config> =
        StorageMap<_, Blake2_128Concat, HashFor<T>, Proposal<T>, OptionQuery>;

    #[pallet::storage]
    /// Stores a vector of the hashes of currently active proposals for iteration
    pub type Members<T: Config> = StorageValue<_, Vec<AccountIdFor<T>>, ValueQuery>;

    #[pallet::storage]
    /// Store a mapping (hash) -> VoteAggregate for all existing proposals.
    pub type Votes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        HashFor<T>,
        VoteAggregate<AccountIdFor<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    // end storage defs

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Proposed(AccountIdFor<T>, T::ProposalNonce, T::Hash),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The origin making the call is not a member and it is a requirement that they are
        NotMember,
        /// Member has attempted to vote multiple times on a single proposal
        DuplicateVote,
        /// The hash provided does not have an associated proposal
        NoProposalWithHash,
        /// The data type for enumerating the proposals has reached its upper bound.
        /// No more proposals can be made
        ProposalNonceExhausted,
        /// There was a numerical overflow or underflow in calculating when the voting period
        /// should end
        InvalidOperationInEndBlockComputation,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    impl<T: Config> Pallet<T> {
        fn take_and_increment_nonce() -> Result<T::ProposalNonce, Error<T>> {
            let nonce = <ProposalCount<T>>::get();
            match nonce.checked_add(&T::ProposalNonce::one()) {
                Some(next) => {
                    <ProposalCount<T>>::set(next);
                    Ok(nonce)
                }
                None => Err(Error::ProposalNonceExhausted),
            }
        }

        pub fn active_proposals() -> Vec<HashFor<T>> {
            <ActiveProposals<T>>::get()
        }

        pub fn get_proposal(hash: &HashFor<T>) -> Option<Proposal<T>> {
            <Proposals<T>>::get(hash)
        }

        /// Returns None if no proposal exists
        pub fn get_votes_for(
            hash: &HashFor<T>,
        ) -> Option<VoteAggregate<AccountIdFor<T>, BlockNumberFor<T>>> {
            <Votes<T>>::get(hash)
        }

        pub fn members() -> Vec<AccountIdFor<T>> {
            <Members<T>>::get()
        }

        pub fn ensure_member(origin: OriginFor<T>) -> Result<AccountIdFor<T>, DispatchError> {
            let who = ensure_signed(origin)?;
            let members = Self::members();
            ensure!(members.contains(&who), Error::<T>::NotMember);
            Ok(who)
        }

        pub fn get_next_voting_period_end() -> Result<BlockNumberFor<T>, DispatchError> {
            utils::get_vote_end(
                &frame_system::Pallet::<T>::block_number(),
                &T::VotingPeriod::get(),
                &T::ProposalSubmissionPeriod::get(),
            ).ok_or(Error::<T>::InvalidOperationInEndBlockComputation.into())
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn propose(origin: OriginFor<T>, action: Box<T::Action>) -> DispatchResultWithPostInfo {
            let proposer = ensure_signed(origin.clone())?;
            T::ProposalSubmissionOrigin::ensure_origin(origin)?;

            // Create a new proposal with a unique nonce
            let nonce = Self::take_and_increment_nonce()?;
            let proposal = Proposal::<T>(nonce.clone(), *action);

            let proposal_hash = proposal.hash();

            // Store the proposal by its hash.
            <Proposals<T>>::insert(proposal_hash, proposal);

            // Add the proposal to the active proposals and set the initial votes
            // Set the end block number to the end of the next voting period
            <ActiveProposals<T>>::append(&proposal_hash);
            let end = Self::get_next_voting_period_end()?;

            <Votes<T>>::insert(proposal_hash, VoteAggregate::new_with_end(end));

            Self::deposit_event(Event::Proposed(proposer, nonce, proposal_hash));

            Ok(().into())
        }

        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn vote(
            origin: OriginFor<T>,
            proposal_hash: HashFor<T>,
            vote: Vote,
        ) -> DispatchResultWithPostInfo {
            // Only members can vote
            let voter = Self::ensure_member(origin)?;

            <Votes<T>>::try_mutate(&proposal_hash, |votes| {
                if let Some(votes) = votes {
                    // members can vote only once
                    ensure!(!votes.has_voted(&voter), Error::<T>::DuplicateVote);
                    votes.cast_vote(voter, vote); // mutates votes in place
                    Ok(())
                } else {
                    Err(Error::<T>::NoProposalWithHash)
                }
            })?;

            Ok(().into())
        }
    }

    impl<T: Config> InitializeMembers<AccountIdFor<T>> for Pallet<T> {
        fn initialize_members(members: &[AccountIdFor<T>]) {
            if !members.is_empty() {
                assert!(
                    <Members<T>>::get().is_empty(),
                    "Members are already initialized!"
                );
                <Members<T>>::put(members);
            }
        }
    }

    impl<T: Config> ChangeMembers<AccountIdFor<T>> for Pallet<T> {
        fn change_members_sorted(
            _incoming: &[AccountIdFor<T>],
            outgoing: &[AccountIdFor<T>],
            new: &[AccountIdFor<T>],
        ) {
            // Remove outgoing members from any currently active votes
            for proposal_hash in ActiveProposals::<T>::get() {
                <Votes<T>>::mutate(&proposal_hash, |votes| {
                    if let Some(votes) = votes {
                        votes.remove_voters(outgoing); // mutates votes in place
                    }
                });
            }
            Members::<T>::put(new);
        }
    }
}
