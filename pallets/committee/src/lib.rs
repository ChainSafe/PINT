// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types;
mod utils;

#[frame_support::pallet]
// requires unused_unit exception as the #[pallet::event] proc macro generates code that violates this lint
// requires boxed_local exception as extrinsics must accept boxed calls but clippy only sees the local function
#[allow(clippy::unused_unit, clippy::boxed_local)]
pub mod pallet {
    pub use crate::types::*;
    use crate::utils;
    use frame_support::{
        dispatch::{Codec, DispatchResultWithPostInfo},
        pallet_prelude::*,
        sp_runtime::traits::Dispatchable,
        traits::{ChangeMembers, InitializeMembers},
        weights::{GetDispatchInfo, PostDispatchInfo},
    };
    use frame_system::pallet_prelude::*;
    // use frame_system::RawOrigin;
    use sp_runtime::traits::{CheckedAdd, One, Zero};

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    type HashFor<T> = <T as frame_system::Config>::Hash;
    type BlockNumberFor<T> = <T as frame_system::Config>::BlockNumber;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The outer origin type.
        type Origin: From<CommitteeOrigin<Self::AccountId>>;
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

    pub type Origin<T> = CommitteeOrigin<<T as frame_system::Config>::AccountId>;

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
    /// Store a mapping (hash) -> () for all proposals that have been executed
    pub type ExecutedProposals<T: Config> =
        StorageMap<_, Blake2_128Concat, HashFor<T>, (), OptionQuery>;

    #[pallet::storage]
    /// Stores a vector of account IDs of current committee members
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
        /// A new proposal has been created
        /// \[proposer_address, proposal_nonce, proposal_hash\]
        Proposed(AccountIdFor<T>, T::ProposalNonce, T::Hash),
        /// A vote was cast
        /// \[voter_address, proposal_hash, vote\]
        VoteCast(AccountIdFor<T>, T::Hash, Vote),
        /// A proposal was closed and executed. Any errors for calling the proposal action
        /// are included
        /// [proposal_hash, result]
        ClosedAndExecutedProposal(T::Hash, DispatchResult),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The origin making the call is not a member and it is a requirement that they are
        NotMember,
        /// Member has attempted to vote multiple times on a single proposal
        DuplicateVote,
        /// Attempted to cast a vote outside the accepted voting period for a proposal
        NotInVotingPeriod,
        /// Attempted to close a proposal before the voting period is over
        VotingPeriodNotElapsed,
        /// Tried to close a proposal that does not meet the vote requirements
        ProposalNotAccepted,
        /// Attempted to execute a proposal that has already been executed
        ProposalAlreadyExecuted,
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
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            // perform upkeep only at the start of a new cycle
            if Self::get_next_voting_period_end(&n)
                == Ok(n + T::VotingPeriod::get() + T::ProposalSubmissionPeriod::get())
            {
                Self::upkeep(n);
            }
            0 // TODO: Calcualte the non-negotiable weight consumed by performing upkeep
        }
    }

    impl<T: Config> Pallet<T> {
        /// Gets a new unused proposal nonce and increments the nonce in the store
        /// Returns an error if the data type used for the nonce exceeds is maximum value
        fn take_and_increment_nonce() -> Result<T::ProposalNonce, Error<T>> {
            let nonce = <ProposalCount<T>>::get();
            match nonce.checked_add(&T::ProposalNonce::one()) {
                Some(next) => {
                    ProposalCount::<T>::set(next);
                    Ok(nonce)
                }
                None => Err(Error::ProposalNonceExhausted),
            }
        }

        pub fn active_proposals() -> Vec<HashFor<T>> {
            ActiveProposals::<T>::get()
        }

        pub fn get_proposal(hash: &HashFor<T>) -> Option<Proposal<T>> {
            Proposals::<T>::get(hash)
        }

        /// Get the votes for a proposal. Returns None if no proposal exists
        pub fn get_votes_for(
            hash: &HashFor<T>,
        ) -> Option<VoteAggregate<AccountIdFor<T>, BlockNumberFor<T>>> {
            Votes::<T>::get(hash)
        }

        pub fn members() -> Vec<AccountIdFor<T>> {
            Members::<T>::get()
        }

        /// Used to check if an origin is signed and the signer is a member of
        /// the committee
        pub fn ensure_member(origin: OriginFor<T>) -> Result<AccountIdFor<T>, DispatchError> {
            let who = ensure_signed(origin)?;
            let members = Self::members();
            ensure!(members.contains(&who), Error::<T>::NotMember);
            Ok(who)
        }

        /// Returns the block at the end of the next voting period
        pub fn get_next_voting_period_end(
            block_number: &BlockNumberFor<T>,
        ) -> Result<BlockNumberFor<T>, DispatchError> {
            utils::get_vote_end(
                block_number,
                &T::VotingPeriod::get(),
                &T::ProposalSubmissionPeriod::get(),
            )
            .ok_or_else(|| Error::<T>::InvalidOperationInEndBlockComputation.into())
        }

        /// Return true if the current block indicates it is the voting period 
        /// for the given VoteAggregate. 
        pub fn within_voting_period(
            votes: &VoteAggregate<AccountIdFor<T>, BlockNumberFor<T>>,
        ) -> bool {
            let current_block = frame_system::Pallet::<T>::block_number();
            current_block < votes.end && current_block >= votes.end - T::VotingPeriod::get()
        }

        /// Function executed at the initialization of the first block in 
        /// a new voting period cycle. Used to maintain the active proposals store.
        fn upkeep(n: BlockNumberFor<T>) {
            // clear out proposals that are no longer active
            ActiveProposals::<T>::mutate(|proposals| {
                proposals.retain(|hash| {
                    if let Some(votes) = Self::get_votes_for(hash) {
                        votes.end > n
                    } else {
                        false
                    }
                })
            })
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)] // TODO: Set weights
        /// Extrinsic to propose a new action to be voted upon in the next voting period.
        /// The provided action will be turned into a proposal and added to the list of current active proposals
        /// to be voted on in the next voting period.
        pub fn propose(origin: OriginFor<T>, action: Box<T::Action>) -> DispatchResultWithPostInfo {
            let proposer = ensure_signed(origin.clone())?;
            T::ProposalSubmissionOrigin::ensure_origin(origin)?;

            // Create a new proposal with a unique nonce
            let nonce = Self::take_and_increment_nonce()?;
            let proposal = Proposal::<T>(nonce.clone(), *action);

            let proposal_hash = proposal.hash();

            // Store the proposal by its hash.
            Proposals::<T>::insert(proposal_hash, proposal);

            // Add the proposal to the active proposals and set the initial votes
            // Set the end block number to the end of the next voting period
            ActiveProposals::<T>::append(&proposal_hash);
            let end = Self::get_next_voting_period_end(&frame_system::Pallet::<T>::block_number())?;

            Votes::<T>::insert(proposal_hash, VoteAggregate::new_with_end(end));

            Self::deposit_event(Event::Proposed(proposer, nonce, proposal_hash));

            Ok(().into())
        }

        #[pallet::weight(10_000)] // TODO: Set weights
        /// Extrinsic to vote on an existing proposal.
        /// This can only be called by members of the committee.
        /// Successfully cast votes will be recorded in the state and a proposal
        /// meeting voting requirements can be executed.
        pub fn vote(
            origin: OriginFor<T>,
            proposal_hash: HashFor<T>,
            vote: Vote,
        ) -> DispatchResultWithPostInfo {
            // Only members can vote
            let voter = Self::ensure_member(origin)?;

            <Votes<T>>::try_mutate(&proposal_hash, |votes| {
                if let Some(votes) = votes {
                    // Can only vote within the allowed range of blocks for this proposal
                    ensure!(
                        Self::within_voting_period(&votes),
                        Error::<T>::NotInVotingPeriod
                    );
                    // members can vote only once
                    ensure!(!votes.has_voted(&voter), Error::<T>::DuplicateVote);
                    votes.cast_vote(voter.clone(), &vote); // mutates votes in place
                    Self::deposit_event(Event::VoteCast(voter, proposal_hash, vote));
                    Ok(())
                } else {
                    Err(Error::<T>::NoProposalWithHash)
                }
            })?;

            Ok(().into())
        }

        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn close(
            origin: OriginFor<T>,
            proposal_hash: HashFor<T>,
        ) -> DispatchResultWithPostInfo {
            let closer = ensure_signed(origin.clone())?;
            T::ProposalExecutionOrigin::ensure_origin(origin)?;

            // ensure proposal has not already been executed
            ensure!(
                !ExecutedProposals::<T>::contains_key(proposal_hash),
                Error::<T>::ProposalAlreadyExecuted
            );

            let votes =
                Self::get_votes_for(&proposal_hash).ok_or(Error::<T>::NoProposalWithHash)?;
            let current_block = frame_system::Pallet::<T>::block_number();

            // Ensure voting period is over
            ensure!(
                current_block > votes.end,
                Error::<T>::VotingPeriodNotElapsed
            );

            // Ensure voting has accepted proposal
            ensure!(votes.is_accepted(), Error::<T>::ProposalNotAccepted);

            // Execute the proposal
            let proposal =
                Self::get_proposal(&proposal_hash).ok_or(Error::<T>::NoProposalWithHash)?;
            let result = proposal
                .1
                .dispatch(Origin::<T>::ApprovedByCommittee(closer, votes.ayes).into());

            // register that this proposal has been executed
            ExecutedProposals::<T>::insert(proposal_hash, ());

            Self::deposit_event(Event::ClosedAndExecutedProposal(
                proposal_hash,
                result.map(|_| ()).map_err(|e| e.error),
            ));

            // TODO: Handle weight used by the dispatch call in weight calculation

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
                Members::<T>::put(members);
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
