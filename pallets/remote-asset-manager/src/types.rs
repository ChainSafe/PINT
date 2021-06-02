// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Additional types for the remote asset manager pallet
use codec::{Decode, Encode, EncodeLike};
use frame_support::{dispatch::Output, sp_runtime::RuntimeDebug, sp_std::prelude::*};
use xcm::v0::Outcome as XcmOutcome;

use crate::traits::BalanceEncoder;
use crate::EncodedBalance;
use frame_support::sp_std::marker::PhantomData;
use frame_support::weights::constants::RocksDbWeight;
use frame_support::weights::Weight;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use xcm::opaque::v0::Outcome;

/// Represents an extrinsic of a pallet configured inside a runtime
pub struct RuntimeCall<Call: Encode> {
    /// The index of the call's pallet within the runtime.
    ///
    /// This must be equivalent with the `#[codec(index = <pallet_index>)]` annotation.
    pub pallet_index: u8,

    /// The actual that should be dispatched
    pub call: Call,
}

impl<Call: EncodeLike> EncodeLike for RuntimeCall<Call> {}

impl<Call: EncodeLike> Encode for RuntimeCall<Call> {
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        dest.push_byte(self.pallet_index);
        self.call.encode_to(dest)
    }
}

/// Encodes a `u128` using `CompactRef` regardless of the asset id
pub struct CompactU128BalanceEncoder<T>(PhantomData<T>);

impl<AssetId> BalanceEncoder<AssetId, u128> for CompactU128BalanceEncoder<AssetId> {
    fn encoded_balance(_: &AssetId, balance: u128) -> Option<EncodedBalance> {
        // Compact(balance).encode()
        let encoded =
            <<u128 as codec::HasCompact>::Type as codec::EncodeAsRef<'_, u128>>::RefType::from(
                &balance,
            )
            .encode();
        Some(encoded.into())
    }
}

/// Represents dispatchable calls of the FRAME `pallet_staking` pallet.
///
/// *NOTE*: `CompactBalance` is expected to encode with `HasCompact`
#[derive(Encode)]
pub enum StakingCall<AccountId: Encode, CompactBalance: Encode, Source: Encode> {
    /// The [`bond_extra`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond_extra) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the stash account.
    #[codec(index = 0)]
    Bond(
        // The controller to use
        // on polkadot this is of type `MultiAddress<AcountId, AccountIndex>`
        Source,
        CompactBalance,
        RewardDestination<AccountId>,
    ),

    /// The [`bond_extra`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.bond_extra) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
    #[codec(index = 1)]
    BondExtra(CompactBalance),
    /// The [`unbond`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.unbond) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
    #[codec(index = 2)]
    Unbond(CompactBalance),
    /// The [`withdraw_unbonded`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.withdraw_unbonded) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
    #[codec(index = 3)]
    WithdrawUnbonded(u32),
    /// The [`nominate`](https://crates.parity.io/pallet_staking/enum.Call.html#variant.nominate) extrinsic.
    ///
    /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
    #[codec(index = 5)]
    Nominate(Vec<Source>),
}

impl<AccountId: Encode, CompactBalance: Encode, Source: Encode>
    StakingCall<AccountId, CompactBalance, Source>
{
    /// Wraps the staking pallet call into a `RuntimeCall`
    pub fn into_runtime_call(self, pallet_index: u8) -> RuntimeCall<Self> {
        RuntimeCall {
            pallet_index,
            call: self,
        }
    }
}

/// A destination account for payment.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum RewardDestination<AccountId> {
    /// Pay into the stash account, increasing the amount at stake accordingly.
    Staked,
    /// Pay into the stash account, not increasing the amount at stake.
    Stash,
    /// Pay into the controller account.
    Controller,
    /// Pay into a specified account.
    Account(AccountId),
    /// Receive no reward.
    None,
}

/// The `pallet_staking` configuration for a particular chain
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StakingConfig<AccountId, Balance> {
    /// The index of `pallet_index` within the parachain's runtime
    pub pallet_index: u32,
    /// The limitation to the number of fund-chunks that can be scheduled to be unlocked via `unbond`.
    ///
    /// If this is reached, the bonded account _must_ first wait until successful call to
    /// `withdraw_unbonded` to remove some of the chunks.
    pub max_unlocking_chunks: u32,
    /// Counter for the sent `unbond` calls.
    pub pending_unbond_calls: u32,
    /// The configured reward destination
    pub reward_destination: RewardDestination<AccountId>,
    /// The specified `minimum_balance` specified the parachain's `T::Currency`
    pub minimum_balance: Balance,
    /// The configured weights for `pallet_staking`
    pub weights: StakingWeights,
    // TODO add minumum (un)bond  that has to be met for executing XCM (un)bonding calls
}

/// The index of `pallet_staking` in the polkadot runtime
pub const POLKADOT_PALLET_STAKING_INDEX: u8 = 7u8;

/// Represents an excerpt from the `pallet_staking` weights
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StakingWeights {
    /// Weight for `bond` extrinsic
    pub bond: Weight,
    /// Weight for `bond_extra` extrinsic
    pub bond_extra: Weight,
    /// Weight for `unbond` extrinsic
    pub unbond: Weight,
}

impl StakingWeights {
    /// The weights as defined in `pallet_staking` on polkadot
    // TODO: import pallet_staking weights directly?
    pub fn polkadot() -> Self {
        let weight = RocksDbWeight::get();
        Self {
            bond: (75_102_000 as Weight)
                .saturating_add(weight.reads(5 as Weight))
                .saturating_add(weight.writes(4 as Weight)),
            bond_extra: (57_637_000 as Weight)
                .saturating_add(weight.reads(3 as Weight))
                .saturating_add(weight.writes(2 as Weight)),
            unbond: (52_115_000 as Weight)
                .saturating_add(weight.reads(4 as Weight))
                .saturating_add(weight.writes(3 as Weight)),
        }
    }
}

/// Outcome of an XCM staking execution.
#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug)]
pub enum StakingOutcome {
    /// Staking is not supported for the given asset
    ///
    /// No `StakingConfig` found
    NotSupported,
    /// Outcome of the executed staking xcm routine
    XcmOutcome(XcmOutcome),
}

impl From<XcmOutcome> for StakingOutcome {
    fn from(outcome: XcmOutcome) -> Self {
        StakingOutcome::XcmOutcome(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use frame_election_provider_support::onchain;
    use frame_support::traits::Imbalance;
    use frame_support::traits::OnUnbalanced;
    use frame_support::{
        parameter_types,
        traits::{Currency, FindAuthor,OneSessionHandler},
        weights::constants::RocksDbWeight,
    };
    use pallet_staking as staking;
    use pallet_staking::*;
    use sp_core::H256;
    use sp_runtime::{
        curve::PiecewiseLinear,
        testing::{Header, TestXt, UintAuthorityId},
        traits::{IdentityLookup},
        Perbill,
    };
    use std::{cell::RefCell, collections::HashSet};
    use xcm::DoubleEncoded;

    /// The AccountId alias in this test module.
    pub(crate) type AccountId = u64;
    pub(crate) type AccountIndex = u64;
    pub(crate) type BlockNumber = u64;
    pub(crate) type Balance = u128;

    thread_local! {
        static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
    }

    type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    /// Another session handler struct to test on_disabled.
    pub struct OtherSessionHandler;
    impl OneSessionHandler<AccountId> for OtherSessionHandler {
        type Key = UintAuthorityId;

        fn on_genesis_session<'a, I: 'a>(_: I)
        where
            I: Iterator<Item = (&'a AccountId, Self::Key)>,
            AccountId: 'a,
        {
        }

        fn on_new_session<'a, I: 'a>(_: bool, validators: I, _: I)
        where
            I: Iterator<Item = (&'a AccountId, Self::Key)>,
            AccountId: 'a,
        {
            SESSION.with(|x| {
                *x.borrow_mut() = (validators.map(|x| x.0.clone()).collect(), HashSet::new())
            });
        }

        fn on_disabled(validator_index: usize) {
            SESSION.with(|d| {
                let mut d = d.borrow_mut();
                let value = d.0[validator_index];
                d.1.insert(value);
            })
        }
    }

    impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
        type Public = UintAuthorityId;
    }

    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
            Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
            Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},

            // use index 7
            Staking: staking::{Pallet, Call, Storage, Event<T>} = 7,

            Session: pallet_session::{Pallet, Call, Storage, Event},
        }
    );

    /// Author of block is always 11
    pub struct Author11;
    impl FindAuthor<AccountId> for Author11 {
        fn find_author<'a, I>(_digests: I) -> Option<AccountId>
        where
            I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
        {
            Some(11)
        }
    }

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub BlockWeights: frame_system::limits::BlockWeights =
            frame_system::limits::BlockWeights::simple_max(
                frame_support::weights::constants::WEIGHT_PER_SECOND * 2
            );
        pub const MaxLocks: u32 = 1024;
        pub static SessionsPerEra: sp_staking::SessionIndex = 3;
        pub static ExistentialDeposit: Balance = 1;
        pub static SlashDeferDuration: EraIndex = 0;
        pub static Period: BlockNumber = 5;
        pub static Offset: BlockNumber = 0;
    }

    impl frame_system::Config for Test {
        type BaseCallFilter = ();
        type BlockWeights = ();
        type BlockLength = ();
        type DbWeight = RocksDbWeight;
        type Origin = Origin;
        type Index = AccountIndex;
        type BlockNumber = BlockNumber;
        type Call = Call;
        type Hash = H256;
        type Hashing = ::sp_runtime::traits::BlakeTwo256;
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
        type SS58Prefix = ();
        type OnSetCode = ();
    }
    impl pallet_balances::Config for Test {
        type MaxLocks = MaxLocks;
        type Balance = Balance;
        type Event = Event;
        type DustRemoval = ();
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
    }
    parameter_types! {
        pub const UncleGenerations: u64 = 0;
        pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
    }
    sp_runtime::impl_opaque_keys! {
        pub struct SessionKeys {
            pub other: OtherSessionHandler,
        }
    }
    impl pallet_session::Config for Test {
        type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
        type Keys = SessionKeys;
        type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
        type SessionHandler = (OtherSessionHandler,);
        type Event = Event;
        type ValidatorId = AccountId;
        type ValidatorIdOf = pallet_staking::StashOf<Test>;
        type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
        type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
        type WeightInfo = ();
    }

    impl pallet_session::historical::Config for Test {
        type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
        type FullIdentificationOf = pallet_staking::ExposureOf<Test>;
    }
    parameter_types! {
        pub const MinimumPeriod: u64 = 5;
    }
    impl pallet_timestamp::Config for Test {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = MinimumPeriod;
        type WeightInfo = ();
    }
    pallet_staking_reward_curve::build! {
        const I_NPOS: PiecewiseLinear<'static> = curve!(
            min_inflation: 0_025_000,
            max_inflation: 0_100_000,
            ideal_stake: 0_500_000,
            falloff: 0_050_000,
            max_piece_count: 40,
            test_precision: 0_005_000,
        );
    }
    parameter_types! {
        pub const BondingDuration: EraIndex = 3;
        pub const RewardCurve: &'static PiecewiseLinear<'static> = &I_NPOS;
        pub const MaxNominatorRewardedPerValidator: u32 = 64;
    }

    thread_local! {
        pub static REWARD_REMAINDER_UNBALANCED: RefCell<u128> = RefCell::new(0);
    }

    pub struct RewardRemainderMock;

    impl OnUnbalanced<NegativeImbalanceOf<Test>> for RewardRemainderMock {
        fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<Test>) {
            REWARD_REMAINDER_UNBALANCED.with(|v| {
                *v.borrow_mut() += amount.peek();
            });
            drop(amount);
        }
    }

    impl onchain::Config for Test {
        type AccountId = AccountId;
        type BlockNumber = BlockNumber;
        type BlockWeights = BlockWeights;
        type Accuracy = Perbill;
        type DataProvider = Staking;
    }
    impl staking::Config for Test {
        const MAX_NOMINATIONS: u32 = 16;
        type Currency = Balances;
        type UnixTime = Timestamp;
        type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
        type RewardRemainder = RewardRemainderMock;
        type Event = Event;
        type Slash = ();
        type Reward = ();
        type SessionsPerEra = SessionsPerEra;
        type SlashDeferDuration = SlashDeferDuration;
        type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
        type BondingDuration = BondingDuration;
        type SessionInterface = Self;
        type EraPayout = ConvertCurve<RewardCurve>;
        type NextNewSession = Session;
        type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
        type ElectionProvider = onchain::OnChainSequentialPhragmen<Self>;
        type WeightInfo = ();
    }

    impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
    where
        Call: From<LocalCall>,
    {
        type OverarchingCall = Call;
        type Extrinsic = TestXt<Call, ()>;
    }

    type PalletStakingCall = pallet_staking::Call<Test>;
    type RemoteStakingCall = StakingCall<AccountId, EncodedBalance, AccountId>;

    #[test]
    fn test_pallet_staking_call_codec() {
        let bond_extra = PalletStakingCall::bond_extra(100);
        let call: Call = bond_extra.clone().into();
        let mut encoded: DoubleEncoded<Call> = call.encode().into();
        assert!(encoded.ensure_decoded().is_ok());
        assert_eq!(encoded.take_decoded().unwrap(), call)
    }

    #[test]
    fn can_encode_decode_bond_extra() {
        let balance = CompactU128BalanceEncoder::<u128>::encoded_balance(&0, 100).unwrap();
        let remote_bond_extra = RemoteStakingCall::BondExtra(balance);
        let bond_extra = PalletStakingCall::bond_extra(100);

        let remote_pallet_call_encoded = remote_bond_extra.encode();
        let call_encoded = bond_extra.encode();
        assert_eq!(remote_pallet_call_encoded, call_encoded);

        let bond_extra_decoded =
            PalletStakingCall::decode(&mut remote_pallet_call_encoded.as_slice()).unwrap();
        assert_eq!(bond_extra, bond_extra_decoded);

        let runtime_call: Call = bond_extra.into();
        let remote_runtime_call_encoded = remote_bond_extra.into_runtime_call(7).encode();
        let runtime_call_encoded = runtime_call.encode();
        assert_eq!(remote_runtime_call_encoded, runtime_call_encoded);

        let runtime_call_decoded =
            Call::decode(&mut remote_runtime_call_encoded.as_slice()).unwrap();
        assert_eq!(runtime_call, runtime_call_decoded);
    }

    #[test]
    fn can_encode_decode_bond() {
        let balance = CompactU128BalanceEncoder::<u128>::encoded_balance(&0, 100).unwrap();
        let account = 1337;

        let remote_bond =
            RemoteStakingCall::Bond(account, balance, super::RewardDestination::Stash);
        let bond = PalletStakingCall::bond(account, 100, pallet_staking::RewardDestination::Stash);

        let remote_pallet_call_encoded = remote_bond.encode();
        let call_encoded = bond.encode();
        assert_eq!(remote_pallet_call_encoded, call_encoded);

        let bond_extra_decoded =
            PalletStakingCall::decode(&mut remote_pallet_call_encoded.as_slice()).unwrap();
        assert_eq!(bond, bond_extra_decoded);

        let runtime_call: Call = bond.into();
        let remote_runtime_call_encoded = remote_bond.into_runtime_call(7).encode();
        let runtime_call_encoded = runtime_call.encode();
        assert_eq!(remote_runtime_call_encoded, runtime_call_encoded);

        let runtime_call_decoded =
            Call::decode(&mut remote_runtime_call_encoded.as_slice()).unwrap();
        assert_eq!(runtime_call, runtime_call_decoded);
    }

    #[test]
    fn can_encode_decode_unbond() {
        let balance = CompactU128BalanceEncoder::<u128>::encoded_balance(&0, 100).unwrap();

        let remote_unbond = RemoteStakingCall::Unbond(balance);
        let unbond = PalletStakingCall::unbond(100);

        let remote_pallet_call_encoded = remote_unbond.encode();
        let call_encoded = unbond.encode();
        assert_eq!(remote_pallet_call_encoded, call_encoded);

        let bond_extra_decoded =
            PalletStakingCall::decode(&mut remote_pallet_call_encoded.as_slice()).unwrap();
        assert_eq!(unbond, bond_extra_decoded);

        let runtime_call: Call = unbond.into();
        let remote_runtime_call_encoded = remote_unbond.into_runtime_call(7).encode();
        let runtime_call_encoded = runtime_call.encode();
        assert_eq!(remote_runtime_call_encoded, runtime_call_encoded);

        let runtime_call_decoded =
            Call::decode(&mut remote_runtime_call_encoded.as_slice()).unwrap();
        assert_eq!(runtime_call, runtime_call_decoded);
    }
}
