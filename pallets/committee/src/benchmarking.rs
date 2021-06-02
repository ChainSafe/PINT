// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{benchmarks, vec, whitelisted_caller, Box};
use frame_support::assert_ok;
use frame_system::{Call as SystemCall, RawOrigin as SystemOrigin};

benchmarks! {
    propose {
        let caller: T::AccountId = whitelisted_caller();
    }: _(
        SystemOrigin::Signed(caller.clone()),
        Box::new(<SystemCall<T>>::remark(vec![0; 0]).into())
    ) verify {
        // TODO:
        //
        // assert_eq!(
        //     <System<T>>::events().pop().expect("Event expected").event,
        //     Event::pallet_committee(crate::Event::Proposed(caller, _, _))
        // );
    }

    vote {
        let caller: T::AccountId = whitelisted_caller();
        let expected_nonce = <pallet::ProposalCount<T>>::get();
        let action: T::Action = <SystemCall<T>>::remark(vec![0; 0]).into();
        assert_ok!(<Pallet<T>>::propose(
            SystemOrigin::Signed(caller.clone()).into(),
            Box::new(action.clone())),
        );

        let proposal: pallet::Proposal<T> = <pallet::Proposal<T>>::new(
            expected_nonce,
            action,
        );
    }: _(
        SystemOrigin::Signed(caller.clone()),
        proposal.hash(),
        Vote::Abstain
    ) verify {

    }
    // close {
    //
    // }: _(
    //
    // ) verify {
    //
    // }
    //
    // add_constituent {
    //
    // }: _(
    //
    // ) verify {
    //
    // }
}
