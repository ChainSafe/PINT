// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#[cfg(feature = "runtime-benchmarks")]
use frame_support::pallet_prelude::DispatchResultWithPostInfo;
#[cfg(feature = "runtime-benchmarks")]
use pallet_price_feed::PriceFeedBenchmarks;

use super::{types::*, ADMIN_ACCOUNT, PARA_ASSET, RELAY_CHAIN_ASSET};
use frame_support::{
	construct_runtime,
	dispatch::DispatchError,
	ord_parameter_types, parameter_types,
	sp_runtime::traits::{AccountIdConversion, Zero},
	sp_std::marker::PhantomData,
	traits::{Everything, Get, LockIdentifier, Nothing},
	weights::{constants::WEIGHT_PER_SECOND, Weight},
	PalletId,
};
use frame_system as system;
use frame_system::EnsureRoot;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;
use orml_xcm_support::{IsNativeConcrete, MultiCurrencyAdapter};
use pallet_price_feed::{AssetPricePair, Price, PriceFeed};
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use shot_runtime::{AccountId32Convert, AssetIdConvert};
use sp_core::H256;
use xcm::v1::{Junction, Junctions, MultiLocation, NetworkId};
use xcm_builder::{
	AccountId32Aliases, AllowUnpaidExecutionFrom, EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds,
	LocationInverter, NativeAsset, ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
};
use xcm_executor::XcmExecutor;

/// Support for call encoders
pub mod calls {
	use frame_support::sp_std::marker::PhantomData;
	use orml_traits::{parameter_type_with_key, GetByKey};

	use xcm_calls::{
		proxy::{ProxyCallEncoder, ProxyType},
		staking::StakingCallEncoder,
		PalletCallEncoder, PassthroughCompactEncoder, PassthroughEncoder,
	};

	use crate::types::*;

	// A type that states that all calls to the asset's native location can be
	// encoded
	parameter_type_with_key! {
		pub CanEncodeAll: |_asset_id: AssetId| -> bool {
		   true
		};
	}

	/// The encoder to use when transacting `pallet_proxy` calls
	pub struct PalletProxyEncoder<T>(PhantomData<T>);
	impl<T: GetByKey<AssetId, bool>> ProxyCallEncoder<AccountId, ProxyType, BlockNumber> for PalletProxyEncoder<T> {
		type AccountIdEncoder = PassthroughEncoder<AccountId, AssetId>;
		type ProxyTypeEncoder = PassthroughEncoder<ProxyType, AssetId>;
		type BlockNumberEncoder = PassthroughEncoder<BlockNumber, AssetId>;
	}

	impl<T: GetByKey<AssetId, bool>> PalletCallEncoder for PalletProxyEncoder<T> {
		type Context = AssetId;
		fn can_encode(ctx: &Self::Context) -> bool {
			T::get(ctx)
		}
	}

	/// The encoder to use when transacting `pallet_staking` calls
	pub struct PalletStakingEncoder<T>(PhantomData<T>);
	impl<T: GetByKey<AssetId, bool>> StakingCallEncoder<AccountLookupSource, Balance, AccountId>
		for PalletStakingEncoder<T>
	{
		type CompactBalanceEncoder = PassthroughCompactEncoder<Balance, AssetId>;
		type SourceEncoder = PassthroughEncoder<AccountLookupSource, AssetId>;
		type AccountIdEncoder = PassthroughEncoder<AccountId, AssetId>;
	}

	impl<T: GetByKey<AssetId, bool>> PalletCallEncoder for PalletStakingEncoder<T> {
		type Context = AssetId;
		fn can_encode(ctx: &Self::Context) -> bool {
			T::get(ctx)
		}
	}
}

parameter_types! {
	pub const BlockHashCount: u32 = 250;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;
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
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Junction::Parachain(
		ParachainInfo::parachain_id().into()
	).into();
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
	pub const MaxInstructions: u32 = 100;
	pub KsmPerSecond: (xcm::v1::AssetId, u128) = (xcm::v1::AssetId::Concrete(KsmLocation::get()), 1);
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

pub type XcmRouter = crate::ParachainXcmRouter<ParachainInfo>;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = PolkadotXcm;
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
	type DustRemovalWhitelist = Everything;
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
	type AccountIdToMultiLocation = AccountId32Convert;
	type SelfLocation = SelfLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type LocationInverter = LocationInverter<Ancestry>;
}

parameter_type_with_key! {
	pub MinimumReserve: |_asset_id: AssetId| -> Balance {
		ExistentialDeposit::get()
	};
}

parameter_type_with_key! {
	pub MinimumBondExtra: |_asset_id: AssetId| -> Balance {
		1_000
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
	pub TreasuryPalletId: PalletId = PalletId(*b"12345678");
	pub IndexTokenLockIdentifier: LockIdentifier = *b"pintlock";
	pub ParaTreasuryAccount: AccountId = TreasuryPalletId::get().into_account();
	pub StringLimit: u32 = 4;

	pub const RelayChainAssetId: AssetId = RELAY_CHAIN_ASSET;
	pub const PINTAssetId: AssetId = PARA_ASSET;
	pub SelfLocation: MultiLocation = MultiLocation::new(1, Junctions::X1(Junction::Parachain(ParachainInfo::get().into())));

	 // No fees for now
	pub const BaseWithdrawalFee: primitives::fee::FeeRate = primitives::fee::FeeRate{ numerator: 0, denominator: 1_000,};
}

ord_parameter_types! {
	pub const AdminAccountId: AccountId = ADMIN_ACCOUNT;
}

impl pallet_saft_registry::Config for Runtime {
	#[cfg(feature = "runtime-benchmarks")]
	type AssetRecorderBenchmarks = AssetIndex;
	type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
	type Event = Event;
	type Balance = Balance;
	type AssetRecorder = AssetIndex;
	type AssetId = AssetId;
	type WeightInfo = ();
}

parameter_types! {
	pub MaxDecimals: u8 = 12;
	pub MaxActiveDeposits: u32 = 50;
}

/// Range of voting period
pub struct LockupPeriodRange<T>(PhantomData<T>);

impl<T: frame_system::Config> pallet_asset_index::traits::LockupPeriodRange<T::BlockNumber> for LockupPeriodRange<T> {
	fn min() -> T::BlockNumber {
		10u32.into()
	}

	fn max() -> T::BlockNumber {
		70u32.into()
	}
}

impl pallet_asset_index::Config for Runtime {
	type AdminOrigin = frame_system::EnsureSigned<AccountId>;
	type Event = Event;
	type AssetId = AssetId;
	type SelfAssetId = PINTAssetId;
	type IndexTokenLockIdentifier = IndexTokenLockIdentifier;
	type IndexToken = Balances;
	type Balance = Balance;
	type MaxActiveDeposits = MaxActiveDeposits;
	type MaxDecimals = MaxDecimals;
	type RedemptionFee = ();
	type LockupPeriod = LockupPeriod;
	type LockupPeriodRange = LockupPeriodRange<Self>;
	type MinimumRedemption = MinimumRedemption;
	type WithdrawalPeriod = WithdrawalPeriod;
	type RemoteAssetManager = RemoteAssetManager;
	type Currency = Currency;
	type PriceFeed = MockPriceFeed;
	#[cfg(feature = "runtime-benchmarks")]
	type PriceFeedBenchmarks = MockPriceFeed;
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

#[cfg(feature = "runtime-benchmarks")]
impl PriceFeedBenchmarks<AccountId, AssetId> for MockPriceFeed {
	fn create_feed(_caller: AccountId, _asset_id: AssetId) -> DispatchResultWithPostInfo {
		Ok(().into())
	}
}

impl pallet_remote_asset_manager::Config for Runtime {
	type Balance = Balance;
	type AssetId = AssetId;
	type AssetIdConvert = AssetIdConvert;
	// Encodes `pallet_staking` calls before transaction them to other chains
	type PalletStakingCallEncoder = shot_runtime::PalletStakingEncoder;
	// Encodes `pallet_proxy` calls before transaction them to other chains
	type PalletProxyCallEncoder = shot_runtime::PalletProxyEncoder;
	type MinimumStatemintTransferAmount = MinimumStatemintTransferAmount;
	type SelfAssetId = PINTAssetId;
	type SelfLocation = SelfLocation;
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type RelayChainAssetId = RelayChainAssetId;
	type AssetStakingCap = (MinimumReserve, MinimumBondExtra);
	type Assets = Currency;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmAssetTransfer = XTokens;
	// Using root as the admin origin for now
	type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
	type XcmSender = XcmRouter;
	type Event = Event;
	type WeightInfo = ();
}

pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type LocationInverter = LocationInverter<Ancestry>;
	type Origin = Origin;
	type Call = Call;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
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
		PolkadotXcm: pallet_xcm::{Pallet, Storage, Call, Event<T>, Origin, Config},
	}
);
