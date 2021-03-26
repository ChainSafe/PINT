// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_committee;
use frame_support::{ord_parameter_types, parameter_types};
use frame_system as system;

use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
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
        Committee: pallet_committee::{Module, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

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

parameter_types! {
    pub const ProposalSubmissionPeriod: <Test as system::Config>::BlockNumber = 10;
    pub const VotingPeriod: <Test as system::Config>::BlockNumber = 10;
}
pub(crate) const PROPOSER_ACCOUNT_ID: AccountId = 88;
ord_parameter_types! {
    pub const AdminAccountId: AccountId = PROPOSER_ACCOUNT_ID;
}

impl pallet_committee::Config for Test {
    type ProposalSubmissionPeriod = ProposalSubmissionPeriod;
    type VotingPeriod = VotingPeriod;
    type ProposalSubmissionOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
    type ProposalExecutionOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
    type ProposalNonce = u32;
    type Origin = Origin;
    type Action = Call;
    type Event = Event;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    t.into()
}