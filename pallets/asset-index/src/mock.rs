// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_asset_index;
use frame_support::{
	ord_parameter_types, parameter_types,
	traits::{GenesisBuild, LockIdentifier, StorageMapShim},
	PalletId,
};
use frame_system as system;
use orml_traits::parameter_type_with_key;
use pallet_price_feed::PriceFeed;
use primitives::{fee::FeeRate, traits::RemoteAssetManager, AssetPricePair, Price};
use sp_core::H256;
use sp_std::cell::RefCell;
use std::collections::HashMap;

use rand::{thread_rng, Rng};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, Zero},
	DispatchError,
};
use std::ops::Range;
use xcm::v0::Outcome;

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
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
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
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
}

pub(crate) const ADMIN_ACCOUNT_ID: AccountId = 0;
pub(crate) const ASHLEY: AccountId = 1;

ord_parameter_types! {
	pub const AdminAccountId: AccountId = ADMIN_ACCOUNT_ID;
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
	type WeightInfo = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
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
	type OnDust = ();
	type MaxLocks = MaxLocks;
}

impl pallet_saft_registry::Config for Test {
	type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
	type Event = Event;
	type Balance = Balance;
	type AssetRecorder = AssetIndex;
	type AssetId = AssetId;
	type WeightInfo = ();
}

parameter_types! {
	pub LockupPeriod: <Test as system::Config>::BlockNumber = 10;
	pub MinimumRedemption: u32 = 3;
	pub WithdrawalPeriod: <Test as system::Config>::BlockNumber = 10;
	pub DOTContributionLimit: Balance = 999;
	pub TreasuryPalletId: PalletId = PalletId(*b"12345678");
	pub IndexTokenLockIdentifier: LockIdentifier = *b"pintlock";
	pub StringLimit: u32 = 4;
	pub DepositLimit: u32 = 50;
	pub const PINTAssetId: AssetId = PINT_ASSET_ID;

	// No fees for now
	pub const BaseWithdrawalFee: FeeRate = FeeRate{ numerator: 0, denominator: 1_000,};
}

pub struct RedemptionFee;

impl primitives::traits::RedemptionFee<BlockNumber, Balance> for RedemptionFee {}

impl pallet_asset_index::Config for Test {
	type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
	type IndexToken = Balances;
	type Balance = Balance;
	type DepositLimit = DepositLimit;
	type RedemptionFee = RedemptionFee;
	type LockupPeriod = LockupPeriod;
	type IndexTokenLockIdentifier = IndexTokenLockIdentifier;
	type MinimumRedemption = MinimumRedemption;
	type WithdrawalPeriod = WithdrawalPeriod;
	type DOTContributionLimit = DOTContributionLimit;
	type RemoteAssetManager = MockRemoteAssetManager;
	type AssetId = AssetId;
	type SelfAssetId = PINTAssetId;
	type Currency = Currency;
	type PriceFeed = MockPriceFeed;
	type SaftRegistry = SaftRegistry;
	type BaseWithdrawalFee = BaseWithdrawalFee;
	type TreasuryPalletId = TreasuryPalletId;
	type Event = Event;
	type StringLimit = StringLimit;
	type WeightInfo = ();
}

pub struct MockRemoteAssetManager;
impl<AccountId, AssetId, Balance> RemoteAssetManager<AccountId, AssetId, Balance> for MockRemoteAssetManager {
	fn transfer_asset(_: AccountId, _: AssetId, _: Balance) -> Result<Outcome, DispatchError> {
		Ok(Outcome::Complete(0))
	}

	fn deposit(_: AssetId, _: Balance) {}

	fn announce_withdrawal(_: AssetId, _: Balance) {}
}

pub const PINT_ASSET_ID: AssetId = 0u32;
pub const ASSET_A_ID: AssetId = 1u32;
pub const ASSET_B_ID: AssetId = 2u32;
pub const SAFT_ASSET_ID: AssetId = 99u32;
pub const UNKNOWN_ASSET_ID: AssetId = 3u32;

pub const ASSET_A_PRICE_MULTIPLIER: Balance = 2;
pub const ASSET_B_PRICE_MULTIPLIER: Balance = 3;

thread_local! {
	pub static PRICES: RefCell<HashMap<AssetId, Price>> = RefCell::new(HashMap::new());
}

pub struct MockPriceFeed;

impl MockPriceFeed {
	pub fn set_prices(prices: impl IntoIterator<Item = (AssetId, Price)>) {
		PRICES.with(|v| *v.borrow_mut() = prices.into_iter().collect());
	}

	/// Use some random prices for the given assets
	pub fn set_random_prices(assets: impl IntoIterator<Item = AssetId>, range: Range<u128>) {
		let mut rng = thread_rng();
		Self::set_prices(assets.into_iter().map(|asset| (asset, Price::from(rng.gen_range(range.clone())))))
	}
}

impl PriceFeed<AssetId> for MockPriceFeed {
	// mock price supposed to return the price pair with the same `quote` price, like USD
	fn get_price(asset: AssetId) -> Result<Price, DispatchError> {
		PRICES.with(|v| {
			v.borrow().get(&asset).cloned().ok_or_else(|| pallet_asset_index::Error::<Test>::UnsupportedAsset.into())
		})
	}

	fn get_relative_price_pair(_base: AssetId, _quote: AssetId) -> Result<AssetPricePair<AssetId>, DispatchError> {
		todo!()
	}
}

pub struct ExtBuilder {
	balances: Vec<(AccountId, AssetId, Balance)>,
}

// Returns default values for genesis config
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			balances: vec![
				(ADMIN_ACCOUNT_ID, ASSET_A_ID, 1_000_000_000_000_000_u128),
				(ADMIN_ACCOUNT_ID, ASSET_B_ID, 1_000_000_000_000_000_u128),
				(ADMIN_ACCOUNT_ID, SAFT_ASSET_ID, 1_000_000_000_000_000_u128),
			],
		}
	}
}

impl ExtBuilder {
	// builds genesis config

	pub fn with_balances(mut self, balances: Vec<(AccountId, AssetId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> { balances: self.balances }.assimilate_storage(&mut t).unwrap();

		t.into()
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut ext = ExtBuilder::default().build();
	ext.execute_with(|| System::set_block_number(1));

	MockPriceFeed::set_prices(vec![
		(ASSET_A_ID, Price::from(ASSET_A_PRICE_MULTIPLIER)),
		(ASSET_B_ID, Price::from(ASSET_B_PRICE_MULTIPLIER)),
	]);

	ext
}

pub fn new_test_ext_with_balance(balances: Vec<(AccountId, AssetId, Balance)>) -> sp_io::TestExternalities {
	let mut ext = ExtBuilder::default().with_balances(balances).build();
	ext.execute_with(|| System::set_block_number(1));

	MockPriceFeed::set_prices(vec![
		(ASSET_A_ID, Price::from(ASSET_A_PRICE_MULTIPLIER)),
		(ASSET_B_ID, Price::from(ASSET_B_PRICE_MULTIPLIER)),
	]);

	ext
}
