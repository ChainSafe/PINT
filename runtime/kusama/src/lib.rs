// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

// A few exports that help ease life for downstream crates.
use codec::Decode;
// Polkadot imports
use cumulus_primitives_core::ParaId;
pub use frame_support::{
	construct_runtime, match_type, ord_parameter_types, parameter_types,
	traits::{IsInVec, Randomness},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		DispatchClass, IdentityFee, Weight,
	},
	PalletId, StorageValue,
};

// orml imports
use orml_currencies::BasicCurrencyAdapter;
use orml_xcm_support::{IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use sp_api::impl_runtime_apis;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, Convert},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};
pub use sp_runtime::{Perbill, Permill, Perquintill};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use xcm::v0::{BodyId, Junction, Junction::*, MultiAsset, MultiLocation, MultiLocation::*, NetworkId};
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, EnsureXcmOrigin, FixedRateOfConcreteFungible,
	FixedWeightBounds, LocationInverter, ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
	TakeRevenue, TakeWeightCredit,
};
use xcm_executor::XcmExecutor;

use frame_support::traits::Everything;
use pallet_committee::EnsureMember;
pub use pint_runtime_common::{
	constants::*,
	types::{CommitteeOrigin, GovernanceOrigin},
	weights,
};
use primitives::traits::MultiAssetRegistry;
pub use primitives::*;
use xcm_calls::{
	proxy::{ProxyCallEncoder, ProxyType},
	staking::StakingCallEncoder,
	PalletCallEncoder, PassthroughCompactEncoder, PassthroughEncoder,
};

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Opaque types. These are used by the CLI to instantiate machinery that don't
/// need to know the specifics of the runtime. They can then be made to be
/// agnostic over specific formats of data like extrinsics, allowing for them to
/// continue syncing the network through upgrades to even the core data
/// structures.
pub mod opaque {
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	use super::*;

	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;

	pub type SessionHandlers = ();

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
		}
	}
}

pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("pint-parachain"),
	impl_name: create_runtime_str!("pint-parachain"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// The version information used to identify this runtime when compiled
/// natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub Ancestry: MultiLocation = Junction::Parachain(
		ParachainInfo::parachain_id().into()
	).into();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub SelfLocation: MultiLocation = MultiLocation::X2(Junction::Parent, Junction::Parachain(ParachainInfo::parachain_id().into()));
	pub const Version: RuntimeVersion = VERSION;
	pub const ProposalSubmissionPeriod: BlockNumber = 10;
	pub const VotingPeriod: BlockNumber = 10;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in
	/// dispatchers.
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
	/// Maximum number of block number to block hash mappings to keep (oldest
	/// pruned first).
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
	/// This is used as an identifier of the chain. 42 is the generic substrate
	/// prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic of the parachain.
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
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
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
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

impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

/// Type for specifying how a `MultiLocation` can be converted into an
/// `AccountId`. This is used when determining ownership of accounts for asset
/// transacting and when attempting to use XCM `Transact` in order to determine
/// the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the default `AccountId`.
	ParentIsDefault<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = MultiCurrencyAdapter<
	// Use this multicurrency for asset balances
	Currencies,
	// handle in event of unknown tokens
	UnknownTokens,
	// Convert
	IsNativeConcrete<AssetId, AssetIdConvert>,
	AccountId,
	LocationToAccountId,
	AssetId,
	AssetIdConvert,
>;

/// This is the type we use to convert an (incoming) XCM origin into a local
/// `Origin` instance, ready for dispatching a transaction with Xcm's
/// `Transact`. There is an `OriginKind` which can biases the kind of local
/// `Origin` it will become.
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
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

match_type! {
	pub type ParentOrParentsUnitPlurality: impl Contains<MultiLocation> = {
		X1(Parent) | X2(Parent, Plurality { id: BodyId::Unit, .. })
	};
}

pub type Barrier = (TakeWeightCredit, AllowTopLevelPaidExecutionFrom<Everything>);

pub struct ToTreasury;
impl TakeRevenue for ToTreasury {
	fn take_revenue(revenue: MultiAsset) {
		use orml_traits::currency::MultiCurrency;
		if let MultiAsset::ConcreteFungible { id, amount } = revenue {
			if let Some(asset_id) = AssetIdConvert::convert(id) {
				// ensure PINT Treasury account have ed for all of the cross-chain asset.
				// Ignore the result.
				let _ = Currencies::deposit(asset_id, &PintTreasuryAccount::get(), amount);
			}
		}
	}
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = MultiNativeAsset;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	type Trader = FixedRateOfConcreteFungible<UnitPerSecond, ToTreasury>;
	type ResponseHandler = (); // Don't handle responses for now.
}

pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into
/// the right message queues.
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
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = ();
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	type LocationInverter = LocationInverter<Ancestry>;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = CollatorSelection;
}

impl pallet_session::Config for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <opaque::SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type WeightInfo = ();
}

impl pallet_collator_selection::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type UpdateOrigin = GovernanceOrigin<AccountId, Runtime>;
	type PotId = PotId;
	type MaxCandidates = MaxCandidates;
	type MinCandidates = MinCandidates;
	type MaxInvulnerables = MaxInvulnerables;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = ();
}

impl pallet_local_treasury::Config for Runtime {
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type Event = Event;
	type WeightInfo = weights::pallet_local_treasury::WeightInfo<Self>;
}

impl pallet_remote_treasury::Config for Runtime {
	type Event = Event;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type Balance = Balance;
	type AssetId = AssetId;
	type PalletId = TreasuryPalletId;
	type SelfAssetId = PINTAssetId;
	type RelayChainAssetId = RelayChainAssetId;
	type XcmAssetTransfer = XTokens;
	type AssetIdConvert = AssetIdConvert;
	type AccountId32Convert = AccountId32Convert;
	type WeightInfo = ();
}

impl pallet_saft_registry::Config for Runtime {
	type AdminOrigin = CommitteeOrigin<Runtime>;
	type AssetRecorder = AssetIndex;
	#[cfg(feature = "runtime-benchmarks")]
	type AssetRecorderBenchmarks = AssetIndex;
	type Balance = Balance;
	type AssetId = AssetId;
	type Event = Event;
	type WeightInfo = weights::pallet_saft_registry::WeightInfo<Runtime>;
}

impl pallet_committee::Config for Runtime {
	type Origin = Origin;
	type Action = Call;
	type ProposalNonce = u32;
	type Days = Days;
	type ProposalSubmissionPeriod = ProposalSubmissionPeriod;
	type VotingPeriod = VotingPeriod;
	type MinCouncilVotes = MinCouncilVotes;
	type ProposalExecutionOrigin = EnsureMember<Self>;
	type ProposalSubmissionOrigin = EnsureMember<Self>;
	type ApprovedByCommitteeOrigin = GovernanceOrigin<AccountId, Runtime>;
	type Event = Event;
	type WeightInfo = weights::pallet_committee::WeightInfo<Runtime>;
}

impl pallet_price_feed::Config for Runtime {
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type SelfAssetId = PINTAssetId;
	type AssetId = AssetId;
	type Time = Timestamp;
	type Event = Event;
	type WeightInfo = weights::pallet_price_feed::WeightInfo<Runtime>;
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
	type OnAnswerHandler = PriceFeed;
	type WeightInfo = ();
}

impl pallet_asset_index::Config for Runtime {
	type AdminOrigin = CommitteeOrigin<Runtime>;
	type IndexToken = Balances;
	type Balance = Balance;
	type MaxActiveDeposits = MaxActiveDeposits;
	type MaxDecimals = MaxDecimals;
	type RedemptionFee = ();
	type LockupPeriod = LockupPeriod;
	type IndexTokenLockIdentifier = IndexTokenLockIdentifier;
	type MinimumRedemption = MinimumRedemption;
	type WithdrawalPeriod = WithdrawalPeriod;
	type RemoteAssetManager = RemoteAssetManager;
	type AssetId = AssetId;
	type SelfAssetId = PINTAssetId;
	type Currency = Currencies;
	type PriceFeed = PriceFeed;
	#[cfg(feature = "runtime-benchmarks")]
	type PriceFeedBenchmarks = PriceFeed;
	type SaftRegistry = SaftRegistry;
	type BaseWithdrawalFee = BaseWithdrawalFee;
	type TreasuryPalletId = TreasuryPalletId;
	type Event = Event;
	type StringLimit = PalletIndexStringLimit;
	type WeightInfo = weights::pallet_asset_index::WeightInfo<Self>;
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = AssetId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = orml_tokens::TransferDust<Runtime, PintTreasuryAccount>;
	type MaxLocks = MaxLocks;
	type DustRemovalWhitelist = DustRemovalWhitelist;
}

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type GetNativeCurrencyId = PINTAssetId;
	type WeightInfo = ();
}

impl orml_xtokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = AssetId;
	type CurrencyIdConvert = AssetIdConvert;
	type AccountIdToMultiLocation = AccountId32Convert;
	type SelfLocation = SelfLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	type BaseXcmWeight = BaseXcmWeight;
}

impl orml_unknown_tokens::Config for Runtime {
	type Event = Event;
}

pub struct AssetIdConvert;
impl Convert<AssetId, Option<MultiLocation>> for AssetIdConvert {
	fn convert(asset: AssetId) -> Option<MultiLocation> {
		AssetIndex::native_asset_location(&asset)
	}
}

impl Convert<MultiLocation, Option<AssetId>> for AssetIdConvert {
	fn convert(location: MultiLocation) -> Option<AssetId> {
		match location {
			MultiLocation::X1(Junction::Parent) => return Some(RelayChainAssetId::get()),
			MultiLocation::X3(Junction::Parent, Junction::Parachain(id), Junction::GeneralKey(key))
				if ParaId::from(id) == ParachainInfo::parachain_id() =>
			{
				// decode the general key
				if let Ok(asset_id) = AssetId::decode(&mut &key[..]) {
					// check `asset_id` is supported
					if AssetIndex::is_liquid_asset(&asset_id) {
						return Some(asset_id);
					}
				}
			}
			_ => {}
		}
		None
	}
}

impl Convert<MultiAsset, Option<AssetId>> for AssetIdConvert {
	fn convert(asset: MultiAsset) -> Option<AssetId> {
		if let MultiAsset::ConcreteFungible { ref id, amount: _ } = asset {
			Self::convert(id.clone())
		} else {
			None
		}
	}
}

pub struct AccountId32Convert;
impl Convert<AccountId, [u8; 32]> for AccountId32Convert {
	fn convert(account_id: AccountId) -> [u8; 32] {
		account_id.into()
	}
}

impl Convert<AccountId, MultiLocation> for AccountId32Convert {
	fn convert(account_id: AccountId) -> MultiLocation {
		Junction::AccountId32 { network: NetworkId::Any, id: Self::convert(account_id) }.into()
	}
}

/// The encoder to use when transacting `pallet_proxy` calls
pub struct PalletProxyEncoder;
impl ProxyCallEncoder<AccountId, ProxyType, BlockNumber> for PalletProxyEncoder {
	type AccountIdEncoder = PassthroughEncoder<AccountId, AssetId>;
	type ProxyTypeEncoder = PassthroughEncoder<ProxyType, AssetId>;
	type BlockNumberEncoder = PassthroughEncoder<BlockNumber, AssetId>;
}
impl PalletCallEncoder for PalletProxyEncoder {
	type Context = AssetId;
	fn can_encode(_ctx: &Self::Context) -> bool {
		// TODO check in `AssetRegistry`
		true
	}
}

type AccountLookupSource = sp_runtime::MultiAddress<AccountId, ()>;

/// The encoder to use when transacting `pallet_staking` calls
pub struct PalletStakingEncoder;
impl StakingCallEncoder<AccountLookupSource, Balance, AccountId> for PalletStakingEncoder {
	type CompactBalanceEncoder = PassthroughCompactEncoder<Balance, AssetId>;
	type SourceEncoder = PassthroughEncoder<AccountLookupSource, AssetId>;
	type AccountIdEncoder = PassthroughEncoder<AccountId, AssetId>;
}

impl PalletCallEncoder for PalletStakingEncoder {
	type Context = AssetId;
	fn can_encode(_ctx: &Self::Context) -> bool {
		// TODO check in `AssetRegistry`
		true
	}
}

impl pallet_remote_asset_manager::Config for Runtime {
	type Balance = Balance;
	type AssetId = AssetId;
	type AssetIdConvert = AssetIdConvert;
	// Encodes `pallet_staking` calls before transaction them to other chains
	type PalletStakingCallEncoder = PalletStakingEncoder;
	// Encodes `pallet_proxy` calls before transaction them to other chains
	type PalletProxyCallEncoder = PalletProxyEncoder;
	type MinimumStatemintTransferAmount = MinimumStatemintTransferAmount;
	type SelfAssetId = PINTAssetId;
	type SelfLocation = SelfLocation;
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type RelayChainAssetId = RelayChainAssetId;
	type AssetStakingCap = (MinimumRemoteReserveBalance, MinimumBondExtra);
	type Assets = Currencies;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmAssetTransfer = XTokens;
	// Using root as the admin origin for now
	type AdminOrigin = frame_system::EnsureSigned<AccountId>;
	type XcmSender = XcmRouter;
	type Event = Event;
	type WeightInfo = weights::pallet_remote_asset_manager::WeightInfo<Self>;
}

// Create the runtime by composing the FRAME pallets that were previously
// configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 1,
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 2,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 3,
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 4,

		// Parachain
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Config, Event<T>} = 20,
		ParachainInfo: parachain_info::{Pallet, Storage, Config} = 21,

		// Collator. The order of the 4 below are important and shall not change.
		Authorship: pallet_authorship::{Pallet, Call, Storage} = 40,
		CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>} = 41,
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 42,
		Aura: pallet_aura::{Pallet, Config<T>} = 43,
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Config} = 44,

		// ORML related pallets
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>} = 60,
		Currencies: orml_currencies::{Pallet, Call, Event<T>} = 61,
		XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>} = 62,
		UnknownTokens: orml_unknown_tokens::{Pallet, Storage, Event} = 63,

		// PINT pallets
		AssetIndex: pallet_asset_index::{Pallet, Call, Storage, Event<T>} = 80,
		Committee: pallet_committee::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 81,
		LocalTreasury: pallet_local_treasury::{Pallet, Call, Storage, Event<T>} = 82,
		RemoteTreasury: pallet_remote_treasury::{Pallet, Call, Storage, Event<T>} = 83,
		SaftRegistry: pallet_saft_registry::{Pallet, Call, Storage, Event<T>} = 84,
		RemoteAssetManager: pallet_remote_asset_manager::{Pallet, Call, Storage, Event<T>, Config<T>} = 85,
		PriceFeed: pallet_price_feed::{Pallet, Call, Storage, Event<T>} = 86,
		ChainlinkFeed: pallet_chainlink_feed::{Pallet, Call, Storage, Event<T>, Config<T>} = 90,

		// XCM
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 100,
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 101,
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin} = 102,
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 103
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
pub type Executive =
	frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllPallets, ()>;

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
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
		   Executive::validate_transaction(source, tx, block_hash)
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

		impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities()
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info() -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info()
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

	impl pallet_asset_index_rpc_runtime_api::AssetIndexApi<
		Block,
		AccountId,
		AssetId,
		Balance,
	> for Runtime {
		fn get_nav() -> primitives::Ratio {
			use primitives::traits::NavProvider;
			AssetIndex::nav().unwrap_or_default()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{BenchmarkList, list_benchmark, Benchmarking};
			use frame_support::traits::StorageInfoTrait;

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmark!(list, extra, pallet_asset_index, AssetIndex);
			list_benchmark!(list, extra, pallet_committee, Committee);
			list_benchmark!(list, extra, pallet_local_treasury, LocalTreasury);
			list_benchmark!(list, extra, pallet_price_feed, PriceFeed);
			list_benchmark!(list, extra, pallet_saft_registry, SaftRegistry);

			let storage_info = AllPalletsWithSystem::storage_info();
			return (list, storage_info)
		}

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
			// add_benchmark!(params, batches, pallet_remote_asset_manager, RemoteAssetManager);
			add_benchmark!(params, batches, pallet_saft_registry, SaftRegistry);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
	fn check_inherents(
		block: &Block,
		relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
	) -> sp_inherents::CheckInherentsResult {
		let relay_chain_slot =
			relay_state_proof.read_slot().expect("Could not read the relay chain slot from the proof");

		let inherent_data = cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
			relay_chain_slot,
			sp_std::time::Duration::from_secs(6),
		)
		.create_inherent_data()
		.expect("Could not create the timestamp inherent data");

		inherent_data.check_extrinsics(block)
	}
}

cumulus_pallet_parachain_system::register_validate_block!(
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
	CheckInherents = CheckInherents,
);
