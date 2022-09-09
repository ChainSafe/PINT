// // Copyright 2021 ChainSafe Systems
// // SPDX-License-Identifier: LGPL-3.0-only
//
// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_price_feed;
use frame_support::{
	ord_parameter_types, parameter_types,
	traits::{Everything, ConstU32, SortedMembers}
};
use frame_system::EnsureSignedBy;
use frame_system as system;
// use pallet_chainlink_feed::RoundId;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use primitives::Price;
//
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
//
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
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		OrmlOracle: orml_oracle::{Pallet, Call, Event<T>, Storage},
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
	type BaseCallFilter = Everything;
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
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1000;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

pub(crate) const MIN_RESERVE: u64 = 100;

pub(crate) type AssetId = u32;
pub(crate) const ADMIN_ACCOUNT_ID: AccountId = 88;

parameter_types! {
	pub const PINTAssetId: AssetId = 1u32;
}

ord_parameter_types! {
	pub const AdminAccountId: AccountId = ADMIN_ACCOUNT_ID;
}

impl pallet_price_feed::Config for Test {
	type AdminOrigin = EnsureSignedBy<AdminAccountId, AccountId>;
	type SelfAssetId = PINTAssetId;
	type AssetId = AssetId;
	type Time = Timestamp;
	type Event = Event;
	type WeightInfo = ();
	type DataProvider = OrmlOracle;
}

parameter_types! {
	pub const MinimumCount: u32 = 1;
	pub const ExpiresIn: u64 = 1000 * 60 * 60; // 1 hours
	pub static OracleMembers: Vec<AccountId> = vec![1, 2, 3];
}

pub struct Members;

impl SortedMembers<AccountId> for Members {
	fn sorted_members() -> Vec<AccountId> {
		OracleMembers::get()
	}
}

impl orml_oracle::Config for Test {
	type Event = Event;
	type OnNewData = ();
	type CombineData = orml_oracle::DefaultCombineData<Test, MinimumCount, ExpiresIn>;
	type Time = Timestamp;
	type OracleKey = AssetId;
	type OracleValue = Price;
	type RootOperatorAccountId = AdminAccountId;
	type Members = Members;
	type WeightInfo = ();
	type MaxHasDispatchedSize = ConstU32<40>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	pallet_balances::GenesisConfig::<Test> { balances: vec![(ADMIN_ACCOUNT_ID, 100 * MIN_RESERVE)] }
		.assimilate_storage(&mut t)
		.unwrap();

	t.into()
}
