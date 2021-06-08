// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_saft_registry;
use frame_support::{ord_parameter_types, parameter_types};
use frame_system as system;
use pallet_asset_index::traits::{AssetAvailability, AssetRecorder};

use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    DispatchError,
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
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        SaftRegistry: pallet_saft_registry::{Pallet, Call, Storage, Event<T>},
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
    type OnSetCode = ();
}

pub(crate) const ADMIN_ACCOUNT_ID: AccountId = 88;

ord_parameter_types! {
    pub const AdminAccountId: AccountId = ADMIN_ACCOUNT_ID;
}

pub struct MockAssetRecorder();

impl<AssetId, Balance> AssetRecorder<AssetId, Balance> for MockAssetRecorder {
    fn add_asset(_: &AssetId, _: &Balance, _: &AssetAvailability) -> Result<(), DispatchError> {
        Ok(())
    }
    fn remove_asset(_: &AssetId) -> Result<(), DispatchError> {
        Ok(())
    }
}

impl pallet_saft_registry::Config for Test {
    type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
    type Event = Event;
    type Balance = Balance;
    type AssetRecorder = MockAssetRecorder;
    type AssetId = u32;
    type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    t.into()
}
