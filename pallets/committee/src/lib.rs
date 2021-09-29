// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Committee Pallet
//!
//! The Committee pallet uses a set of AccountIds to identify who
//! can vote on proposals. This set can be modified via proposals to
//! the Governance Committee. Members may be added, removed or swapped
//! with new members. There is no bound on how many members may exist
//! in the committee.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod traits;
mod types;
mod utils;

// requires unused_unit exception as the #[pallet::event] proc macro generates code that violates
// this lint requires boxed_local exception as extrinsics must accept boxed calls but clippy only
// sees the local function
#[allow(clippy::unused_unit, clippy::boxed_local)]
#[frame_support::pallet]
pub mod pallet {
	pub use crate::types::*;
	use crate::{traits::*, utils};
	use frame_support::{
		dispatch::{Codec, DispatchResultWithPostInfo},
		pallet_prelude::*,
		sp_runtime::traits::{CheckedAdd, Dispatchable, One, Saturating, Zero},
		sp_std::{boxed::Box, prelude::*, vec::Vec},
		transactional,
		weights::{GetDispatchInfo, PostDispatchInfo},
	};
	use frame_system::pallet_prelude::*;

	type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
	type HashFor<T> = <T as frame_system::Config>::Hash;
	type BlockNumberFor<T> = <T as frame_system::Config>::BlockNumber;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The outer origin type.
		type Origin: From<CommitteeOrigin<Self::AccountId, Self::BlockNumber>>;
		/// The outer call dispatch type.
		type Action: Parameter
			+ Dispatchable<Origin = <Self as Config>::Origin, PostInfo = PostDispatchInfo>
			+ From<frame_system::Call<Self>>
			+ GetDispatchInfo;

		/// A unique number assigned to each new instance of a proposal
		type ProposalNonce: Parameter + Member + One + Zero + Codec + Default + MaybeSerializeDeserialize + CheckedAdd;

		/// Duration (in blocks) of the proposal submission period
		type ProposalSubmissionPeriod: Get<Self::BlockNumber>;

		/// Duration (in blocks) of the voting period
		type VotingPeriod: Get<Self::BlockNumber>;

		/// Range of the voting period
		type VotingPeriodRange: VotingPeriodRange<Self::BlockNumber>;

		/// Minimum number of council members that must vote for a action to be
		/// passed
		type MinCouncilVotes: Get<usize>;

		/// Origin that is permitted to create proposals
		type ProposalSubmissionOrigin: EnsureOrigin<
			<Self as frame_system::Config>::Origin,
			Success = <Self as frame_system::Config>::AccountId,
		>;

		/// Origin that is permitted to execute approved proposals
		type ProposalExecutionOrigin: EnsureOrigin<
			<Self as frame_system::Config>::Origin,
			Success = <Self as frame_system::Config>::AccountId,
		>;

		/// Origin that is permitted to execute priviliged extrinsics
		type ApprovedByCommitteeOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The weight for this pallet's extrinsics.
		type WeightInfo: WeightInfo;
	}

	#[pallet::origin]
	pub type Origin<T> = CommitteeOrigin<AccountIdFor<T>, BlockNumberFor<T>>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// Storage defs

	/// Stores a vector of the hashes of currently active proposals for
	/// iteration
	#[pallet::storage]
	pub type ActiveProposals<T: Config> = StorageValue<_, Vec<HashFor<T>>, ValueQuery>;

	/// Increments with each new proposal. Used to produce a unique nonce per
	/// proposal instance
	#[pallet::storage]
	pub type ProposalCount<T: Config> = StorageValue<_, T::ProposalNonce, ValueQuery>;

	/// Store a mapping (hash) -> Proposal for all existing proposals.
	#[pallet::storage]
	pub type Proposals<T: Config> = StorageMap<_, Identity, HashFor<T>, Proposal<T>, OptionQuery>;

	/// Maps accountIDs to their member type (council or constituent)
	#[pallet::storage]
	pub type Members<T: Config> = StorageMap<_, Blake2_128Concat, AccountIdFor<T>, MemberType, OptionQuery>;

	/// Store a mapping (hash) -> VoteAggregate for all existing proposals.
	#[pallet::storage]
	pub type Votes<T: Config> =
		StorageMap<_, Blake2_128Concat, HashFor<T>, VoteAggregate<AccountIdFor<T>, BlockNumberFor<T>>, OptionQuery>;

	/// Store a duration (in blocks) of the voting period
	#[pallet::storage]
	pub type VotingPeriod<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	/// Stores the block height at which point a member is eligible to cast their vote
	///
	/// For new members, this will be the block they were added as members plus the duration of one
	/// voting period. This means new members are eligible to cast their vote in the next voting
	/// period.
	#[pallet::storage]
	pub type VotingEligibility<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdFor<T>, BlockNumberFor<T>, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub council_members: Vec<T::AccountId>,
		pub constituent_members: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { council_members: Default::default(), constituent_members: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			VotingPeriod::<T>::set(T::VotingPeriod::get());

			for member in &self.council_members {
				Members::<T>::insert(member, MemberType::Council);
				VotingEligibility::<T>::insert(member, T::BlockNumber::zero());
			}

			for member in &self.constituent_members {
				Members::<T>::insert(member, MemberType::Constituent);
				VotingEligibility::<T>::insert(member, T::BlockNumber::zero());
			}
		}
	}

	// end storage defs

	#[pallet::event]
	#[pallet::metadata(T::ProposalNonce = "ProposalNonce",T::Hash = "Hash", AccountIdFor<T> = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new proposal has been created
		/// \[proposer_address, proposal_nonce, proposal_hash\]
		Proposed(AccountIdFor<T>, T::ProposalNonce, T::Hash),
		/// A vote was cast
		/// \[voter_address, proposal_hash, vote\]
		VoteCast(CommitteeMember<AccountIdFor<T>>, T::Hash, VoteKind),
		/// A proposal was closed and executed. Any errors for calling the
		/// proposal action are included
		/// \[proposal_hash, result\]
		ClosedAndExecutedProposal(T::Hash, DispatchResult),
		/// A new consituent has been added
		/// \[constituent_address]
		NewConstituent(AccountIdFor<T>),
		/// A member has been removed
		/// \[member_type, address]
		RemoveMember(AccountIdFor<T>, MemberType),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The origin making the call is not a member and it is a requirement
		/// that they are
		NotMember,
		/// Member has attempted to vote multiple times on a single proposal
		DuplicateVote,
		/// Attempted to cast a vote outside the accepted voting period for a
		/// proposal
		NotInVotingPeriod,
		/// Attempted to cast a vote without voting eligibility
		NotEligibileToVoteYet,
		/// Attempted to add a constituent that is already a member of the
		/// council
		AlreadyCouncilMember,
		/// Attempted to add a constituent that is already a constituent
		AlreadyConstituentMember,
		/// Attempted to close a proposal before the voting period is over
		VotingPeriodNotElapsed,
		/// Tried to close a proposal but not enough council members voted
		ProposalNotAcceptedInsufficientVotes,
		/// Tried to close a proposal but the constituent members voted to veto
		/// proposal
		ProposalNotAcceptedConstituentVeto,
		/// Tried to close a proposal but proposal was denied by council
		ProposalNotAcceptedCouncilDeny,
		/// Attempted to execute a proposal that is timeout
		ProposalTimeout,
		/// Attempted to execute a proposal that has already been executed
		ProposalAlreadyExecuted,
		/// Reach the minimal number of the limit of council members
		MinimalCouncilMembers,
		/// The hash provided does not have an associated proposal
		NoProposalWithHash,
		/// The data type for enumerating the proposals has reached its upper
		/// bound. No more proposals can be made
		ProposalNonceExhausted,
		/// There was a numerical overflow or underflow in calculating when the
		/// voting period should end
		InvalidOperationInEndBlockComputation,
		/// Attempted to set VotingPeriod out of the range of 7 days ~ 28 days
		InvalidVotingPeriod,
	}

	impl<T> From<VoteRejectionReason> for Error<T> {
		fn from(reason: VoteRejectionReason) -> Self {
			match reason {
				VoteRejectionReason::InsuffientVotes => Self::ProposalNotAcceptedInsufficientVotes,
				VoteRejectionReason::ConstituentVeto => Self::ProposalNotAcceptedConstituentVeto,
				VoteRejectionReason::CouncilDeny => Self::ProposalNotAcceptedCouncilDeny,
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			// perform upkeep only at the start of a new cycle
			match Self::get_next_voting_period_end(&n) {
				Ok(end) => {
					if end == n + VotingPeriod::<T>::get() + T::ProposalSubmissionPeriod::get() {
						return Self::upkeep(n);
					}
				}
				Err(err) => {
					// this can only happen due to misconfig, in which case we log the error
					log::error!("Failed to determine next voting period end: {:?}", err);
				}
			}
			0
		}
	}

	impl<T: Config> Pallet<T> {
		/// Gets a new unused proposal nonce and increments the nonce in the
		/// store Returns an error if the data type used for the nonce
		/// exceeds is maximum value
		fn take_and_increment_nonce() -> Result<T::ProposalNonce, Error<T>> {
			ProposalCount::<T>::try_mutate(|nonce| -> Result<T::ProposalNonce, Error<T>> {
				let current = nonce.clone();
				*nonce = current.checked_add(&One::one()).ok_or(Error::ProposalNonceExhausted)?;
				Ok(current)
			})
		}

		pub fn active_proposals() -> Vec<HashFor<T>> {
			ActiveProposals::<T>::get()
		}

		pub fn get_proposal(hash: &HashFor<T>) -> Option<Proposal<T>> {
			Proposals::<T>::get(hash)
		}

		/// Get the votes for a proposal. Returns None if no proposal exists
		pub fn get_votes_for(hash: &HashFor<T>) -> Option<VoteAggregate<AccountIdFor<T>, BlockNumberFor<T>>> {
			Votes::<T>::get(hash)
		}

		/// Returns the block at the end of the next voting period
		pub fn get_next_voting_period_end(
			block_number: &BlockNumberFor<T>,
		) -> Result<BlockNumberFor<T>, DispatchError> {
			utils::get_vote_end(block_number, &VotingPeriod::<T>::get(), &T::ProposalSubmissionPeriod::get())
				.ok_or_else(|| Error::<T>::InvalidOperationInEndBlockComputation.into())
		}

		/// Return true if the current block indicates it is the voting period
		/// for the given VoteAggregate.
		pub fn within_voting_period(votes: &VoteAggregate<AccountIdFor<T>, BlockNumberFor<T>>) -> bool {
			let current_block = frame_system::Pallet::<T>::block_number();
			current_block < votes.end && current_block >= votes.end - VotingPeriod::<T>::get()
		}

		/// Function executed at the initialization of the first block in
		/// a new voting period cycle. Used to maintain the active proposals
		/// store.
		///
		/// Returns the consumed weight:
		///
		/// `Storage: ActiveProposals (r:1 w:1) + Votes (r1) * len(proposals)`
		fn upkeep(n: BlockNumberFor<T>) -> Weight {
			// ActiveProposals.retain (r:1 w:1)
			let mut reads: Weight = 1;
			let writes: Weight = 1;

			// clear out proposals that are no longer active
			ActiveProposals::<T>::mutate(|proposals| {
				// consumed weight for all `Storage: Votes (r1)` lookups
				reads = reads.saturating_add(proposals.len() as Weight);

				proposals.retain(|hash| if let Some(votes) = Self::get_votes_for(hash) { votes.end > n } else { false })
			});

			T::DbWeight::get().reads_writes(reads, writes)
		}

		/// Used to check if an origin is signed and the signer is a member of
		/// the committee
		pub fn ensure_member(origin: OriginFor<T>) -> Result<CommitteeMember<AccountIdFor<T>>, DispatchError> {
			let who = ensure_signed(origin)?;
			let members = Members::<T>::get(&who).ok_or(Error::<T>::NotMember)?;
			Ok(CommitteeMember::new(who, members))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic to propose a new action to be voted upon in the next
		/// voting period.
		///
		/// The provided action will be turned into a proposal and added to the list of current
		/// active proposals to be voted on in the next voting period.
		#[pallet::weight(T::WeightInfo::propose())]
		pub fn propose(origin: OriginFor<T>, action: Box<T::Action>) -> DispatchResultWithPostInfo {
			let proposer = T::ProposalSubmissionOrigin::ensure_origin(origin)?;

			// Create a new proposal with a unique nonce
			let nonce = Self::take_and_increment_nonce()?;
			let proposal = Proposal::<T>::new(*action, proposer.clone(), nonce.clone(), ProposalStatus::Active);

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

		/// Extrinsic to vote on an existing proposal.
		///
		/// This can only be called by members of the committee that are eligible to vote.
		///
		/// New members are eligible to vote after 1 voting period has passed from the block they
		/// were added to the members set. Successfully cast votes will be recorded in the state and
		/// a proposal meeting voting requirements can be executed.
		#[transactional]
		#[pallet::weight((T::WeightInfo::vote(), DispatchClass::Operational))]
		pub fn vote(origin: OriginFor<T>, proposal_hash: HashFor<T>, vote: VoteKind) -> DispatchResult {
			let voter = Self::ensure_member(origin)?;

			VotingEligibility::<T>::get(&voter.account_id)
				.filter(|block_number| frame_system::Pallet::<T>::block_number() >= *block_number)
				.ok_or(Error::<T>::NotEligibileToVoteYet)?;

			Votes::<T>::try_mutate(&proposal_hash, |maybe_votes| -> DispatchResult {
				let votes = maybe_votes.as_mut().ok_or(Error::<T>::NoProposalWithHash)?;

				// Can only vote within the allowed range of blocks for this proposal
				ensure!(Self::within_voting_period(&votes), Error::<T>::NotInVotingPeriod);
				// members can vote only once
				ensure!(!votes.has_voted(&voter.account_id), Error::<T>::DuplicateVote);
				votes.cast_vote(MemberVote::new(voter.clone(), vote.clone())); // mutates votes in place

				Self::deposit_event(Event::VoteCast(voter, proposal_hash, vote));
				Ok(())
			})
		}

		/// Extrinsic to close and execute a proposal.
		///
		/// Proposal must have been voted on and have majority approval.
		///
		/// Only the proposal execution origin can execute.
		#[pallet::weight((T::WeightInfo::close(), DispatchClass::Operational))]
		pub fn close(origin: OriginFor<T>, proposal_hash: HashFor<T>) -> DispatchResultWithPostInfo {
			T::ProposalExecutionOrigin::ensure_origin(origin)?;

			// register that this proposal has been executed
			Proposals::<T>::try_mutate_exists(&proposal_hash, |maybe_proposal| -> DispatchResultWithPostInfo {
				let mut proposal = maybe_proposal.take().ok_or(Error::<T>::NoProposalWithHash)?;
				let votes = Self::get_votes_for(&proposal_hash).ok_or(Error::<T>::NoProposalWithHash)?;
				let current_block = frame_system::Pallet::<T>::block_number();

				// ensure proposal has not already been executed
				(match proposal.status {
					ProposalStatus::Active => {
						// proposal timeout after two voting periods
						if current_block.saturating_sub(votes.end) >= VotingPeriod::<T>::get() {
							proposal.status = ProposalStatus::Timeout;
							*maybe_proposal = Some(proposal);
							return Ok(().into());
						}

						Ok(())
					}
					ProposalStatus::Timeout => Err(Error::<T>::ProposalTimeout),
					ProposalStatus::Executed => Err(Error::<T>::ProposalAlreadyExecuted),
				})?;

				// Ensure voting period is over
				ensure!(current_block > votes.end, Error::<T>::VotingPeriodNotElapsed);

				// Ensure voting has accepted proposal
				votes.is_accepted(T::MinCouncilVotes::get()).map_err(Into::<Error<T>>::into)?;

				// Execute the proposal
				let result = proposal
					.action
					.clone()
					.dispatch(Origin::<T>::ApprovedByCommittee(proposal.issuer.clone(), votes).into());

				proposal.status = ProposalStatus::Executed;
				*maybe_proposal = Some(proposal);

				Self::deposit_event(Event::ClosedAndExecutedProposal(
					proposal_hash,
					result.map(|_| ()).map_err(|e| e.error),
				));

				// TODO: Handle weight used by the dispatch call in weight calculation

				Ok(().into())
			})
		}

		/// Add new constituent to the committee
		///
		/// This call can only be called after the approval of the committee
		#[pallet::weight(T::WeightInfo::add_constituent())]
		pub fn add_constituent(origin: OriginFor<T>, constituent: AccountIdFor<T>) -> DispatchResult {
			T::ApprovedByCommitteeOrigin::ensure_origin(origin)?;

			Members::<T>::try_mutate(constituent.clone(), |member| -> Result<(), DispatchError> {
				if let Some(ty) = member {
					Err(match ty {
						MemberType::Council => <Error<T>>::AlreadyCouncilMember,
						MemberType::Constituent => <Error<T>>::AlreadyConstituentMember,
					}
					.into())
				} else {
					*member = Some(MemberType::Constituent);
					Ok(())
				}
			})?;

			let block_numer = frame_system::Pallet::<T>::block_number();
			VotingEligibility::<T>::insert(&constituent, block_numer + Self::get_next_voting_period_end(&block_numer)?);

			Self::deposit_event(Event::NewConstituent(constituent));
			Ok(())
		}

		/// Remove council or constituent via governance
		///
		/// This call can only be called after the approval of the committee
		#[pallet::weight(T::WeightInfo::remove_member())]
		pub fn remove_member(origin: OriginFor<T>, member: AccountIdFor<T>) -> DispatchResult {
			T::ApprovedByCommitteeOrigin::ensure_origin(origin)?;

			let ty = Members::<T>::try_mutate_exists(&member, |maybe_member| -> Result<MemberType, DispatchError> {
				let ty = maybe_member.take().ok_or(Error::<T>::NotMember)?;

				// Check if have enough council members
				if ty == MemberType::Constituent
					|| Members::<T>::iter_values().filter(|m| *m == MemberType::Council).count()
						> T::MinCouncilVotes::get()
				{
					VotingEligibility::<T>::take(&member);
					Ok(ty)
				} else {
					Err(Error::<T>::MinimalCouncilMembers.into())
				}
			})?;

			Self::deposit_event(Event::RemoveMember(member, ty));
			Ok(())
		}

		/// Set voting period
		///
		/// only accept 7~28 days
		#[pallet::weight(T::WeightInfo::set_voting_period())]
		pub fn set_voting_period(origin: OriginFor<T>, voting_period: T::BlockNumber) -> DispatchResult {
			T::ApprovedByCommitteeOrigin::ensure_origin(origin)?;

			ensure!(
				T::VotingPeriodRange::min() < voting_period && voting_period < T::VotingPeriodRange::max(),
				Error::<T>::InvalidVotingPeriod
			);

			VotingPeriod::<T>::set(voting_period);
			Ok(())
		}
	}

	/// Trait for the asset-index pallet extrinsic weights.
	pub trait WeightInfo {
		fn propose() -> Weight;
		fn vote() -> Weight;
		fn close() -> Weight;
		fn add_constituent() -> Weight;
		fn remove_member() -> Weight;
		fn set_voting_period() -> Weight;
	}

	/// For backwards compatibility and tests
	impl WeightInfo for () {
		fn propose() -> Weight {
			Default::default()
		}

		fn vote() -> Weight {
			Default::default()
		}

		fn close() -> Weight {
			Default::default()
		}

		fn add_constituent() -> Weight {
			Default::default()
		}

		fn remove_member() -> Weight {
			Default::default()
		}

		fn set_voting_period() -> Weight {
			Default::default()
		}
	}
}
