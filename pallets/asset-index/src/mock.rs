// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// Required as construct_runtime! produces code that violates this lint
#![allow(clippy::from_over_into)]

use crate as pallet_asset_index;
use frame_support::traits::StorageMapShim;
use frame_support::{ord_parameter_types, parameter_types, PalletId};
use frame_system as system;
use pallet_asset_index::traits::{AssetAvailability, AssetRecorder};

use frame_support::dispatch::DispatchResult;
use frame_support::sp_runtime::FixedPointNumber;
use pallet_price_feed::{AssetPricePair, Price, PriceFeed};
use pallet_remote_asset_manager::RemoteAssetManager;
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
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        AssetIndex: pallet_asset_index::{Pallet, Call, Storage, Event<T>},
        AssetDepository: pallet_asset_depository::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

pub(crate) type Balance = u128;
pub(crate) type AccountId = u64;
pub(crate) type AssetId = u32;

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

pub struct MockAssetRecorder;

impl<AssetId, Balance> AssetRecorder<AssetId, Balance> for MockAssetRecorder {
    fn add_asset(_: &AssetId, _: &Balance, _: &AssetAvailability) -> Result<(), DispatchError> {
        Ok(())
    }
    fn remove_asset(_: &AssetId) -> Result<(), DispatchError> {
        Ok(())
    }
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
    type WeightInfo = ();
}

impl pallet_asset_depository::Config for Test {
    type Event = Event;
    type AssetId = AssetId;
    type Balance = Balance;
}

parameter_types! {
    pub LockupPeriod: <Test as system::Config>::BlockNumber = 10;
    pub MinimumRedemption: u32 = 2;
    pub WithdrawalPeriod: <Test as system::Config>::BlockNumber = 10;
    pub DOTContributionLimit: Balance = 999;
    pub TreasuryPalletId: PalletId = PalletId(*b"12345678");
}

impl pallet_asset_index::Config for Test {
    type AdminOrigin = frame_system::EnsureSignedBy<AdminAccountId, AccountId>;
    type Event = Event;
    type AssetId = AssetId;
    type IndexToken = Balances;
    type Balance = Balance;
    type LockupPeriod = LockupPeriod;
    type MinimumRedemption = MinimumRedemption;
    type WithdrawalPeriod = WithdrawalPeriod;
    type DOTContributionLimit = DOTContributionLimit;
    type RemoteAssetManager = MockRemoteAssetManager;
    type MultiAssetDepository = AssetDepository;
    type PriceFeed = MockPriceFeed;
    type TreasuryPalletId = TreasuryPalletId;
    type WithdrawalFee = ();
    type WeightInfo = ();
}

pub struct MockRemoteAssetManager;
impl<AccountId, AssetId, Balance> RemoteAssetManager<AccountId, AssetId, Balance>
    for MockRemoteAssetManager
{
    fn reserve_withdraw_and_deposit(
        _who: AccountId,
        _asset: AssetId,
        _amount: Balance,
    ) -> DispatchResult {
        Ok(())
    }

    fn bond(_: AssetId, _: Balance) -> DispatchResult {
        Ok(())
    }

    fn unbond(_: AssetId, _: Balance) -> DispatchResult {
        Ok(())
    }
    fn withdraw_unbonded(_who: AccountId, _asset: AssetId, _amount: Balance) -> DispatchResult {
        Ok(())
    }
}

pub const PINT_ASSET_ID: AssetId = 0u32;
pub const ASSET_A_ID: AssetId = 1u32;
pub const ASSET_B_ID: AssetId = 2u32;
pub const UNKNOWN_ASSET_ID: AssetId = 3u32;

pub const ASSET_A_PRICE_MULTIPLIER: Balance = 2;
pub const ASSET_B_PRICE_MULTIPLIER: Balance = 3;

pub struct MockPriceFeed;
impl PriceFeed<AssetId> for MockPriceFeed {
    fn get_price(quote: AssetId) -> Result<AssetPricePair<AssetId>, DispatchError> {
        Self::get_price_pair(PINT_ASSET_ID, quote)
    }

    fn get_price_pair(
        base: AssetId,
        quote: AssetId,
    ) -> Result<AssetPricePair<AssetId>, DispatchError> {
        let price = match quote {
            // includes unknown asset id since we don't need to mock initial price pair here
            ASSET_A_ID | UNKNOWN_ASSET_ID => {
                Price::checked_from_rational(600, 600 / ASSET_A_PRICE_MULTIPLIER).unwrap()
            }
            ASSET_B_ID => {
                Price::checked_from_rational(900, 900 / ASSET_B_PRICE_MULTIPLIER).unwrap()
            }
            _ => return Err(pallet_asset_index::Error::<Test>::UnsupportedAsset.into()),
        };
        Ok(AssetPricePair { base, quote, price })
    }

    fn ensure_price(_: AssetId, _: Price) -> Result<AssetPricePair<AssetId>, DispatchError> {
        // pass all unknown asset ids
        Self::get_price(UNKNOWN_ASSET_ID)
    }
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        // Assign initial balances to accounts
        balances,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
