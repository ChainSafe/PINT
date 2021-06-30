// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_remote_asset_manager;
use frame_support::{
    dispatch::DispatchResult,
    ord_parameter_types, parameter_types,
    sp_runtime::FixedPointNumber,
    traits::{GenesisBuild, StorageMapShim},
    PalletId,
};
use frame_system as system;
use orml_traits::parameter_type_with_key;
use primitives::traits::MultiAssetRegistry;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, Zero},
    DispatchError,
};
use xcm::v0::{
    Junction::{self, Parachain, Parent},
    MultiAsset,
    MultiLocation::{self, X1, X2, X3},
    NetworkId, Xcm,
};
use xcm_calls::{
    proxy::{ProxyCallEncoder, ProxyType},
    staking::StakingCallEncoder,
    PalletCallEncoder, PassthroughCompactEncoder, PassthroughEncoder,
};

use cumulus_primitives_core::ParaId;
use frame_support::{
    construct_runtime,
    traits::All,
    weights::{constants::WEIGHT_PER_SECOND, Weight},
};
use frame_system::EnsureRoot;
use polkadot_runtime_parachains::{configuration, origin, shared, ump};
use sp_runtime::AccountId32;

use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use xcm_builder::{
    AccountId32Aliases, AllowUnpaidExecutionFrom, ChildParachainAsNative,
    ChildParachainConvertsVia, ChildSystemParachainAsSuperuser,
    CurrencyAdapter as XcmCurrencyAdapter, FixedRateOfConcreteFungible, FixedWeightBounds,
    IsConcrete, LocationInverter, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation,
};
pub use xcm_builder::{
    AllowTopLevelPaidExecutionFrom, EnsureXcmOrigin, NativeAsset, ParentAsSuperuser,
    ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
    TakeWeightCredit,
};
use xcm_executor::{Config, XcmExecutor};
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);


pub mod para {
    use super::*;

    pub type AccountId = AccountId32;
    pub type Balance = u128;

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
        }
    );
}

//
//
//
// type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
// type Block = frame_system::mocking::MockBlock<Test>;
//
// // Configure a mock runtime to test the pallet.
// frame_support::construct_runtime!(
//     pub enum Test where
//         Block = Block,
//         NodeBlock = Block,
//         UncheckedExtrinsic = UncheckedExtrinsic,
//     {
//         System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
//         Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
//         Currency: orml_tokens::{Pallet, Event<T>},
//     }
// );
//
// parameter_types! {
//     pub const BlockHashCount: u64 = 250;
//     pub const SS58Prefix: u8 = 42;
// }
//
// pub(crate) type Balance = u128;
// pub(crate) type Amount = i128;
// pub(crate) type AccountId = u64;
// pub(crate) type AssetId = u32;
// pub(crate) type BlockNumber = u64;
//
// impl system::Config for Test {
//     type BaseCallFilter = ();
//     type BlockWeights = ();
//     type BlockLength = ();
//     type DbWeight = ();
//     type Origin = Origin;
//     type Call = Call;
//     type Index = u64;
//     type BlockNumber = BlockNumber;
//     type Hash = H256;
//     type Hashing = BlakeTwo256;
//     type AccountId = AccountId;
//     type Lookup = IdentityLookup<Self::AccountId>;
//     type Header = Header;
//     type Event = Event;
//     type BlockHashCount = BlockHashCount;
//     type Version = ();
//     type PalletInfo = PalletInfo;
//     type AccountData = ();
//     type OnNewAccount = ();
//     type OnKilledAccount = ();
//     type SystemWeightInfo = ();
//     type SS58Prefix = SS58Prefix;
//     type OnSetCode = ();
// }
//
// pub(crate) const ADMIN_ACCOUNT_ID: AccountId = 88;
//
// ord_parameter_types! {
//     pub const AdminAccountId: AccountId = ADMIN_ACCOUNT_ID;
// }
//
// // param types for balances
// parameter_types! {
//     pub const MaxLocks: u32 = 1024;
//     pub static ExistentialDeposit: Balance = 420;
// }
//
// impl pallet_balances::Config for Test {
//     type Balance = Balance;
//     type DustRemoval = ();
//     type Event = Event;
//     type ExistentialDeposit = ExistentialDeposit;
//     type AccountStore = StorageMapShim<
//         pallet_balances::Account<Test>,
//         system::Provider<Test>,
//         AccountId,
//         pallet_balances::AccountData<Balance>,
//     >;
//     type MaxLocks = MaxLocks;
//     type MaxReserves = ();
//     type ReserveIdentifier = [u8; 8];
//     type WeightInfo = ();
// }
//
// parameter_type_with_key! {
//     pub ExistentialDeposits: |_asset_id: AssetId| -> Balance {
//         Zero::zero()
//     };
// }
//
// impl orml_tokens::Config for Test {
//     type Event = Event;
//     type Balance = Balance;
//     type Amount = Amount;
//     type CurrencyId = AssetId;
//     type WeightInfo = ();
//     type ExistentialDeposits = ExistentialDeposits;
//     type MaxLocks = MaxLocks;
//     type OnDust = ();
// }
//
// parameter_type_with_key! {
//     pub MinimumRemoteStashBalance: |_asset_id: AssetId| -> Balance {
//         ExistentialDeposit::get()
//     };
// }
//
// parameter_types! {
//     pub LockupPeriod: <Test as system::Config>::BlockNumber = 10;
//     pub MinimumRedemption: u32 = 2;
//     pub WithdrawalPeriod: <Test as system::Config>::BlockNumber = 10;
//     pub DOTContributionLimit: Balance = 999;
//     pub TreasuryPalletId: PalletId = PalletId(*b"12345678");
//     pub StringLimit: u32 = 4;
//
//     pub const RelayChainAssetId: AssetId = 0;
//     pub const PINTAssetId: AssetId = 1;
//     pub const SelfParaId: ParaId = ParaId(*b"pint");
//     pub SelfLocation: MultiLocation = MultiLocation::X2(Junction::Parent, Junction::Parachain(SelfParaId::get()));
//
// }
//
//
// impl pallet_remote_asset_manager::Config for Test {
//     type Balance = Balance;
//     type AssetId = AssetId;
//     type AssetIdConvert = AssetIdConvert;
//     type AccountId32Convert = AccountId32Convert;
//     // Encodes `pallet_staking` calls before transaction them to other chains
//     type PalletStakingCallEncoder = PalletStakingEncoder;
//     // Encodes `pallet_proxy` calls before transaction them to other chains
//     type PalletProxyCallEncoder = PalletProxyEncoder;
//     type SelfAssetId = PINTAssetId;
//     type SelfLocation = SelfLocation;
//     type SelfParaId = SelfParaId;
//     type RelayChainAssetId = RelayChainAssetId;
//     type MinimumRemoteStashBalance = MinimumRemoteStashBalance;
//     type Assets = Currencies;
//     type XcmExecutor = XcmExecutor<XcmConfig>;
//     type XcmAssets = xcm_assets::XcmAssetExecutor<XcmAssetConfig>;
//     // Using root as the admin origin for now
//     type AdminOrigin = frame_system::EnsureRoot<AccountId>;
//     type XcmSender = XcmRouter;
//     type Event = Event;
//     type AssetRegistry = MockAssetRegistry;
//     type WeightInfo = ();
// }
//
// struct MockAssetRegistry;
// impl MultiAssetRegistry<AssetId> for MockAssetRegistry {
//     fn native_asset_location(_asset: &AssetId) -> Option<MultiLocation> {
//         None
//     }
//
//     fn is_liquid_asset(_asset: &AssetId) -> bool {
//         true
//     }
// }
//
// /// The encoder to use when transacting `pallet_staking` calls
// pub struct PalletStakingEncoder;
// impl StakingCallEncoder<AccountId, Balance, AccountId> for PalletStakingEncoder {
//     type CompactBalanceEncoder = PassthroughCompactEncoder<Balance, AssetId>;
//     type SourceEncoder = PassthroughEncoder<AccountId, AssetId>;
//     type AccountIdEncoder = PassthroughEncoder<AccountId, AssetId>;
// }
//
// impl PalletCallEncoder for PalletStakingEncoder {
//     type Context = AssetId;
//     fn can_encode(_ctx: &Self::Context) -> bool {
//         true
//     }
// }
//
// pub struct PalletProxyEncoder;
// impl ProxyCallEncoder<AccountId, ProxyType, BlockNumber> for PalletProxyEncoder {
//     type AccountIdEncoder = PassthroughEncoder<AccountId, AssetId>;
//     type ProxyTypeEncoder = PassthroughEncoder<ProxyType, AssetId>;
//     type BlockNumberEncoder = PassthroughEncoder<BlockNumber, AssetId>;
// }
// impl PalletCallEncoder for PalletProxyEncoder {
//     type Context = AssetId;
//     fn can_encode(_ctx: &Self::Context) -> bool {
//         // TODO check in `AssetRegistry`
//         true
//     }
// }
//
//
// pub const PINT_ASSET_ID: AssetId = 0u32;
// pub const ASSET_A_ID: AssetId = 1u32;
// pub const ASSET_B_ID: AssetId = 2u32;
// pub const SAFT_ASSET_ID: AssetId = 99u32;
// pub const UNKNOWN_ASSET_ID: AssetId = 3u32;
//
// pub struct ExtBuilder {
//     balances: Vec<(AccountId, AssetId, Balance)>,
// }
//
// pub(crate) const ASHLEY: AccountId = 0;
//
// // Returns default values for genesis config
// impl Default for ExtBuilder {
//     fn default() -> Self {
//         Self {
//             balances: vec![
//                 (ADMIN_ACCOUNT_ID, ASSET_A_ID, 1000_000_000_000_000u128),
//                 (ADMIN_ACCOUNT_ID, ASSET_B_ID, 1000_000_000_000_000u128),
//                 (ADMIN_ACCOUNT_ID, SAFT_ASSET_ID, 1000_000_000_000_000u128),
//             ],
//         }
//     }
// }
//
// impl ExtBuilder {
//     // builds genesis config
//
//     pub fn with_balances(mut self, balances: Vec<(AccountId, AssetId, Balance)>) -> Self {
//         self.balances = balances;
//         self
//     }
//
//     pub fn build(self) -> sp_io::TestExternalities {
//         let mut t = frame_system::GenesisConfig::default()
//             .build_storage::<Test>()
//             .unwrap();
//
//         orml_tokens::GenesisConfig::<Test> {
//             balances: self.balances,
//         }
//         .assimilate_storage(&mut t)
//         .unwrap();
//
//         t.into()
//     }
// }


decl_test_parachain! {
    pub struct ParaA {
        Runtime = para::Runtime,
        new_ext = para_ext(1),
    }
}

decl_test_parachain! {
    pub struct ParaB {
        Runtime = para::Runtime,
        new_ext = para_ext(2),
    }
}

decl_test_relay_chain! {
    pub struct Relay {
        Runtime = relay::Runtime,
        XcmConfig = relay::XcmConfig,
        new_ext = relay_ext(),
    }
}

decl_test_network! {
    pub struct MockNet {
        relay_chain = Relay,
        parachains = vec![
            (1, ParaA),
            (2, ParaB),
        ],
    }
}

pub const INITIAL_BALANCE: u128 = 1_000_000_000;

pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
    use para::{Runtime, System};

    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let parachain_info_config = parachain_info::GenesisConfig {
        parachain_id: para_id.into(),
    };

    <parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
        &parachain_info_config,
        &mut t,
    )
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(ALICE, INITIAL_BALANCE)],
    }
        .assimilate_storage(&mut t)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
    use relay::{Runtime, System};

    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(ALICE, INITIAL_BALANCE)],
    }
        .assimilate_storage(&mut t)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub type RelayChainPalletXcm = pallet_xcm::Pallet<xcm_test_support::relay::Runtime>;
pub type ParachainPalletXcm = pallet_xcm::Pallet<para::Runtime>;