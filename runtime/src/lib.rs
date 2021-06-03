// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, Verify};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, MultiSignature,
};

use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_system::limits::{BlockLength, BlockWeights};

// Polkadot imports
use cumulus_primitives_core::ParaId;
use polkadot_parachain::primitives::Sibling;
use xcm::v0::{Junction, MultiAsset, MultiLocation, NetworkId};
use xcm_builder::{
    AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, EnsureXcmOrigin,
    FixedRateOfConcreteFungible, FixedWeightBounds, LocationInverter, NativeAsset, ParentIsDefault,
    RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
    SignedAccountId32AsNative, SovereignSignedViaLocation, TakeWeightCredit,
};
use xcm_executor::{traits::Convert, Config, XcmExecutor};

// A few exports that help ease life for downstream crates.
use codec::Decode;
pub use frame_support::{
    construct_runtime, ord_parameter_types, parameter_types,
    traits::{All, IsInVec, Randomness},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
    PalletId, StorageValue,
};
use pallet_asset_index::{MultiAssetAdapter, MultiAssetRegistry};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill, Perquintill};
use xcm_executor::traits::MatchesFungible;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Identifier for an asset.
pub type AssetId = u32;

/// Identifier for price feeds.
pub type FeedId = u64;

/// Value type for price feeds.
pub type Value = u128;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;

    pub type SessionHandlers = ();

    impl_opaque_keys! {
        pub struct SessionKeys {}
    }
}

pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("cumulus-test-parachain"),
    impl_name: create_runtime_str!("cumulus-test-parachain"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
};

/// This determines the average expected block time that we are targetting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

#[derive(codec::Encode, codec::Decode)]
pub enum XCMPMessage<XAccountId, XBalance> {
    /// Transfer tokens to the given account from the Parachain account.
    TransferToken(XAccountId, XBalance),
}

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 0.5 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u8 = 0;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = ();
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = AccountIdLookup<AccountId, ()>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a pallet to the index of the pallet in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// The set code logic of the parachain.
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    /// Same as Polkadot Relay Chain.
    pub const ExistentialDeposit: Balance = 500;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1 ;
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type Event = Event;
    type OnValidationData = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type DownwardMessageHandlers = cumulus_primitives_utility::UnqueuedDmpAsParent<
        MaxDownwardMessageWeight,
        XcmExecutor<XcmConfig>,
        Call,
    >;
    type OutboundXcmpMessageSource = XcmpQueue;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl parachain_info::Config for Runtime {}

parameter_types! {
    pub const RococoLocation: MultiLocation = MultiLocation::X1(Junction::Parent);
    pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
    pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
    pub Ancestry: MultiLocation = Junction::Parachain {
        id: ParachainInfo::parachain_id().into()
    }.into();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsDefault<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RococoNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = MultiAssetAdapter<
    // Use this balance type
    Balance,
    // Use this depository for asset balances
    AssetDepository,
    // Use this for a registry of supported asset
    AssetIndex,
    // Use this to convert from fungible to balance type
    IsAsset,
    // The account type
    AccountId,
    // Use this to convert Multilocations to accounts
    LocationToAccountId,
    // The asset identifier type
    AssetId,
    // Use this to determine convert a Multiasset to an AssetId
    AssetIdConvert,
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, Origin>,
    // Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, Origin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RococoNetwork, Origin>,
);

parameter_types! {
    pub UnitWeightCost: Weight = 1_000;
}

parameter_types! {
    // 1_000_000_000_000 => 1 unit of asset for 1 unit of Weight.
    // TODO: Should take the actual weight price. This is just 1_000 ROC per second of weight.
    pub const WeightPrice: (MultiLocation, u128) = (MultiLocation::X1(Junction::Parent), 1_000);
    pub AllowUnpaidFrom: Vec<MultiLocation> = vec![ MultiLocation::X1(Junction::Parent) ];
}

pub type Barrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<All<MultiLocation>>,
    AllowUnpaidExecutionFrom<IsInVec<AllowUnpaidFrom>>, // <- Parent gets free execution
);

pub struct XcmConfig;
impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = XcmRouter;
    // How to withdraw and deposit an asset.
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = NativeAsset;
    type IsTeleporter = NativeAsset;
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
    type Trader = FixedRateOfConcreteFungible<WeightPrice>;
    type ResponseHandler = (); // Don't handle responses for now.
}

parameter_types! {
    pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = ();

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
    type Event = Event;
    type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcm::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
}

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"12345678");
}

impl pallet_local_treasury::Config for Runtime {
    // Using signed as the admin origin for now
    type AdminOrigin = frame_system::EnsureSigned<AccountId>;
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type Event = Event;
}

impl pallet_saft_registry::Config for Runtime {
    // Using signed as the admin origin for now
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type Event = Event;
    type Balance = Balance;
    type AssetRecorder = AssetIndex;
    type AssetId = AssetId;
}

parameter_types! {
    pub const ProposalSubmissionPeriod: <Runtime as frame_system::Config>::BlockNumber = 10;
    pub const VotingPeriod: <Runtime as frame_system::Config>::BlockNumber = 5;
}

ord_parameter_types! {
     pub const MinCouncilVotes: usize = 4;
}

type EnsureApprovedByCommittee = frame_system::EnsureOneOf<
    AccountId,
    frame_system::EnsureRoot<AccountId>,
    pallet_committee::EnsureApprovedByCommittee<AccountId, BlockNumber>,
>;

impl pallet_committee::Config for Runtime {
    type ProposalSubmissionPeriod = ProposalSubmissionPeriod;
    type VotingPeriod = VotingPeriod;
    type MinCouncilVotes = MinCouncilVotes;
    // Using signed as the admin origin for now
    type ProposalSubmissionOrigin = frame_system::EnsureSigned<AccountId>;
    type ProposalExecutionOrigin = frame_system::EnsureSigned<AccountId>;
    type ApprovedByCommitteeOrigin = EnsureApprovedByCommittee;
    type ProposalNonce = u32;
    type Origin = Origin;
    type Action = Call;
    type Event = Event;
}

impl pallet_asset_depository::Config for Runtime {
    type Event = Event;
    type AssetId = AssetId;
    type Balance = Balance;
}

impl pallet_price_feed::Config for Runtime {
    // Using signed as the admin origin for now
    type AdminOrigin = frame_system::EnsureSigned<AccountId>;
    type SelfAssetId = PINTAssetId;
    type AssetId = AssetId;
    type Oracle = ChainlinkFeed;
    type Event = Event;
}

parameter_types! {
    // Used to determine the account for storing the funds used to pay the oracles.
    pub const FeedPalletId: PalletId = PalletId(*b"linkfeed");
    // Minimum amount of funds that need to be present in the fund account
    pub const MinimumReserve: Balance = 100;
    // Maximum allowed string length for feed names
    pub const StringLimit: u32 = 15;
    // Maximum number of oracles per feed
    pub const OracleLimit: u32 = 10;
    // Maximum number of feeds
    pub const FeedLimit: u16 = 10;
    // Number of rounds to keep around per feed
    pub const PruningWindow: u32 = 3;
}

impl pallet_chainlink_feed::Config for Runtime {
    type Event = Event;
    type FeedId = FeedId;
    type Value = Value;
    type Currency = Balances;
    type PalletId = FeedPalletId;
    type MinimumReserve = MinimumReserve;
    type StringLimit = StringLimit;
    type OracleCountLimit = OracleLimit;
    type FeedLimit = FeedLimit;
    type PruningWindow = PruningWindow;
    type WeightInfo = ();
}

parameter_types! {
    pub LockupPeriod: <Runtime as frame_system::Config>::BlockNumber = 10;
    pub MinimumRedemption: u32 = 0;
    pub WithdrawalPeriod: <Runtime as frame_system::Config>::BlockNumber = 10;
    pub DOTContributionLimit: Balance = 999;
}

impl pallet_asset_index::Config for Runtime {
    // TODO:
    //
    // Using signed as the admin origin for testing now
    type AdminOrigin = frame_system::EnsureSigned<AccountId>;
    type Event = Event;
    type AssetId = AssetId;
    type IndexToken = Balances;
    type Balance = Balance;
    type LockupPeriod = LockupPeriod;
    type MinimumRedemption = MinimumRedemption;
    type WithdrawalPeriod = WithdrawalPeriod;
    type DOTContributionLimit = DOTContributionLimit;
    type RemoteAssetManager = RemoteAssetManager;
    type MultiAssetDepository = AssetDepository;
    type PriceFeed = PriceFeed;
    type TreasuryPalletId = TreasuryPalletId;
    type WithdrawalFee = ();
}

parameter_types! {
    pub const RelayChainAssetId: AssetId = 0;
    pub const PINTAssetId: AssetId = 1;
    pub SelfLocation: MultiLocation = MultiLocation::X2(Junction::Parent, Junction::Parachain { id: ParachainInfo::parachain_id().into() });
}

pub struct AssetIdConvert;
impl Convert<AssetId, MultiLocation> for AssetIdConvert {
    fn convert(asset: AssetId) -> sp_std::result::Result<MultiLocation, AssetId> {
        AssetIndex::native_asset_location(&asset).ok_or(asset)
    }
}
impl Convert<MultiLocation, AssetId> for AssetIdConvert {
    fn convert(location: MultiLocation) -> sp_std::result::Result<AssetId, MultiLocation> {
        match &location {
            MultiLocation::X1(Junction::Parent) => return Ok(RelayChainAssetId::get()),
            MultiLocation::X3(
                Junction::Parent,
                Junction::Parachain { id },
                Junction::GeneralKey(key),
            ) if ParaId::from(*id) == ParachainInfo::parachain_id().into() => {
                // decode the general key
                if let Ok(asset_id) = AssetId::decode(&mut &key.clone()[..]) {
                    // check `asset_id` is supported
                    if AssetIndex::is_liquid_asset(&asset_id) {
                        return Ok(asset_id);
                    }
                }
            }
            _ => {}
        }
        Err(location)
    }
}

impl Convert<MultiAsset, AssetId> for AssetIdConvert {
    fn convert(asset: MultiAsset) -> sp_std::result::Result<AssetId, MultiAsset> {
        if let MultiAsset::ConcreteFungible { ref id, amount: _ } = asset {
            Self::convert(id.clone()).map_err(|_| asset)
        } else {
            Err(asset)
        }
    }
}

/// Type to check if an asset is supported and then convert it into native balance
pub struct IsAsset;
impl MatchesFungible<Balance> for IsAsset {
    fn matches_fungible(a: &MultiAsset) -> Option<Balance> {
        if let MultiAsset::ConcreteFungible { id, amount } = a {
            if AssetIdConvert::convert(id.clone()).is_ok() {
                return Some(*amount);
            }
        }
        None
    }
}

pub struct AccountId32Convert;
impl sp_runtime::traits::Convert<AccountId, [u8; 32]> for AccountId32Convert {
    fn convert(account_id: AccountId) -> [u8; 32] {
        account_id.into()
    }
}

impl pallet_remote_asset_manager::Config for Runtime {
    type Balance = Balance;
    type AssetId = AssetId;
    type AssetIdConvert = AssetIdConvert;
    type AccountId32Convert = AccountId32Convert;
    type SelfAssetId = PINTAssetId;
    type SelfLocation = SelfLocation;
    type RelayChainAssetId = RelayChainAssetId;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Event = Event;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Call, Storage},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>},
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage},
        ParachainInfo: parachain_info::{Pallet, Storage, Config},
        Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>},

        // PINT pallets
        AssetIndex: pallet_asset_index::{Pallet, Call, Storage, Event<T>},
        AssetDepository: pallet_asset_depository::{Pallet, Call, Storage, Event<T>},
        Committee: pallet_committee::{Pallet, Call, Storage, Origin<T>, Event<T>},
        LocalTreasury: pallet_local_treasury::{Pallet, Call, Storage, Event<T>},
        SaftRegistry: pallet_saft_registry::{Pallet, Call, Storage, Event<T>},
        RemoteAssetManager: pallet_remote_asset_manager::{Pallet, Call, Storage, Event<T>},
        PriceFeed: pallet_price_feed::{Pallet, Call, Storage, Event<T>},
        ChainlinkFeed: pallet_chainlink_feed::{Pallet, Call, Storage, Event<T>},

        // XCM helpers
        XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
        PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
        CumulusXcm: cumulus_pallet_xcm::{Pallet, Origin},
    }
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various pallets.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPallets,
>;

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            Runtime::metadata().into()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }

        fn random_seed() -> <Block as BlockT>::Hash {
            RandomnessCollectiveFlip::random_seed().0
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

            use frame_system_benchmarking::Pallet as SystemBench;
            impl frame_system_benchmarking::Config for Runtime {}

            let whitelist: Vec<TrackedStorageKey> = vec![
                // Block Number
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
                // Total Issuance
                hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
                // Execution Phase
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
                // Event Count
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
                // System Events
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);

            add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
            add_benchmark!(params, batches, pallet_balances, Balances);
            add_benchmark!(params, batches, pallet_timestamp, Timestamp);
            add_benchmark!(params, batches, pallet_asset_index, AssetIndex);
            add_benchmark!(params, batches, pallet_committee, Committee);
            add_benchmark!(params, batches, pallet_local_treasury, LocalTreasury);
            add_benchmark!(params, batches, pallet_price_feed, PriceFeed);
            add_benchmark!(params, batches, pallet_remote_asset_manager, RemoteAssetManager);
            add_benchmark!(params, batches, pallet_saft_registry, SaftRegistry);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }
}

cumulus_pallet_parachain_system::register_validate_block!(Runtime, Executive);
