// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
// requires unused_unit exception as the #[pallet::event] proc macro generates code that violates this lint
// requires boxed_local exception as extrincis must accept boxed calls but clippy only sees the local function
#[allow(clippy::unused_unit, clippy::boxed_local)]
pub mod pallet {
    use frame_support::{
        dispatch::{Codec, DispatchResultWithPostInfo},
        pallet_prelude::*,
        sp_runtime::traits::Dispatchable,
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
    pub struct VoteAggregate<T: Config> {
        /// The current set of voters that approved it.
        ayes: Vec<AccountIdFor<T>>,
        /// The current set of voters that rejected it.
        nays: Vec<AccountIdFor<T>>,
        /// The hard end time of this vote.
        end: BlockNumberFor<T>,
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

    // end storage defs

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Proposed(AccountIdFor<T>, T::ProposalNonce, T::Hash),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The data type for enumerating the proposals has reached its upper bound.
        /// No more proposals can be made
        ProposalNonceExhausted,
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

            // Add the proposal to the active proposals
            <ActiveProposals<T>>::append(&proposal_hash);

            Self::deposit_event(Event::Proposed(proposer, nonce, proposal_hash));

            Ok(().into())
        }
    }
}
