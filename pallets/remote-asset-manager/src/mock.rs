// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use cumulus_primitives_core::ParaId;
use frame_support::{
	construct_runtime, ord_parameter_types, parameter_types,
	traits::{All, GenesisBuild, LockIdentifier},
	weights::{constants::WEIGHT_PER_SECOND, Weight},
	PalletId,
};
use frame_system as system;
use frame_system::EnsureRoot;
use orml_traits::parameter_type_with_key;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, Zero},
};
use xcm::v0::{
	Junction::{self, Parachain, Parent},
	MultiAsset,
	MultiLocation::{self, X1},
	NetworkId, Xcm,
};
use xcm_builder::{
	AccountId32Aliases, AllowUnpaidExecutionFrom, FixedRateOfConcreteFungible, FixedWeightBounds, LocationInverter,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
};
pub use xcm_builder::{
	AllowTopLevelPaidExecutionFrom, EnsureXcmOrigin, NativeAsset, ParentAsSuperuser, ParentIsDefault,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia, TakeWeightCredit,
};
use xcm_executor::{Config, XcmExecutor};
use xcm_simulator::{decl_test_network, decl_test_parachain};

use primitives::traits::MultiAssetRegistry;

use crate as pallet_remote_asset_manager;
use xcm_calls::{
	proxy::{ProxyConfig, ProxyWeights},
	staking::{RewardDestination, StakingConfig, StakingWeights},
};

// import this directly so we can override the relay_ext function and XcmRouter
#[path = "../../../test-utils/xcm-test-support/src/lib.rs"]
mod xcm_test_support;
pub use xcm_test_support::{relay, types::*, Relay};

pub const ALICE: AccountId = AccountId::new([0u8; 32]);
pub const ADMIN_ACCOUNT: AccountId = AccountId::new([1u8; 32]);
pub const EMPTY_ACCOUNT: AccountId = AccountId::new([3u8; 32]);
pub const INITIAL_BALANCE: Balance = 10_000;
pub const PARA_ID: u32 = 1u32;
pub const STATEMINT_PARA_ID: u32 = 200u32;
pub const PARA_ASSET: AssetId = 1;
pub const RELAY_CHAIN_ASSET: AssetId = 42;

decl_test_parachain! {
	pub struct Para {
		Runtime = para::Runtime,
		new_ext = para_ext(PARA_ID, vec![(ALICE, INITIAL_BALANCE)]),
	}
}

// creates a `Statemint` runtime where the PINT parachains sovereign account has
// funds
decl_test_parachain! {
	pub struct Statemint {
		Runtime = statemint::Runtime,
		new_ext = statemint_ext(STATEMINT_PARA_ID, vec![(ALICE, INITIAL_BALANCE),(sibling_sovereign_account(), INITIAL_BALANCE)]),
	}
}

/// Returns the parachain's account on the relay chain
pub fn relay_sovereign_account() -> AccountId {
	let para: ParaId = PARA_ID.into();
	para.into_account()
}

/// Returns the parachain's account on a sibling chain
pub fn sibling_sovereign_account() -> AccountId {
	use xcm_executor::traits::Convert;
	statemint::LocationToAccountId::convert(MultiLocation::X2(Junction::Parent, Junction::Parachain(PARA_ID)))
		.expect("Failed to convert para")
}

decl_test_network! {
	pub struct MockNet {
		relay_chain = Relay,
		parachains = vec![
			(STATEMINT_PARA_ID, Statemint),
			(PARA_ID, Para),
		],
	}
}

pub fn para_ext(parachain_id: u32, balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
	use para::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: parachain_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> { balances }.assimilate_storage(&mut t).unwrap();

	// add xcm transact configs for the native asset of the relay chain
	// NOTE: weights are raw estimates
	pallet_remote_asset_manager::GenesisConfig::<Runtime> {
		staking_configs: vec![(
			RELAY_CHAIN_ASSET,
			StakingConfig {
				pallet_index: relay::STAKING_PALLET_INDEX,
				max_unlocking_chunks: 42,
				pending_unbond_calls: 42,
				reward_destination: RewardDestination::Staked,
				minimum_balance: 0,
				weights: StakingWeights {
					bond: 650_000_000,
					bond_extra: 350_000_000,
					unbond: 1000_u64,
					withdraw_unbonded: 1000_u64,
				},
			},
		)],
		proxy_configs: vec![(
			RELAY_CHAIN_ASSET,
			ProxyConfig {
				pallet_index: relay::PROXY_PALLET_INDEX,
				weights: ProxyWeights { add_proxy: 180_000_000, remove_proxy: 1000_u64 },
			},
		)],
		statemint_config: None,
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn statemint_ext(parachain_id: u32, balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
	use statemint::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: parachain_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> { balances }.assimilate_storage(&mut t).unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	// also fund the parachain's sovereign account on the relay chain
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE), (relay_sovereign_account(), INITIAL_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay::Runtime>;

pub mod para {
	use super::{
		xcm_test_support::calls::{PalletProxyEncoder, PalletStakingEncoder},
		*,
	};
	use crate::mock::xcm_test_support::calls::PalletAssetsEncoder;
	use codec::Decode;
	use frame_support::dispatch::DispatchError;
	use orml_currencies::BasicCurrencyAdapter;
	use orml_xcm_support::{IsNativeConcrete, MultiCurrencyAdapter};
	use pallet_price_feed::{AssetPricePair, Price, PriceFeed};
	use sp_runtime::traits::Convert;

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
	}

	impl frame_system::Config for Runtime {
		type BaseCallFilter = ();
		type BlockWeights = ();
		type BlockLength = ();
		type Origin = Origin;
		type Call = Call;
		type Index = u64;
		type BlockNumber = BlockNumber;
		type Hash = H256;
		type Hashing = sp_runtime::traits::BlakeTwo256;
		type AccountId = AccountId;
		type Lookup = Lookup;
		type Header = Header;
		type Event = Event;
		type BlockHashCount = BlockHashCount;
		type DbWeight = ();
		type Version = ();
		type PalletInfo = PalletInfo;
		type AccountData = pallet_balances::AccountData<Balance>;
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
		type SS58Prefix = ();
		type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	}

	parameter_types! {
		pub const ExistentialDeposit: Balance = 1;
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
		type WeightInfo = ();
		type MaxReserves = MaxReserves;
		type ReserveIdentifier = [u8; 8];
	}

	parameter_types! {
		pub const ReservedXcmpWeight: Weight = WEIGHT_PER_SECOND / 4;
		pub const ReservedDmpWeight: Weight = WEIGHT_PER_SECOND / 4;
	}

	impl cumulus_pallet_parachain_system::Config for Runtime {
		type Event = Event;
		type OnValidationData = ();
		type SelfParaId = ParachainInfo;
		type DmpMessageHandler = DmpQueue;
		type ReservedDmpWeight = ReservedDmpWeight;
		type OutboundXcmpMessageSource = XcmpQueue;
		type XcmpMessageHandler = XcmpQueue;
		type ReservedXcmpWeight = ReservedXcmpWeight;
	}

	impl parachain_info::Config for Runtime {}

	parameter_types! {
		pub const KsmLocation: MultiLocation = MultiLocation::X1(Parent);
		pub const RelayNetwork: NetworkId = NetworkId::Kusama;
		pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
		pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
	}

	pub type LocationToAccountId = (
		ParentIsDefault<AccountId>,
		SiblingParachainConvertsVia<Sibling, AccountId>,
		AccountId32Aliases<RelayNetwork, AccountId>,
	);

	pub type XcmOriginToCallOrigin = (
		SovereignSignedViaLocation<LocationToAccountId, Origin>,
		RelayChainAsNative<RelayChainOrigin, Origin>,
		SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
		SignedAccountId32AsNative<RelayNetwork, Origin>,
		XcmPassthrough<Origin>,
	);

	parameter_types! {
		pub const UnitWeightCost: Weight = 1;
		pub KsmPerSecond: (MultiLocation, u128) = (X1(Parent), 1);
	}

	/// Means for transacting assets on this chain.
	pub type LocalAssetTransactor = MultiCurrencyAdapter<
		// Use this multicurrency for asset balances
		Currency,
		// handle in event of unknown tokens
		UnknownTokens,
		// Convert
		IsNativeConcrete<AssetId, AssetIdConvert>,
		AccountId,
		LocationToAccountId,
		AssetId,
		AssetIdConvert,
	>;

	pub type XcmRouter = super::ParachainXcmRouter<ParachainInfo>;
	pub type Barrier = AllowUnpaidExecutionFrom<All<MultiLocation>>;

	pub struct XcmConfig;
	impl Config for XcmConfig {
		type Call = Call;
		type XcmSender = XcmRouter;
		type AssetTransactor = LocalAssetTransactor;
		type OriginConverter = XcmOriginToCallOrigin;
		type IsReserve = NativeAsset;
		type IsTeleporter = ();
		type LocationInverter = LocationInverter<Ancestry>;
		type Barrier = Barrier;
		type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
		type Trader = FixedRateOfConcreteFungible<KsmPerSecond, ()>;
		type ResponseHandler = ();
	}

	impl cumulus_pallet_xcmp_queue::Config for Runtime {
		type Event = Event;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type ChannelInfo = ParachainSystem;
	}

	impl cumulus_pallet_dmp_queue::Config for Runtime {
		type Event = Event;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	}

	impl cumulus_pallet_xcm::Config for Runtime {
		type Event = Event;
		type XcmExecutor = XcmExecutor<XcmConfig>;
	}

	parameter_type_with_key! {
		pub ExistentialDeposits: |_asset_id: AssetId| -> Balance {
			Zero::zero()
		};
	}

	impl orml_tokens::Config for Runtime {
		type Event = Event;
		type Balance = Balance;
		type Amount = Amount;
		type CurrencyId = AssetId;
		type WeightInfo = ();
		type ExistentialDeposits = ExistentialDeposits;
		type MaxLocks = MaxLocks;
		type OnDust = ();
	}

	impl orml_unknown_tokens::Config for Runtime {
		type Event = Event;
	}

	impl orml_currencies::Config for Runtime {
		type Event = Event;
		type MultiCurrency = Tokens;
		type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
		type GetNativeCurrencyId = PINTAssetId;
		type WeightInfo = ();
	}

	parameter_types! {
		 pub const BaseXcmWeight: Weight = 100_000_000;
	}

	impl orml_xtokens::Config for Runtime {
		type Event = Event;
		type Balance = Balance;
		type CurrencyId = AssetId;
		type CurrencyIdConvert = AssetIdConvert;
		type AccountIdToMultiLocation = xcm_test_support::convert::AccountId32Convert;
		type SelfLocation = SelfLocation;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
		type BaseXcmWeight = BaseXcmWeight;
	}

	parameter_type_with_key! {
		pub MinimumRemoteStashBalance: |_asset_id: AssetId| -> Balance {
			ExistentialDeposit::get()
		};
	}

	parameter_type_with_key! {
		pub CanEncodeAsset: |_asset_id: AssetId| -> bool {
		   true
		};
	}

	parameter_types! {
		pub LockupPeriod: <Runtime as system::Config>::BlockNumber = 0;
		pub MinimumRedemption: u32 = 0;
		pub MinimumStatemintTransferAmount: Balance = 1;
		pub WithdrawalPeriod: <Runtime as system::Config>::BlockNumber = 10;
		pub DOTContributionLimit: Balance = 999;
		pub TreasuryPalletId: PalletId = PalletId(*b"12345678");
		pub IndexTokenLockIdentifier: LockIdentifier = *b"pintlock";
		pub ParaTreasuryAccount: AccountId = TreasuryPalletId::get().into_account();
		pub StatemintCustodian: AccountId = PalletId(*b"pint/smt").into_account();
		pub StringLimit: u32 = 4;

		pub const RelayChainAssetId: AssetId = RELAY_CHAIN_ASSET;
		pub const PINTAssetId: AssetId = PARA_ASSET;
		pub SelfLocation: MultiLocation = MultiLocation::X2(Junction::Parent, Junction::Parachain(ParachainInfo::parachain_id().into()));

		 // No fees for now
		pub const BaseWithdrawalFee: primitives::fee::FeeRate = primitives::fee::FeeRate{ numerator: 0, denominator: 1_000,};
	}

	ord_parameter_types! {
		pub const AdminAccountId: AccountId = ADMIN_ACCOUNT;
	}

	impl pallet_saft_registry::Config for Runtime {
		type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
		type Event = Event;
		type Balance = Balance;
		type AssetRecorder = AssetIndex;
		type AssetId = AssetId;
		type WeightInfo = ();
	}

	impl pallet_asset_index::Config for Runtime {
		type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
		type Event = Event;
		type AssetId = AssetId;
		type SelfAssetId = PINTAssetId;
		type IndexTokenLockIdentifier = IndexTokenLockIdentifier;
		type IndexToken = Balances;
		type Balance = Balance;
		type LockupPeriod = LockupPeriod;
		type MinimumRedemption = MinimumRedemption;
		type WithdrawalPeriod = WithdrawalPeriod;
		type DOTContributionLimit = DOTContributionLimit;
		type RemoteAssetManager = RemoteAssetManager;
		type Currency = Currency;
		type PriceFeed = MockPriceFeed;
		type SaftRegistry = SaftRegistry;
		type TreasuryPalletId = TreasuryPalletId;
		type StringLimit = StringLimit;
		type BaseWithdrawalFee = BaseWithdrawalFee;
		type WeightInfo = ();
	}

	// mock price relay asset
	pub const RELAY_PRICE: Balance = 3;

	pub struct MockPriceFeed;
	impl PriceFeed<AssetId> for MockPriceFeed {
		fn get_price(quote: AssetId) -> Result<Price, DispatchError> {
			let price = match quote {
				RELAY_CHAIN_ASSET => Price::from(RELAY_PRICE),
				_ => return Err(pallet_asset_index::Error::<Runtime>::UnsupportedAsset.into()),
			};
			Ok(price)
		}

		fn get_relative_price_pair(_base: AssetId, _quote: AssetId) -> Result<AssetPricePair<AssetId>, DispatchError> {
			todo!()
		}
	}

	impl pallet_remote_asset_manager::Config for Runtime {
		type Balance = Balance;
		type AssetId = AssetId;
		type AssetIdConvert = AssetIdConvert;
		type AccountId32Convert = xcm_test_support::convert::AccountId32Convert;
		// Encodes `pallet_staking` calls before transaction them to other chains
		type PalletStakingCallEncoder = PalletStakingEncoder<CanEncodeAsset>;
		// Encodes `pallet_proxy` calls before transaction them to other chains
		type PalletProxyCallEncoder = PalletProxyEncoder<CanEncodeAsset>;
		type PalletAssetsCallEncoder = PalletAssetsEncoder<CanEncodeAsset>;
		type StatemintCustodian = StatemintCustodian;
		type MinimumStatemintTransferAmount = MinimumStatemintTransferAmount;
		type SelfAssetId = PINTAssetId;
		type SelfLocation = SelfLocation;
		type SelfParaId = parachain_info::Pallet<Runtime>;
		type RelayChainAssetId = RelayChainAssetId;
		type MinimumRemoteStashBalance = MinimumRemoteStashBalance;
		type Assets = Currency;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type XcmAssetTransfer = XTokens;
		// Using root as the admin origin for now
		type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
		type XcmSender = XcmRouter;
		type Event = Event;
		type AssetRegistry = AssetIndex;
		type WeightInfo = ();
	}

	pub struct AssetIdConvert;
	impl Convert<AssetId, Option<MultiLocation>> for AssetIdConvert {
		fn convert(asset: AssetId) -> Option<MultiLocation> {
			AssetIndex::native_asset_location(&asset)
		}
	}

	impl Convert<MultiLocation, Option<AssetId>> for AssetIdConvert {
		fn convert(location: MultiLocation) -> Option<AssetId> {
			match &location {
				MultiLocation::X1(Junction::Parent) => return Some(RelayChainAssetId::get()),
				MultiLocation::X3(Junction::Parent, Junction::Parachain(id), Junction::GeneralKey(key))
					if ParaId::from(*id) == ParachainInfo::parachain_id().into() =>
				{
					if let Ok(asset_id) = AssetId::decode(&mut &key.clone()[..]) {
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

	pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

	impl pallet_xcm::Config for Runtime {
		type Event = Event;
		type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
		type XcmRouter = XcmRouter;
		type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
		type XcmExecuteFilter = All<(MultiLocation, Xcm<Call>)>;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type XcmTeleportFilter = ();
		type XcmReserveTransferFilter = All<(MultiLocation, Vec<MultiAsset>)>;
		type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	}

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
	type Block = frame_system::mocking::MockBlock<Runtime>;

	construct_runtime!(
		pub enum Runtime where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},

			ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>},
			ParachainInfo: parachain_info::{Pallet, Storage, Config},

			// crate dependencies
			RemoteAssetManager: pallet_remote_asset_manager::{Pallet, Call, Storage, Event<T>, Config<T>},
			Tokens: orml_tokens::{Pallet, Event<T>},
			Currency: orml_currencies::{Pallet, Call, Event<T>},
			XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>},
			UnknownTokens: orml_unknown_tokens::{Pallet, Storage, Event},
			AssetIndex: pallet_asset_index::{Pallet, Call, Storage, Event<T>},
			SaftRegistry: pallet_saft_registry::{Pallet, Call, Storage, Event<T>},

			XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
			DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},
			CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},
			PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
		}
	);
}

pub mod statemint {
	use super::*;
	use frame_support::{
		construct_runtime, parameter_types,
		traits::All,
		weights::{constants::WEIGHT_PER_SECOND, Weight},
	};
	use frame_system::EnsureRoot;
	use sp_core::H256;
	use sp_runtime::{testing::Header, traits::IdentityLookup};

	use pallet_xcm::XcmPassthrough;
	use polkadot_parachain::primitives::Sibling;
	pub use xcm::v0::{
		Junction::{Parachain, Parent},
		MultiAsset,
		MultiLocation::{self, X1, X2, X3},
		NetworkId, Xcm,
	};
	pub use xcm_builder::{
		AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom,
		CurrencyAdapter as XcmCurrencyAdapter, EnsureXcmOrigin, FixedRateOfConcreteFungible, FixedWeightBounds,
		IsConcrete, LocationInverter, NativeAsset, ParentAsSuperuser, ParentIsDefault, RelayChainAsNative,
		SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
		SovereignSignedViaLocation, TakeWeightCredit,
	};
	use xcm_executor::{Config, XcmExecutor};

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
	}

	impl frame_system::Config for Runtime {
		type Origin = Origin;
		type Call = Call;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = ::sp_runtime::traits::BlakeTwo256;
		type AccountId = AccountId;
		type Lookup = IdentityLookup<Self::AccountId>;
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
		type BaseCallFilter = ();
		type SystemWeightInfo = ();
		type SS58Prefix = ();
		type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	}

	parameter_types! {
		pub ExistentialDeposit: Balance = 1;
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
		type WeightInfo = ();
		type MaxReserves = MaxReserves;
		type ReserveIdentifier = [u8; 8];
	}

	parameter_types! {
		pub const ReservedXcmpWeight: Weight = WEIGHT_PER_SECOND / 4;
		pub const ReservedDmpWeight: Weight = WEIGHT_PER_SECOND / 4;
	}

	impl cumulus_pallet_parachain_system::Config for Runtime {
		type Event = Event;
		type OnValidationData = ();
		type SelfParaId = ParachainInfo;
		type DmpMessageHandler = DmpQueue;
		type ReservedDmpWeight = ReservedDmpWeight;
		type OutboundXcmpMessageSource = XcmpQueue;
		type XcmpMessageHandler = XcmpQueue;
		type ReservedXcmpWeight = ReservedXcmpWeight;
	}

	impl parachain_info::Config for Runtime {}

	parameter_types! {
		pub const KsmLocation: MultiLocation = MultiLocation::X1(Parent);
		pub const RelayNetwork: NetworkId = NetworkId::Kusama;
		pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
		pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
	}

	pub type LocationToAccountId = (
		ParentIsDefault<AccountId>,
		SiblingParachainConvertsVia<Sibling, AccountId>,
		AccountId32Aliases<RelayNetwork, AccountId>,
	);

	pub type XcmOriginToCallOrigin = (
		SovereignSignedViaLocation<LocationToAccountId, Origin>,
		RelayChainAsNative<RelayChainOrigin, Origin>,
		SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
		SignedAccountId32AsNative<RelayNetwork, Origin>,
		XcmPassthrough<Origin>,
	);

	parameter_types! {
		pub const UnitWeightCost: Weight = 1;
		pub KsmPerSecond: (MultiLocation, u128) = (X1(Parent), 1);
	}

	pub type LocalAssetTransactor =
		XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, LocationToAccountId, AccountId, ()>;

	pub type XcmRouter = super::ParachainXcmRouter<ParachainInfo>;
	pub type Barrier = AllowUnpaidExecutionFrom<All<MultiLocation>>;

	pub struct XcmConfig;
	impl Config for XcmConfig {
		type Call = Call;
		type XcmSender = XcmRouter;
		type AssetTransactor = LocalAssetTransactor;
		type OriginConverter = XcmOriginToCallOrigin;
		type IsReserve = NativeAsset;
		type IsTeleporter = ();
		type LocationInverter = LocationInverter<Ancestry>;
		type Barrier = Barrier;
		type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
		type Trader = FixedRateOfConcreteFungible<KsmPerSecond, ()>;
		type ResponseHandler = ();
	}

	impl cumulus_pallet_xcmp_queue::Config for Runtime {
		type Event = Event;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type ChannelInfo = ParachainSystem;
	}

	impl cumulus_pallet_dmp_queue::Config for Runtime {
		type Event = Event;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	}

	impl cumulus_pallet_xcm::Config for Runtime {
		type Event = Event;
		type XcmExecutor = XcmExecutor<XcmConfig>;
	}

	pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

	impl pallet_xcm::Config for Runtime {
		type Event = Event;
		type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
		type XcmRouter = XcmRouter;
		type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
		type XcmExecuteFilter = All<(MultiLocation, Xcm<Call>)>;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type XcmTeleportFilter = ();
		type XcmReserveTransferFilter = All<(MultiLocation, Vec<MultiAsset>)>;
		type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	}

	parameter_types! {
		pub const AssetDeposit: Balance = 1_000;
		pub const ApprovalDeposit: Balance = 1;
		pub const AssetsStringLimit: u32 = 50;
		pub const MetadataDepositBase: Balance = 1;
		pub const MetadataDepositPerByte: Balance = 1;
	}

	impl pallet_assets::Config for Runtime {
		type Event = Event;
		type Balance = Balance;
		type AssetId = AssetId;
		type Currency = Balances;
		type ForceOrigin = EnsureRoot<AccountId>;
		type AssetDeposit = AssetDeposit;
		type MetadataDepositBase = MetadataDepositBase;
		type MetadataDepositPerByte = MetadataDepositPerByte;
		type ApprovalDeposit = ApprovalDeposit;
		type StringLimit = AssetsStringLimit;
		type Freezer = ();
		type Extra = ();
		type WeightInfo = ();
	}

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
	type Block = frame_system::mocking::MockBlock<Runtime>;

	construct_runtime!(
		pub enum Runtime where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},

			ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>},
			ParachainInfo: parachain_info::{Pallet, Storage, Config},
			XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
			DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},
			CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},

			PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
			Assets: pallet_assets::{Pallet, Call, Storage, Event<T>} = 50,
		}
	);
}
