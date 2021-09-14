// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{account, benchmarks, vec, Box};
use frame_support::{
	assert_noop, assert_ok,
	traits::{EnsureOrigin, Get, UnfilteredDispatchable},
};
use frame_system::{ensure_signed, Call as SystemCall, Pallet as System, RawOrigin as SystemOrigin};

fn submit_proposal<T: Config>(origin: <T as frame_system::Config>::Origin) -> pallet::Proposal<T> {
	let action: T::Action = <SystemCall<T>>::remark(vec![0; 0]).into();
	let expected_nonce = pallet::ProposalCount::<T>::get();
	assert_ok!(<Pallet<T>>::add_constituent(SystemOrigin::Root.into(), ensure_signed(origin.clone()).unwrap(),));
	let call = <Call<T>>::propose(Box::new(action.clone()));
	assert_ok!(call.dispatch_bypass_filter(origin));
	pallet::Proposal::<T>::new(expected_nonce, action)
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
}
