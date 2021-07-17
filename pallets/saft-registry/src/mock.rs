// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_saft_registry;
use frame_support::{ord_parameter_types, parameter_types, traits::StorageMapShim, PalletId};
use frame_system as system;
use orml_traits::parameter_type_with_key;
use pallet_price_feed::{AssetPricePair, Price, PriceFeed};
use primitives::traits::RemoteAssetManager;
use sp_runtime::DispatchResult;

use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, Zero},
    DispatchError,
};
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
    pub LockupPeriod: <Test as system::Config>::BlockNumber = 10;
    pub MinimumRedemption: u32 = 2;
    pub WithdrawalPeriod: <Test as system::Config>::BlockNumber = 10;
    pub DOTContributionLimit: Balance = 999;
    pub TreasuryPalletId: PalletId = PalletId(*b"12345678");
    pub StringLimit: u32 = 4;
    pub const PINTAssetId: AssetId = 99;

    // No fees for now
    pub const BaseWithdrawalFee: primitives::fee::FeeRate = primitives::fee::FeeRate{ numerator: 0, denominator: 1_000,};
}

impl pallet_asset_index::Config for Test {
    type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
    type Event = Event;
    type AssetId = AssetId;
    type SelfAssetId = PINTAssetId;
    type IndexToken = Balances;
    type Balance = Balance;
    type LockupPeriod = LockupPeriod;
    type MinimumRedemption = MinimumRedemption;
    type WithdrawalPeriod = WithdrawalPeriod;
    type DOTContributionLimit = DOTContributionLimit;
    type RemoteAssetManager = MockRemoteAssetManager;
    type Currency = Currency;
    type PriceFeed = MockPriceFeed;
    type TreasuryPalletId = TreasuryPalletId;
    type StringLimit = StringLimit;
    type BaseWithdrawalFee = BaseWithdrawalFee;
    type WeightInfo = ();
}

pub struct MockRemoteAssetManager;
impl<AccountId, AssetId, Balance> RemoteAssetManager<AccountId, AssetId, Balance>
    for MockRemoteAssetManager
{
    fn transfer_asset(
        _: AccountId,
        _: AssetId,
        _: Balance,
    ) -> frame_support::sp_std::result::Result<Outcome, DispatchError> {
        Ok(Outcome::Complete(0))
    }

    fn bond(_: AssetId, _: Balance) -> DispatchResult {
        Ok(())
    }

    fn unbond(_: AssetId, _: Balance) -> DispatchResult {
        Ok(())
    }
}

pub struct MockPriceFeed;
impl PriceFeed<AssetId> for MockPriceFeed {
    fn get_price(_quote: AssetId) -> Result<AssetPricePair<AssetId>, DispatchError> {
        todo!()
    }

    fn get_price_pair(
        _base: AssetId,
        _quote: AssetId,
    ) -> Result<AssetPricePair<AssetId>, DispatchError> {
        todo!()
    }

    fn ensure_price(_: AssetId, _: Price) -> Result<AssetPricePair<AssetId>, DispatchError> {
        todo!()
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
}
pub(crate) const ADMIN_ACCOUNT_ID: AccountId = 1337;

ord_parameter_types! {
    pub const AdminAccountId: AccountId = ADMIN_ACCOUNT_ID;
}

impl pallet_saft_registry::Config for Test {
    type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
    type Event = Event;
    type Balance = Balance;
    type AssetRecorder = AssetIndex;
    type AssetId = AssetId;
    type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    t.into()
}
