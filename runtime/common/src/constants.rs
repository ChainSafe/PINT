// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use cumulus_pallet_xcm::Origin;
use frame_support::{
    // pallet_predule::Weight,
    parameter_types,
    sp_runtime::{traits::AccountIdConversion, Perbill},
    traits::LockIdentifier,
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_PER_SECOND},
        DispatchClass, Weight,
    },
    PalletId,
};
use frame_system::limits::{BlockLength, BlockWeights};
use orml_traits::{arithmetic::Zero, parameter_type_with_key};
use primitives::{fee::FeeRate, AccountId, AssetId, Balance, BlockNumber};
use xcm::v0::{Junction, MultiLocation};

// 1 in 4 blocks (on average, not counting collisions) will be primary babe
// blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

/// We assume that ~10% of the block weight is consumed by `on_initalize`
/// handlers. This is used to limit the maximal weight of a single extrinsic.
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be
/// used by  Operational  extrinsics.
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 0.5 seconds of compute with a 6 second average block time.
pub const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by
/// `SLOT_DURATION`. `SLOT_DURATION` is picked up by `pallet_timestamp`
/// which is in turn picked up by `pallet_aura` to implement `fn
/// slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLIUNIT: Balance = 1_000_000_000;
pub const MICROUNIT: Balance = 1_000_000;

parameter_types! {
    // TODO: use actual fees
    pub const BaseWithdrawalFee: FeeRate = FeeRate{ numerator: 0, denominator: 1_000,};
    // The base weight for an XCM message
    // The actual weight for an XCM message will determined by
    // `T::BaseXcmWeight  + T::Weigher::weight(&msg)`
    pub const BaseXcmWeight: Weight = 100_000_000;
    pub const BlockHashCount: BlockNumber = 250;
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
    pub const DOTContributionLimit: Balance = 999;
    /// Same as Polkadot Relay Chain.
    pub const ExistentialDeposit: Balance = 500;
    // Used to determine the account for storing the funds used to pay the oracles.
    pub const FeedPalletId: PalletId = PalletId(*b"linkfeed");
    // Maximum number of feeds
    pub const FeedLimit: u16 = 10;
    pub const IndexTokenLockIdentifier: LockIdentifier = *b"pintlock";
    pub const Offset: BlockNumber = 0;
    // Maximum number of oracles per feed
    pub const OracleLimit: u32 = 10;
    pub const PalletIndexStringLimit: u32 = 50;
    pub const Period: u32 = 6 * HOURS;
    pub const PINTAssetId: AssetId = 1;
    pub PintTreasuryAccount: AccountId = TreasuryPalletId::get().into_account();
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const RelayChainAssetId: AssetId = 0;
    pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
    pub const RelayLocation: MultiLocation = MultiLocation::X1(Junction::Parent);
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u8 = 0;
    pub StatemintCustodian: AccountId = PalletId(*b"pint/smt").into_account();
    // Maximum allowed string length for feed names
    pub const StringLimit: u32 = 15;
    pub const TransactionByteFee: Balance = 1 ;
    pub const TreasuryPalletId: PalletId = PalletId(*b"Treasury");
    pub const LockupPeriod: BlockNumber = 10;
    pub const MaxCandidates: u32 = 200;
    pub const MaxInvulnerables: u32 = 50;
    // For weight estimation, we assume that the most locks on an individual account will be 50.
    // This number may need to be adjusted in the future if this assumption no longer holds true.
    pub const MaxLocks: u32 = 50;
    pub const MinCandidates: u32 = 1;
    pub const MinCouncilVotes: usize = 4;
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
    pub const MinimumRedemption: u32 = 0;
    pub const MinimumStatemintTransferAmount: Balance = 1;
    // Minimum amount of funds that need to be present in the fund account
    pub const MinimumReserve: Balance = 100;
    pub const UncleGenerations: u32 = 0;
    // One UNIT buys 1 second of weight.
    pub const UnitPerSecond: (MultiLocation, u128) = (MultiLocation::X1(Junction::Parent), UNIT);
    // One XCM operation is 200_000_000 weight, cross-chain transfer ~= 2x of transfer.
    pub const UnitWeightCost: Weight = 200_000_000;
    pub const WithdrawalPeriod: BlockNumber = 10;
}

// --- ORML configurations
parameter_type_with_key! {
    pub ExistentialDeposits: |_asset_id: AssetId| -> Balance {
        Zero::zero()
    };
}

// The minimum amount of assets that should remain unbonded.
parameter_type_with_key! {
    pub MinimumRemoteStashBalance: |_asset_id: AssetId| -> Balance {
        // Same as relaychain existential deposit
        ExistentialDeposit::get()
    };
}
