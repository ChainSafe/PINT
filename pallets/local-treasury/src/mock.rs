// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_local_treasury;
use frame_support::{parameter_types, traits::StorageMapShim};
use frame_system as system;
use frame_system::{EnsureSignedBy};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    ModuleId,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        LocalTreasury: pallet_local_treasury::{Module, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

pub(crate) type Balance = u64;

// param types for balances
parameter_types! {
    pub const MaxLocks: u32 = 1024;
    pub static ExistentialDeposit: Balance = 0;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Test>,
        system::Provider<Test>,
        Balance,
        pallet_balances::AccountData<Balance>,
    >;
    type MaxLocks = MaxLocks;
    type WeightInfo = ();
}

parameter_types! {
    pub const TestModuleId: ModuleId = ModuleId(*b"fomofomo");
}

impl pallet_local_treasury::Config for Test {
    type AdminOrigin = EnsureOrigin<Origin>;
    type ModuleId = TestModuleId;
    type Currency = Balances;
    type Event = Event;
}

pub(crate) const INITIAL_BALANCE: Balance = 100;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        // Assign initial balances to accounts
        balances: vec![
            (0, INITIAL_BALANCE),
            (1, INITIAL_BALANCE),
            (2, INITIAL_BALANCE),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
