// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Support for creating XCM calls to be used within `Xcm::Transact`

pub mod proxy;
pub mod staking;

use crate::EncodeWith;
use codec::{Encode, EncodeLike, Output};
use frame_support::sp_std::marker::PhantomData;

/// Represents an extrinsic of a pallet configured inside a runtime
#[derive(Encode)]
pub struct RuntimeCall<Call> {
    /// The index of the call's pallet within the runtime.
    ///
    /// This must be equivalent with the `#[codec(index = <pallet_index>)]` annotation.
    pub pallet_index: u8,

    /// The actual that should be dispatched
    pub call: Call,
}

pub trait PalletCall: Sized {
    /// Returns the index of the call within its pallet
    fn pallet_call_index(&self) -> u8;

    fn encoder<'a, 'b, Config: PalletCallEncoder>(
        &'a self,
        ctx: &'b Config::Context,
    ) -> CallEncoder<'a, 'b, Self, Config> {
        CallEncoder::new(self, ctx)
    }
}

/// Common trait for encoders of pallet calls
pub trait PalletCallEncoder {
    type Context;
    /// Whether the encoder can be applied
    fn can_encode(ctx: &Self::Context) -> bool;
}

/// Helps encoding the inner call with additional context
pub struct CallEncoder<'a, 'b, Call, Config: PalletCallEncoder> {
    /// The call to encode
    pub call: &'a Call,
    /// additional context required for encoding
    pub ctx: &'b Config::Context,
    marker: PhantomData<Config>,
}

impl<'a, 'b, Call, Config: PalletCallEncoder> CallEncoder<'a, 'b, Call, Config> {
    pub fn new(call: &'a Call, ctx: &'b Config::Context) -> Self {
        Self {
            call,
            ctx,
            marker: Default::default(),
        }
    }

    /// Wraps the pallet call into a `RuntimeCall` with the given pallet index
    pub fn into_runtime_call(self, pallet_index: u8) -> RuntimeCall<Self> {
        RuntimeCall {
            pallet_index,
            call: self,
        }
    }
}

/// Wrapper around something to encode with additional context
pub struct ContextEncode<'a, I, C, E> {
    pub input: &'a I,
    pub ctx: &'a C,
    pub encoder: PhantomData<E>,
}

impl<'a, I, C, E> Encode for ContextEncode<'a, I, C, E>
where
    E: EncodeWith<I, C>,
{
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        E::encode_to_with(self.input, self.ctx, dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::calls::staking::{Bond, StakingCall, StakingCallEncoder};
    use crate::{
        PassthroughCompactEncoder, PassthroughEncoder, MultiAddressLookupSourceEncoder,
    };
    use frame_election_provider_support::onchain;
    use frame_support::sp_runtime::traits::BlakeTwo256;
    use frame_support::traits::OnUnbalanced;
    use frame_support::traits::{Imbalance, InstanceFilter};
    use frame_support::{
        parameter_types,
        traits::{Currency, FindAuthor, OneSessionHandler},
        weights::constants::RocksDbWeight,
    };
    use pallet_staking as staking;
    use pallet_staking::*;
    use sp_core::H256;
    use sp_runtime::{
        curve::PiecewiseLinear,
        testing::{Header, TestXt, UintAuthorityId},
        traits::IdentityLookup,
        Perbill,
    };

    use codec::{Decode, Encode};
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

            // use polkadot index 7
            Staking: staking::{Pallet, Call, Storage, Event<T>} = 7,

            Session: pallet_session::{Pallet, Call, Storage, Event},

            // use polkadot index 29
            Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 29,
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

    parameter_types! {
        pub const ProxyDepositBase: Balance = 100;
        pub const ProxyDepositFactor: Balance = 100;
        pub const MaxProxies: u16 = 32;
        pub const AnnouncementDepositBase: Balance = 100;
        pub const AnnouncementDepositFactor: Balance = 100;
        pub const MaxPending: u16 = 32;
    }

    /// The type used to represent the kinds of proxying allowed.
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug)]
    pub enum ProxyType {
        Any = 0,
        NonTransfer = 1,
        Governance = 2,
        Staking = 3,
        // Skip 4 as it is now removed (was SudoBalances)
        IdentityJudgement = 5,
        CancelProxy = 6,
    }

    impl Default for ProxyType {
        fn default() -> Self {
            Self::Any
        }
    }
    impl InstanceFilter<Call> for ProxyType {
        fn filter(&self, _: &Call) -> bool {
            true
        }
    }

    impl pallet_proxy::Config for Test {
        type Event = Event;
        type Call = Call;
        type Currency = Balances;
        type ProxyType = ProxyType;
        type ProxyDepositBase = ProxyDepositBase;
        type ProxyDepositFactor = ProxyDepositFactor;
        type MaxProxies = MaxProxies;
        type WeightInfo = ();
        type MaxPending = MaxPending;
        type CallHasher = BlakeTwo256;
        type AnnouncementDepositBase = AnnouncementDepositBase;
        type AnnouncementDepositFactor = AnnouncementDepositFactor;
    }

    impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
    where
        Call: From<LocalCall>,
    {
        type OverarchingCall = Call;
        type Extrinsic = TestXt<Call, ()>;
    }

    type AssetId = u64;

    struct PalletStakingEncoder;
    impl StakingCallEncoder<AccountId, Balance, AccountId> for PalletStakingEncoder {
        type CompactBalanceEncoder = PassthroughCompactEncoder<Balance,AssetId>;
        type SourceEncoder = PassthroughEncoder<AccountId, AssetId>;
        type AccountIdEncoder = PassthroughEncoder<AccountId, AssetId>;
    }

    impl PalletCallEncoder for PalletStakingEncoder {
        type Context = AssetId;
        fn can_encode(ctx: &u64) -> bool {
            true
        }
    }

    type PalletStakingCall = pallet_staking::Call<Test>;
    type PalletProxyCall = pallet_proxy::Call<Test>;

    type RemoteStakingCall = StakingCall<AccountId, Balance, AccountId>;

    // type RemoteProxyCall = ProxyCall<AccountId, u8, BlockNumber>;

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
        let remote_bond_extra = RemoteStakingCall::BondExtra(100);
        let bond_extra = PalletStakingCall::bond_extra(100);
        let remote_encoder = remote_bond_extra.encoder::<PalletStakingEncoder>(&0);
        let remote_pallet_call_encoded = remote_encoder.encode();
        let call_encoded = bond_extra.encode();
        assert_eq!(remote_pallet_call_encoded, call_encoded);

        let bond_extra_decoded =
            PalletStakingCall::decode(&mut remote_pallet_call_encoded.as_slice()).unwrap();
        assert_eq!(bond_extra, bond_extra_decoded);

        let runtime_call: Call = bond_extra.into();
        let remote_runtime_call_encoded = remote_encoder.into_runtime_call(7).encode();
        let runtime_call_encoded = runtime_call.encode();
        assert_eq!(remote_runtime_call_encoded, runtime_call_encoded);

        let runtime_call_decoded =
            Call::decode(&mut remote_runtime_call_encoded.as_slice()).unwrap();
        assert_eq!(runtime_call, runtime_call_decoded);
    }

    #[test]
    fn can_encode_decode_bond() {
        let controller = 9;
        let value = 100;

        let remote_bond = RemoteStakingCall::Bond(Bond {
            controller,
            value,
            payee: super::staking::RewardDestination::Stash,
        });
        let bond =
            PalletStakingCall::bond(controller, value, pallet_staking::RewardDestination::Stash);

        let remote_encoder = remote_bond.encoder::<PalletStakingEncoder>(&0);
        let remote_pallet_call_encoded = remote_encoder.encode();
        assert_eq!(remote_pallet_call_encoded, bond.encode());

        let bond_extra_decoded =
            PalletStakingCall::decode(&mut remote_pallet_call_encoded.as_slice()).unwrap();
        assert_eq!(bond, bond_extra_decoded);

        let runtime_call: Call = bond.into();
        let remote_runtime_call_encoded = remote_encoder.into_runtime_call(7).encode();
        let runtime_call_encoded = runtime_call.encode();
        assert_eq!(remote_runtime_call_encoded, runtime_call_encoded);

        let runtime_call_decoded =
            Call::decode(&mut remote_runtime_call_encoded.as_slice()).unwrap();
        assert_eq!(runtime_call, runtime_call_decoded);
    }

    #[test]
    fn can_encode_decode_unbond() {
        let remote_unbond = RemoteStakingCall::Unbond(100);
        let unbond = PalletStakingCall::unbond(100);

        let remote_encoder = remote_unbond.encoder::<PalletStakingEncoder>(&0);
        let remote_pallet_call_encoded = remote_encoder.encode();
        assert_eq!(remote_pallet_call_encoded, unbond.encode());

        let bond_extra_decoded =
            PalletStakingCall::decode(&mut remote_pallet_call_encoded.as_slice()).unwrap();
        assert_eq!(unbond, bond_extra_decoded);

        let runtime_call: Call = unbond.into();
        let remote_runtime_call_encoded = remote_encoder.into_runtime_call(7).encode();
        let runtime_call_encoded = runtime_call.encode();
        assert_eq!(remote_runtime_call_encoded, runtime_call_encoded);

        let runtime_call_decoded =
            Call::decode(&mut remote_runtime_call_encoded.as_slice()).unwrap();
        assert_eq!(runtime_call, runtime_call_decoded);
    }

    // #[test]
    // fn can_encode_decode_add_proxy() {
    //     let delegate = 1337;
    //     let remote_add_proxy = RemoteProxyCall::AddProxy {
    //         proxy: delegate,
    //         proxy_type: POLKADOT_PALLET_PROXY_TYPE_STAKING_INDEX,
    //         delay: 0,
    //     };
    //
    //     let add_proxy = PalletProxyCall::add_proxy(delegate, ProxyType::Staking, 0);
    //
    //     let remote_pallet_call_encoded = remote_add_proxy.encode();
    //     let call_encoded = add_proxy.encode();
    //     assert_eq!(remote_pallet_call_encoded, call_encoded);
    //
    //     let add_proxy_decoded =
    //         PalletProxyCall::decode(&mut remote_pallet_call_encoded.as_slice()).unwrap();
    //     assert_eq!(add_proxy, add_proxy_decoded);
    //
    //     let runtime_call: Call = add_proxy.into();
    //     let remote_runtime_call_encoded = remote_add_proxy
    //         .into_runtime_call(POLKADOT_PALLET_PROXY_INDEX)
    //         .encode();
    //     let runtime_call_encoded = runtime_call.encode();
    //     assert_eq!(remote_runtime_call_encoded, runtime_call_encoded);
    //
    //     let runtime_call_decoded =
    //         Call::decode(&mut remote_runtime_call_encoded.as_slice()).unwrap();
    //     assert_eq!(runtime_call, runtime_call_decoded);
    // }
}
