// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_price_feed;
use frame_support::{parameter_types, PalletId};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
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
        PriceFeed: pallet_price_feed::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        ChainlinkFeed: pallet_chainlink_feed::{Pallet, Call, Storage, Event<T>},
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
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

pub(crate) const MIN_RESERVE: u64 = 100;

parameter_types! {
    pub const FeedPalletId: PalletId = PalletId(*b"linkfeed");
    pub const MinimumReserve: u64 = MIN_RESERVE;
    pub const StringLimit: u32 = 15;
    pub const OracleLimit: u32 = 10;
    pub const FeedLimit: u16 = 10;
    pub const PruningWindow: u32 = 3;
}

type FeedId = u16;
type Value = u64;

impl pallet_chainlink_feed::Config for Test {
    type Event = Event;
    type FeedId = FeedId;
    type Value = Value;
    type Currency = Balances;
    type PalletId = FeedPalletId;
    type MinimumReserve = MinimumReserve;
    type StringLimit = StringLimit;
    type OracleCountLimit = OracleLimit;
    type FeedLimit = FeedLimit;
    type PruningWindow = PruningWindow;
    type WeightInfo = ();
}

parameter_types! {
    pub const PINTFeed: FeedId = 1u16;
}

type BaseCurrency = u128;

impl pallet_price_feed::Config for Test {
    type Event = Event;
    type Oracle = ChainlinkFeed;
    type BaseCurrency = BaseCurrency;
    type SelfAssetFeedId = PINTFeed;
    type Precision = Perbill;
    type AssetFeedId = FeedId;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    t.into()
}
