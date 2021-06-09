// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{account, benchmarks, vec, whitelisted_caller, Box};
use frame_support::{assert_noop, assert_ok, traits::Get};
use frame_system::{Call as SystemCall, Pallet as System, RawOrigin as SystemOrigin};

fn submit_proposal<T: Config>(caller: T::AccountId) -> pallet::Proposal<T> {
    let action: T::Action = <SystemCall<T>>::remark(vec![0; 0]).into();
    let expected_nonce = pallet::ProposalCount::<T>::get();
    assert_ok!(<Pallet<T>>::propose(
        SystemOrigin::Signed(caller).into(),
        Box::new(action.clone())
    ));
    pallet::Proposal::<T>::new(expected_nonce, action)
}

benchmarks! {
    propose {
        let caller: T::AccountId = whitelisted_caller();
        assert_ok!(<Pallet<T>>::add_constituent(SystemOrigin::Root.into(), caller.clone()));
        let proposal = submit_proposal::<T>(caller.clone());
    }: _(
        SystemOrigin::Signed(caller.clone()),
        Box::new(<SystemCall<T>>::remark(vec![0; 0]).into())
    ) verify {
        assert!(<Pallet<T>>::get_proposal(&proposal.hash()) == Some(proposal));
    }

    vote {
        let caller: T::AccountId = whitelisted_caller();
        assert_ok!(<Pallet<T>>::add_constituent(SystemOrigin::Root.into(), caller.clone()));
        let proposal = submit_proposal::<T>(caller.clone());

        // run to voting period
        <System<T>>::set_block_number(
            <System<T>>::block_number()
                + <T as Config>::VotingPeriod::get()
                + <T as Config>::ProposalSubmissionPeriod::get() + 1_u32.into(),
        );
    }: _(
        SystemOrigin::Signed(caller.clone()),
        proposal.hash(),
        Vote::Abstain
    ) verify {
        assert_eq!(
            <Pallet<T>>::get_votes_for(&proposal.hash()).unwrap().votes.len(),
            1,
        );
    }

    close {
        let caller: T::AccountId = whitelisted_caller();
        assert_ok!(<Pallet<T>>::add_constituent(SystemOrigin::Root.into(), caller.clone()));
        let proposal: pallet::Proposal<T> = submit_proposal::<T>(caller.clone());
        let voters = ["a", "b", "c", "d", "e"];

        // run to voting period
        <System<T>>::set_block_number(<System<T>>::block_number() + <T as Config>::VotingPeriod::get() + <T as Config>::ProposalSubmissionPeriod::get() + 1_u32.into());

        // generate members
        for i in &voters {
            let voter: T::AccountId = account(i, 0, 0);
            <Members<T>>::insert(voter.clone(), MemberType::Council);

            // vote aye
            assert_ok!(<Pallet<T>>::vote(
                SystemOrigin::Signed(voter).into(),
                proposal.hash(),
                Vote::Aye,
            ));
        }

        // run out of voting period
        <System<T>>::set_block_number(
            <System<T>>::block_number()
                + <T as Config>::VotingPeriod::get() * 2_u32.into()
                + <T as Config>::ProposalSubmissionPeriod::get()
                + 1_u32.into()
        );
    }: _(
        SystemOrigin::Signed(caller.clone()),
        proposal.hash()
    ) verify {
        assert_noop!(
            <Pallet<T>>::close(SystemOrigin::Signed(caller.clone()).into(), proposal.hash()),
            <Error<T>>::ProposalAlreadyExecuted
        );
    }

    add_constituent {
        let constituent: T::AccountId = account("constituent", 0, 0);
    }: _(
        SystemOrigin::Root,
        constituent.clone()
    ) verify {
        assert!(<pallet::Members<T>>::contains_key(constituent));
    }
}
