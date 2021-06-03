// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{benchmarks, vec, whitelisted_caller, Box};
use frame_support::assert_ok;
use frame_system::{Call as SystemCall, RawOrigin as SystemOrigin};

fn submit_proposal<T: Config>() -> pallet::Proposal<T> {
    let action: T::Action = <SystemCall<T>>::remark(vec![0; 0]).into();
    let expected_nonce = pallet::ProposalCount::<T>::get();
    assert_ok!(<Pallet<T>>::propose(
        SystemOrigin::Root.into(),
        Box::new(action.clone())
    ));
    pallet::Proposal::<T>::new(expected_nonce, action)
}

benchmarks! {
    propose {
        let caller: T::AccountId = whitelisted_caller();
    }: _(
        SystemOrigin::Root,
        Box::new(<SystemCall<T>>::remark(vec![0; 0]).into())
    ) verify {
        // TODO:
        //
        // verify last event

    }

    vote {
        let caller: T::AccountId = whitelisted_caller();
        let proposal = submit_proposal::<T>();
    }: _(
        SystemOrigin::Root,
        proposal.hash(),
        Vote::Abstain
    ) verify {
        // TODO:
        //
        // verify last event
    }

    close {
        let caller: T::AccountId = whitelisted_caller();
    }: _(
        SystemOrigin::Root,
        submit_proposal::<T>().hash()
    ) verify {
        // TODO:
        //
        // verify last event
    }

    // TODO:
    //
    // This is hard to benchmark limited by the `Call` in environment.
    //
    // Use the weight of `propose` currently
    //
    //
    // add_constituent {
    //
    // }: _() verify {
    //
    // }
}
