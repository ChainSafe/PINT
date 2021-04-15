// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_local_treasury;
use frame_support::{ord_parameter_types, parameter_types, traits::StorageMapShim};
use frame_system as system;

use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::AccountIdConversion,
    traits::{BlakeTwo256, IdentityLookup},
    PalletId,
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

pub(crate) type Balance = u64;
pub(crate) type AccountId = u64;

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
    type AccountId = AccountId;
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

pub(crate) const LOCAL_TREASURE_MODULE_ID: PalletId = PalletId(*b"12345678");
pub(crate) const ADMIN_ACCOUNT_ID: AccountId = 88;

parameter_types! {
    pub const TestPalletId: PalletId = LOCAL_TREASURE_MODULE_ID;
}
ord_parameter_types! {
    pub const AdminAccountId: AccountId = ADMIN_ACCOUNT_ID;
}

impl pallet_local_treasury::Config for Test {
    type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
    type PalletId = TestPalletId;
    type Currency = Balances;
    type Event = Event;
}

pub fn local_treasury_account_id() -> AccountId {
    LOCAL_TREASURE_MODULE_ID.into_account()
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        // Assign initial balances to accounts
        balances,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
