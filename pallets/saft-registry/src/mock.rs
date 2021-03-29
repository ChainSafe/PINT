// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_local_treasury;
use pallet_local_treasury::traits::{AssetRecorder, AssetAvailability};
use frame_support::{ord_parameter_types, parameter_types};
use frame_system as system;

use sp_core::H256;
use sp_runtime::{
    DispatchError,
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

pub(crate) const LOCAL_TREASURE_MODULE_ID: ModuleId = ModuleId(*b"12345678");
pub(crate) const ADMIN_ACCOUNT_ID: AccountId = 88;

parameter_types! {
    pub const TestModuleId: ModuleId = LOCAL_TREASURE_MODULE_ID;
}
ord_parameter_types! {
    pub const AdminAccountId: AccountId = ADMIN_ACCOUNT_ID;
}

pub struct TestAssetRecorder();

impl<AssetId, Balance> AssetRecorder<AssetId, Balance> for TestAssetRecorder {
    fn add_asset(_: AssetId, _: Balance, _: AssetAvailability, _: Balance) -> Result<(), DispatchError> { todo!() }
    fn remove_asset(_: AssetId) -> Result<(), DispatchError> { todo!() }
}

impl pallet_local_treasury::Config for Test {
    type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
    type Event = Event;
    type Balance = u32;
    type AssetRecorder = TestAssetRecorder;
    type NAV = u32;
    type AssetId = u32;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    t.into()
}