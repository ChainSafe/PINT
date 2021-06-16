// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_price_feed;
use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_support::{ord_parameter_types, parameter_types, PalletId};
use frame_system as system;
use pallet_chainlink_feed::RoundId;
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
pub(crate) type BlockNumber = u64;

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
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

pub(crate) type FeedId = u16;
pub(crate) type Value = u128;
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
    type OnAnswerHandler = ();
    type WeightInfo = pallet_chainlink_feed::default_weights::WeightInfo<Test>;
}

pub(crate) type AssetId = u64;
pub(crate) const ADMIN_ACCOUNT_ID: AccountId = 88;

parameter_types! {
    pub const PINTAssetId: AssetId = 1u64;
}

ord_parameter_types! {
    pub const AdminAccountId: AccountId = ADMIN_ACCOUNT_ID;
}

impl pallet_price_feed::Config for Test {
    type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
    type SelfAssetId = PINTAssetId;
    type AssetId = AssetId;
    type Oracle = ChainlinkFeed;
    type Event = Event;
    type WeightInfo = ();
}

#[derive(Debug, Clone, Default)]
pub struct FeedBuilder {
    owner: Option<AccountId>,
    payment: Option<Balance>,
    timeout: Option<BlockNumber>,
    value_bounds: Option<(Value, Value)>,
    min_submissions: Option<u32>,
    description: Option<Vec<u8>>,
    restart_delay: Option<RoundId>,
    oracles: Option<Vec<(AccountId, AccountId)>>,
}

/// Helper implementation
impl FeedBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn owner(mut self, o: AccountId) -> Self {
        self.owner = Some(o);
        self
    }

    pub fn payment(mut self, p: Balance) -> Self {
        self.payment = Some(p);
        self
    }

    pub fn timeout(mut self, t: BlockNumber) -> Self {
        self.timeout = Some(t);
        self
    }

    pub fn value_bounds(mut self, min: Value, max: Value) -> Self {
        self.value_bounds = Some((min, max));
        self
    }

    pub fn min_submissions(mut self, m: u32) -> Self {
        self.min_submissions = Some(m);
        self
    }

    pub fn description(mut self, d: Vec<u8>) -> Self {
        self.description = Some(d);
        self
    }

    pub fn restart_delay(mut self, d: RoundId) -> Self {
        self.restart_delay = Some(d);
        self
    }

    pub fn oracles(mut self, o: Vec<(AccountId, AccountId)>) -> Self {
        self.oracles = Some(o);
        self
    }

    pub fn build_and_store(self) -> DispatchResultWithPostInfo {
        let owner = Origin::signed(self.owner.unwrap_or(1));
        let payment = self.payment.unwrap_or(20);
        let timeout = self.timeout.unwrap_or(1);
        let value_bounds = self.value_bounds.unwrap_or((1, 1_000));
        let min_submissions = self.min_submissions.unwrap_or(2);
        let decimals = 5;
        let description = self.description.unwrap_or(b"desc".to_vec());
        let oracles = self.oracles.unwrap_or(vec![(2, 4), (3, 4), (4, 4)]);
        let restart_delay = self
            .restart_delay
            .unwrap_or(oracles.len().saturating_sub(1) as u32);
        ChainlinkFeed::create_feed(
            owner,
            payment,
            timeout,
            value_bounds,
            min_submissions,
            decimals,
            description,
            restart_delay,
            oracles,
            None,
            None,
        )
    }
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ADMIN_ACCOUNT_ID, 100 * MIN_RESERVE)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_chainlink_feed::GenesisConfig::<Test> {
        pallet_admin: Some(ADMIN_ACCOUNT_ID),
        feed_creators: vec![1],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
