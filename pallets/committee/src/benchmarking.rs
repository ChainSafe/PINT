// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{account, benchmarks, vec, Box};
use frame_support::{
	assert_noop, assert_ok,
	traits::{EnsureOrigin, Get, Hooks, UnfilteredDispatchable},
};
use frame_system::{ensure_signed, Call as SystemCall, Pallet as System, RawOrigin as SystemOrigin};

fn submit_proposal<T: Config>(origin: <T as frame_system::Config>::Origin) -> pallet::Proposal<T> {
	let action: T::Action = <SystemCall<T>>::remark(vec![0; 0]).into();
	let expected_nonce = pallet::ProposalCount::<T>::get();

	let account_id = ensure_signed(origin.clone()).unwrap();
	assert_ok!(<Pallet<T>>::add_constituent(SystemOrigin::Root.into(), account_id.clone()));
	<System<T>>::set_block_number(
		<System<T>>::block_number() +
			<T as Config>::VotingPeriod::get() +
			<T as Config>::ProposalSubmissionPeriod::get() +
			1_u32.into(),
	);

	let call = <Call<T>>::propose(Box::new(action.clone()));
	assert_ok!(call.dispatch_bypass_filter(origin));

	pallet::Proposal::<T>::new(action, account_id, expected_nonce, ProposalStatus::Active)
}

fn run_to_block<T: Config>(n: T::BlockNumber) {
	while System::<T>::block_number() < n {
		System::<T>::set_block_number(System::<T>::block_number() + 1u32.into());
		Pallet::<T>::on_initialize(System::<T>::block_number());
	}
}

benchmarks! {
	propose {
		let origin = T::ProposalSubmissionOrigin::successful_origin();
		let proposal = submit_proposal::<T>(origin.clone());
		let call = <Call<T>>::propose(Box::new(<SystemCall<T>>::remark(vec![0; 0]).into()));
	}: {
		call.dispatch_bypass_filter(origin)?
	} verify {
		assert!(<Pallet<T>>::get_proposal(&proposal.hash()) == Some(proposal));
	}

	vote {
		let origin = T::ProposalSubmissionOrigin::successful_origin();
		let proposal = submit_proposal::<T>(origin.clone());

		// run to voting period
		<System<T>>::set_block_number(
			<System<T>>::block_number()
				+ <T as Config>::VotingPeriod::get()
				+ <T as Config>::ProposalSubmissionPeriod::get() + 1_u32.into(),
		);

		// construct call
		let call = <Call<T>>::vote(proposal.hash(), VoteKind::Abstain);
	}: {
		call.dispatch_bypass_filter(origin)?
	} verify {
		assert_eq!(
			<Pallet<T>>::get_votes_for(&proposal.hash()).unwrap().votes.len(),
			1,
		);
	}

	close {
		let proposal: pallet::Proposal<T> = submit_proposal::<T>(T::ProposalSubmissionOrigin::successful_origin());

		// vote
		for i in 0..5 {
			let voter: T::AccountId = account("voter", i, 0);
			assert_ok!(Votes::<T>::try_mutate(&proposal.hash(), |votes| {
				if let Some(votes) = votes {
					votes.cast_vote(
						MemberVote::new(CommitteeMember::new(voter, MemberType::Council), VoteKind::Aye),
					);
					Ok(())
				} else {
					Err(Error::<T>::NoProposalWithHash)
				}
			}));
		}

		// run out of voting period
		<System<T>>::set_block_number(
			<System<T>>::block_number()
				+ <T as Config>::VotingPeriod::get() * 2_u32.into()
				+ <T as Config>::ProposalSubmissionPeriod::get()
				+ 1_u32.into()
		);

		// construct call
		let call = <Call<T>>::close(proposal.hash());
	}: {
		call.dispatch_bypass_filter(T::ProposalExecutionOrigin::successful_origin())?
	} verify {
		assert_noop!(
			<Pallet<T>>::close(T::ProposalExecutionOrigin::successful_origin(), proposal.hash()),
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

	remove_member {
		let constituent: T::AccountId = account("constituent", 0, 0);
		assert_ok!(<Pallet<T>>::add_constituent(SystemOrigin::Root.into(), constituent.clone()));
	}: _(
		SystemOrigin::Root,
		constituent.clone()
	) verify {
		assert!(!<pallet::Members<T>>::contains_key(constituent));
	}

	set_voting_period {
		let two_weeks: T::BlockNumber = (10u32 * 60 * 24 * 7 * 2).into();
	}: _(
		SystemOrigin::Root,
		two_weeks
	) verify {
		run_to_block::<T>(<T as Config>::VotingPeriod::get());
		assert_eq!(pallet::VotingPeriod::<T>::get(), two_weeks);
	}
}
