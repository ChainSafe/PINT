// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

#[cfg(feature = "runtime-benchmarks")]
use frame_support::pallet_prelude::DispatchResultWithPostInfo;
#[cfg(feature = "runtime-benchmarks")]
use pallet_price_feed::PriceFeedBenchmarks;

use crate as pallet_saft_registry;
use core::cell::RefCell;
use frame_support::{
	assert_ok, ord_parameter_types, parameter_types,
	traits::{LockIdentifier, StorageMapShim},
	PalletId,
};
use frame_system as system;
use orml_traits::parameter_type_with_key;
use pallet_price_feed::{AssetPricePair, Price, PriceFeed};
use xcm::v0::MultiLocation;

use frame_support::traits::Everything;
use primitives::AssetAvailability;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, Zero},
	DispatchError,
};
use std::collections::HashMap;

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
		AssetIndex: pallet_asset_index::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Currency: orml_tokens::{Pallet, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

pub(crate) type Balance = u128;
pub(crate) type Amount = i128;
pub(crate) type AccountId = u64;
pub(crate) type AssetId = u32;
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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
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
		AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

parameter_types! {
	pub LockupPeriod: <Test as system::Config>::BlockNumber = 0;
	pub MinimumRedemption: u32 = 2;
	pub WithdrawalPeriod: <Test as system::Config>::BlockNumber = 10;
	pub MaxActiveDeposits: u32 = 50;
	pub TreasuryPalletId: PalletId = PalletId(*b"12345678");
	pub IndexTokenLockIdentifier: LockIdentifier = *b"pintlock";
	pub StringLimit: u32 = 4;
	pub const PINTAssetId: AssetId = 99;

	// No fees for now
	pub const BaseWithdrawalFee: primitives::fee::FeeRate = primitives::fee::FeeRate{ numerator: 0, denominator: 1_000,};
}

impl pallet_asset_index::Config for Test {
	type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
	type IndexToken = Balances;
	type Balance = Balance;
	type MaxActiveDeposits = MaxActiveDeposits;
	type RedemptionFee = ();
	type LockupPeriod = LockupPeriod;
	type IndexTokenLockIdentifier = IndexTokenLockIdentifier;
	type MinimumRedemption = MinimumRedemption;
	type WithdrawalPeriod = WithdrawalPeriod;
	type RemoteAssetManager = ();
	type AssetId = AssetId;
	type SelfAssetId = PINTAssetId;
	type Currency = Currency;
	type PriceFeed = MockPriceFeed;
	#[cfg(feature = "runtime-benchmarks")]
	type PriceFeedBenchmarks = MockPriceFeed;
	type SaftRegistry = SaftRegistry;
	type BaseWithdrawalFee = BaseWithdrawalFee;
	type TreasuryPalletId = TreasuryPalletId;
	type Event = Event;
	type StringLimit = StringLimit;
	type WeightInfo = ();
}

pub const LIQUID_ASSET_ID: AssetId = 3u32;
pub const SAFT_ASSET_ID: AssetId = 43u32;
pub const LIQUID_ASSET_MULTIPLIER: Balance = 2;
pub const SAFT_ASSET_MULTIPLIER: Balance = 3;

thread_local! {
	pub static PRICES: RefCell<HashMap<AssetId, Price>> = RefCell::new(HashMap::new());
}

pub struct MockPriceFeed;
impl MockPriceFeed {
	pub fn set_prices(prices: impl IntoIterator<Item = (AssetId, Price)>) {
		PRICES.with(|v| *v.borrow_mut() = prices.into_iter().collect());
	}
}

impl PriceFeed<AssetId> for MockPriceFeed {
	fn get_price(asset: AssetId) -> Result<Price, DispatchError> {
		PRICES.with(|v| {
			v.borrow().get(&asset).cloned().ok_or_else(|| pallet_asset_index::Error::<Test>::UnsupportedAsset.into())
		})
	}

	fn get_relative_price_pair(_base: AssetId, _quote: AssetId) -> Result<AssetPricePair<AssetId>, DispatchError> {
		todo!()
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl PriceFeedBenchmarks<AccountId, AssetId> for MockPriceFeed {
	fn create_feed(_caller: AccountId, _asset_id: AssetId) -> DispatchResultWithPostInfo {
		Ok(().into())
	}
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_asset_id: AssetId| -> Balance {
		Zero::zero()
	};
}

impl orml_tokens::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = AssetId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type OnDust = ();
	type DustRemovalWhitelist = Everything;
}
pub(crate) const ADMIN_ACCOUNT_ID: AccountId = 1337;

ord_parameter_types! {
	pub const AdminAccountId: AccountId = ADMIN_ACCOUNT_ID;
}

impl pallet_saft_registry::Config for Test {
	#[cfg(feature = "runtime-benchmarks")]
	type AssetRecorderBenchmarks = AssetIndex;
	type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
	type Event = Event;
	type Balance = Balance;
	type AssetRecorder = AssetIndex;
	type AssetId = AssetId;
	type WeightInfo = ();
}

pub const INDEX_TOKEN_SUPPLY: u128 = 2_0000;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t: sp_io::TestExternalities =
		frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();

	t.execute_with(|| {
		// mint and intial supply of pint
		let initial_liquid_supply = 1_000;
		assert_ok!(AssetIndex::register_asset(
			Origin::signed(ADMIN_ACCOUNT_ID),
			LIQUID_ASSET_ID,
			AssetAvailability::Liquid(MultiLocation::Null)
		));
		assert_ok!(AssetIndex::add_asset(
			Origin::signed(ADMIN_ACCOUNT_ID),
			LIQUID_ASSET_ID,
			initial_liquid_supply,
			INDEX_TOKEN_SUPPLY,
		));

		// set initial prices
		MockPriceFeed::set_prices(vec![
			(LIQUID_ASSET_ID, Price::from(LIQUID_ASSET_MULTIPLIER)),
			(SAFT_ASSET_ID, Price::from(SAFT_ASSET_MULTIPLIER)),
		]);
	});

	t
}
