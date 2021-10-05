// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use std::{cell::RefCell, collections::HashSet};

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::BlakeTwo256,
	traits::{Currency, Everything, FindAuthor, Imbalance, InstanceFilter, OnUnbalanced, OneSessionHandler},
	weights::Weight,
};
use sp_core::H256;
use sp_runtime::{
	curve::PiecewiseLinear,
	testing::{Header, TestXt, UintAuthorityId},
	Perbill,
};

use cumulus_primitives_core::ParaId;
use frame_election_provider_support::onchain;
use pallet_staking as staking;
use pallet_staking::*;
use polkadot_runtime_parachains::{configuration, origin, shared, ump};
use xcm::v1::{Junctions, MultiLocation, NetworkId};
use xcm_builder::{
	AccountId32Aliases, AllowUnpaidExecutionFrom, ChildParachainAsNative, ChildParachainConvertsVia,
	ChildSystemParachainAsSuperuser, CurrencyAdapter as XcmCurrencyAdapter, FixedRateOfFungible, FixedWeightBounds,
	IsConcrete, LocationInverter, SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
};
use xcm_executor::{Config, XcmExecutor};

use super::types::*;
use frame_support::sp_runtime::AccountId32;

impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = Lookup;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

impl shared::Config for Runtime {}

impl configuration::Config for Runtime {}

parameter_types! {
	pub const KsmLocation: MultiLocation = Junctions::Here.into();
	pub const KusamaNetwork: NetworkId = NetworkId::Kusama;
	pub const AnyNetwork: NetworkId = NetworkId::Any;
	pub Ancestry: MultiLocation = MultiLocation::default();
	pub UnitWeightCost: Weight = 1_000;
}

pub type SovereignAccountOf =
	(ChildParachainConvertsVia<ParaId, AccountId>, AccountId32Aliases<KusamaNetwork, AccountId>);

pub type LocalAssetTransactor =
	XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, SovereignAccountOf, AccountId, ()>;

type LocalOriginConverter = (
	SovereignSignedViaLocation<SovereignAccountOf, Origin>,
	ChildParachainAsNative<origin::Origin, Origin>,
	SignedAccountId32AsNative<KusamaNetwork, Origin>,
	ChildSystemParachainAsSuperuser<ParaId, Origin>,
);

parameter_types! {
	pub const BaseXcmWeight: Weight = 1_000;
	pub KsmPerSecond: (xcm::v1::AssetId, u128) = (xcm::v1::AssetId::Concrete(KsmLocation::get()), 1);
}

pub type XcmRouter = super::super::RelayChainXcmRouter;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = ();
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<BaseXcmWeight, Call>;
	type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
	type ResponseHandler = ();
	type SubscriptionService = ();
}

pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, KusamaNetwork>;

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	// Anyone can execute XCM messages locally...
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<BaseXcmWeight, Call>;
	type LocationInverter = LocationInverter<Ancestry>;
}

parameter_types! {
	pub const FirstMessageFactorPercent: u64 = 100;
}

impl ump::Config for Runtime {
	type Event = Event;
	type UmpSink = ump::XcmSink<XcmExecutor<XcmConfig>, Runtime>;
	type FirstMessageFactorPercent = FirstMessageFactorPercent;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

impl origin::Config for Runtime {}

// -- staking support

thread_local! {
	static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
}

type NegativeImbalanceOf<T> =
	<<T as pallet_staking::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

/// Another session handler struct to test on_disabled.
pub struct OtherSessionHandler;
impl OneSessionHandler<AccountId> for OtherSessionHandler {
	type Key = UintAuthorityId;

	fn on_genesis_session<'a, I: 'a>(_: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
	}

	fn on_new_session<'a, I: 'a>(_: bool, validators: I, _: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
		SESSION.with(|x| *x.borrow_mut() = (validators.map(|x| x.0.clone()).collect(), HashSet::new()));
	}

	fn on_disabled(validator_index: usize) {
		SESSION.with(|d| {
			let mut d = d.borrow_mut();
			let value = d.0[validator_index].clone();
			d.1.insert(value);
		})
	}
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
	type Public = UintAuthorityId;
}

/// Author of block is always 11
pub struct Author11;
impl FindAuthor<AccountId> for Author11 {
	fn find_author<'a, I>(_digests: I) -> Option<AccountId>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		Some(AccountId32::new([11u8; 32]))
	}
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(
			frame_support::weights::constants::WEIGHT_PER_SECOND * 2
		);
	pub static SessionsPerEra: sp_staking::SessionIndex = 3;
	pub static ExistentialDeposit: Balance = 1;
	pub static SlashDeferDuration: EraIndex = 0;
	pub static Period: BlockNumber = 5;
	pub static Offset: BlockNumber = 0;

	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}
parameter_types! {
	pub const UncleGenerations: u64 = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub other: OtherSessionHandler,
	}
}

impl pallet_session::Config for Runtime {
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Runtime, Staking>;
	type Keys = SessionKeys;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionHandler = (OtherSessionHandler,);
	type Event = Event;
	type ValidatorId = AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Runtime>;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}
parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}
pallet_staking_reward_curve::build! {
	const I_NPOS: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub const BondingDuration: EraIndex = 3;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &I_NPOS;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
}

thread_local! {
	pub static REWARD_REMAINDER_UNBALANCED: RefCell<u128> = RefCell::new(0);
}

pub struct RewardRemainderMock;

impl OnUnbalanced<NegativeImbalanceOf<Runtime>> for RewardRemainderMock {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<Runtime>) {
		REWARD_REMAINDER_UNBALANCED.with(|v| {
			*v.borrow_mut() += amount.peek();
		});
		drop(amount);
	}
}

impl onchain::Config for Runtime {
	type AccountId = AccountId;
	type BlockNumber = BlockNumber;
	type BlockWeights = BlockWeights;
	type Accuracy = Perbill;
	type DataProvider = Staking;
}
impl staking::Config for Runtime {
	const MAX_NOMINATIONS: u32 = 16;
	type Currency = Balances;
	type UnixTime = Timestamp;
	type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
	type RewardRemainder = RewardRemainderMock;
	type Event = Event;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type SlashDeferDuration = SlashDeferDuration;
	type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BondingDuration = BondingDuration;
	type SessionInterface = Self;
	type EraPayout = ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type ElectionProvider = onchain::OnChainSequentialPhragmen<Self>;
	type GenesisElectionProvider = Self::ElectionProvider;
	type WeightInfo = ();
}

parameter_types! {
	pub const ProxyDepositBase: Balance = 100;
	pub const ProxyDepositFactor: Balance = 100;
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = 100;
	pub const AnnouncementDepositFactor: Balance = 100;
	pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug, MaxEncodedLen)]
pub enum ProxyType {
	Any = 0,
	NonTransfer = 1,
	Governance = 2,
	Staking = 3,
	// Skip 4 as it is now removed (was SudoBalances)
	IdentityJudgement = 5,
	CancelProxy = 6,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, _: &Call) -> bool {
		true
	}
}

impl pallet_proxy::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = ();
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = TestXt<Call, ()>;
}

parameter_types! {
	pub const AssetDeposit: u64 = 1;
	pub const ApprovalDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
}

impl pallet_assets::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type AssetId = AssetId;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type WeightInfo = ();
	type Extra = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

// The configured indices for the runtime
pub const STAKING_PALLET_INDEX: u8 = 7u8;
pub const PROXY_PALLET_INDEX: u8 = 29u8;
pub const ASSETS_PALLET_INDEX: u8 = 50u8;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		ParasOrigin: origin::{Pallet, Origin},
		ParasUmp: ump::{Pallet, Call, Storage, Event},
		XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>},

		 // use polkadot index 7
		Staking: staking::{Pallet, Call, Storage, Event<T>} = 7,

		Session: pallet_session::{Pallet, Call, Storage, Event},

		// use polkadot index 29
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 29,

		// use statemint index 50
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>} = 50,
	}
);
