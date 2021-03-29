// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_committee;
use frame_support::{
    ord_parameter_types, parameter_types,
    traits::{OnFinalize, OnInitialize},
};
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
        Committee: pallet_committee::{Module, Call, Storage, Origin<T>, Event<T>},
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

pub(crate) const PROPOSAL_SUBMISSION_PERIOD: <Test as system::Config>::BlockNumber = 10;
pub(crate) const VOTING_PERIOD: <Test as system::Config>::BlockNumber = 5;

parameter_types! {
    pub const ProposalSubmissionPeriod: <Test as system::Config>::BlockNumber = PROPOSAL_SUBMISSION_PERIOD;
    pub const VotingPeriod: <Test as system::Config>::BlockNumber = VOTING_PERIOD;
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

pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        // add custom module on_finalize here if implemented
        System::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        // need to explicitly call the committee pallet on_initialize
        Committee::on_initialize(System::block_number());
    }
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    t.into()
}
