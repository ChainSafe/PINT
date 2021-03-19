#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    use frame_system::RawOrigin;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::{traits::Dispatchable},
        weights::{GetDispatchInfo, PostDispatchInfo}
    };
    use frame_system::pallet_prelude::*;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    type HashFor<T> = <T as frame_system::Config>::Hash;

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    /// Info for keeping track of a motion being voted on.
    pub struct VoteAggregate<AccountId, BlockNumber> {
        /// The proposal's unique index.
        index: u32,
        /// The current set of voters that approved it.
        ayes: Vec<AccountId>,
        /// The current set of voters that rejected it.
        nays: Vec<AccountId>,
        /// The hard end time of this vote.
        end: BlockNumber,
    }

    impl<AccountId, BlockNumber: std::default::Default> Default for VoteAggregate<AccountId, BlockNumber> {
        fn default() -> Self {
            Self {
                index: 0,
                ayes: Vec::new(),
                nays: Vec::new(),
                end: Default::default()
            }
        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    /// Possible votes a member can cast
    pub enum Vote {
        Aye,
        Nay,
        Abstain
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // Storage defs

    #[pallet::storage]
    /// Stores a vector of the hashes of currently active proposals for iteration
    pub type ActiveProposals<T: Config> = StorageValue<_, Vec<HashFor<T>>, ValueQuery>;

    #[pallet::storage]
    /// Store a mapping (hash) -> Proposal for all existing proposals
    pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, HashFor<T>, T::Proposal, OptionQuery>;

    #[pallet::storage]
    /// Store a mapping (hash) -> VoteAggregate for all existing proposals
    pub type Votes<T: Config> = StorageMap<_, Blake2_128Concat, HashFor<T>, VoteAggregate<AccountIdFor<T>, T::BlockNumber>, OptionQuery>;

    #[pallet::storage]
    /// Store a vector of all members account IDs
    pub type Members<T: Config> = StorageValue<_, Vec<AccountIdFor<T>>, ValueQuery>;

    // end storage defs


    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The outer origin type.
        type Origin: From<RawOrigin<Self::AccountId>>;
        /// The outer call dispatch type.
        type Proposal: Parameter
            + Dispatchable<Origin=<Self as Config>::Origin, PostInfo=PostDispatchInfo>
            + From<frame_system::Call<Self>>
            + GetDispatchInfo;

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

    // /// Origin for the committee module.
    // #[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
    // pub enum RawOrigin<AccountId> {
    //     Member(AccountId),
    // }

    /// Origin for the committee module.
    // pub type Origin<T> = RawOrigin<<T as frame_system::Config>::AccountId>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn withdraw(
            origin: OriginFor<T>,
        ) -> DispatchResultWithPostInfo {
            Ok(().into())
        }
    }
}
