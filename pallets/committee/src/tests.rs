// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::{mock::*, CommitteeMember, MemberType, ProposalStatus, VoteAggregate, VoteKind};
use frame_support::{assert_noop, assert_ok, codec::Encode, sp_runtime::traits::BadOrigin};
use frame_system as system;
use std::convert::{TryFrom, TryInto};

const ASHLEY: AccountId = 0;

const ASHLEY_COUNCIL: CommitteeMember<AccountId> =
	CommitteeMember { account_id: ASHLEY, member_type: MemberType::Council };

const ASHLEY_RANGE: std::ops::Range<AccountId> = 0..1;

const EMPTY_RANGE: std::ops::Range<AccountId> = 0..0;

// Start of the first submission period
const START_OF_S1: <Test as system::Config>::BlockNumber = VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD;
// Start of first voting period
const START_OF_V1: <Test as system::Config>::BlockNumber = 2 * VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD;
const CONSTITUENT: u64 = 42;

/// value is used to make unique actions
fn make_action(value: u64) -> Call {
	Call::System(system::Call::remark(value.encode()))
}

fn submit_proposal(action_value: u64) -> pallet::Proposal<Test> {
	let action = make_action(action_value);
	let expected_nonce = pallet::ProposalCount::<Test>::get();
	assert_ok!(Committee::propose(Origin::signed(PROPOSER_ACCOUNT_ID), Box::new(action.clone())));
	pallet::Proposal::<Test>::new(expected_nonce, action, ProposalStatus::Active)
}

//
// Creating a proposal
//

#[test]
fn proposer_can_create_a_proposal() {
	new_test_ext(EMPTY_RANGE).execute_with(|| {
		let proposal = submit_proposal(123);
		assert!(Committee::active_proposals().contains(&proposal.hash()));
		assert!(Committee::get_proposal(&proposal.hash()) == Some(proposal));
	});
}

#[test]
fn non_proposer_cannot_create_a_proposal() {
	new_test_ext(EMPTY_RANGE).execute_with(|| {
		assert_noop!(Committee::propose(Origin::signed(ASHLEY), Box::new(make_action(123))), BadOrigin);
		assert!(Committee::active_proposals().is_empty());
	});
}

#[test]
fn can_create_multiple_proposals_from_same_action() {
	// Each should get a unique nonce and there should be no hash collisions
	new_test_ext(EMPTY_RANGE).execute_with(|| {
		let action = make_action(123);
		let repeats = 3;

		for _ in 0..repeats {
			submit_proposal(123);
		}

		assert!(Committee::active_proposals().len() == repeats);

		for i in 0..repeats {
			let nonce = u32::try_from(i).unwrap();
			let proposal = pallet::Proposal::<Test>::new(nonce, action.clone(), ProposalStatus::Active);
			assert!(Committee::active_proposals().contains(&proposal.hash()));
			assert!(Committee::get_proposal(&proposal.hash()) == Some(proposal));
		}
	});
}

#[test]
fn cannot_exceed_max_nonce() {
	new_test_ext(EMPTY_RANGE).execute_with(|| {
		super::ProposalCount::<Test>::set(<Test as pallet::Config>::ProposalNonce::max_value() - 1);

		// should work, uses last nonce
		submit_proposal(123);

		assert_noop!(
			Committee::propose(Origin::signed(PROPOSER_ACCOUNT_ID), Box::new(make_action(123))),
			pallet::Error::<Test>::ProposalNonceExhausted
		);
	});
}

#[test]
fn upkeep_drops_proposal_from_active_list() {
	new_test_ext(EMPTY_RANGE).execute_with(|| {
		let proposal = submit_proposal(123);

		assert!(Committee::active_proposals().contains(&proposal.hash()));
		run_to_block(START_OF_V1 - 1);
		assert!(Committee::active_proposals().contains(&proposal.hash()));
		run_to_block(START_OF_V1);
		assert!(!Committee::active_proposals().contains(&proposal.hash())); // Dropped
	});
}

//
// Voting on a proposal
//

#[test]
fn non_member_cannot_vote() {
	new_test_ext(EMPTY_RANGE).execute_with(|| {
		let proposal = submit_proposal(123);
		let expected_votes = VoteAggregate::new_with_end(START_OF_V1);
		assert_noop!(
			Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye),
			<pallet::Error<Test>>::NotMember,
		);
		assert_eq!(Committee::get_votes_for(&proposal.hash()), Some(expected_votes));
	});
}

#[test]
fn cannot_vote_for_non_existent_proposal() {
	new_test_ext(ASHLEY_RANGE).execute_with(|| {
		let action = make_action(123);
		let proposal = pallet::Proposal::<Test>::new(0, action, ProposalStatus::Active);
		assert_noop!(
			Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye),
			pallet::Error::<Test>::NoProposalWithHash
		);
	});
}

#[test]
fn member_cannot_vote_before_voting_period() {
	new_test_ext(ASHLEY_RANGE).execute_with(|| {
		let proposal = submit_proposal(123);
		assert_noop!(
			Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye),
			pallet::Error::<Test>::NotInVotingPeriod
		);
	});
}

#[test]
fn member_can_vote_in_voting_period() {
	new_test_ext(ASHLEY_RANGE).execute_with(|| {
		let expected_votes = VoteAggregate::<AccountId, u64>::new(vec![ASHLEY_COUNCIL], vec![], vec![], START_OF_V1);
		let proposal = submit_proposal(123);

		run_to_block(START_OF_S1 - 1);
		// still not in voting period
		assert_noop!(
			Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye),
			pallet::Error::<Test>::NotInVotingPeriod
		);

		run_to_block(START_OF_S1);
		// first block in voting period
		assert_ok!(Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye));
		assert_eq!(Committee::get_votes_for(&proposal.hash()), Some(expected_votes));
	});
}

#[test]
fn member_can_vote_aye() {
	new_test_ext(ASHLEY_RANGE).execute_with(|| {
		let expected_votes = VoteAggregate::<AccountId, u64>::new(vec![ASHLEY_COUNCIL], vec![], vec![], START_OF_V1);
		let proposal = submit_proposal(123);
		run_to_block(START_OF_S1);
		// first block in voting period
		assert_ok!(Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye));
		assert_eq!(Committee::get_votes_for(&proposal.hash()), Some(expected_votes));
	});
}

#[test]
fn member_can_vote_nay() {
	new_test_ext(ASHLEY_RANGE).execute_with(|| {
		let expected_votes = VoteAggregate::<AccountId, u64>::new(vec![], vec![ASHLEY_COUNCIL], vec![], START_OF_V1);
		let proposal = submit_proposal(123);
		run_to_block(START_OF_S1);
		assert_ok!(Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Nay));
		assert_eq!(Committee::get_votes_for(&proposal.hash()), Some(expected_votes));
	});
}

#[test]
fn member_can_vote_abstain() {
	new_test_ext(ASHLEY_RANGE).execute_with(|| {
		let expected_votes = VoteAggregate::<AccountId, u64>::new(vec![], vec![], vec![ASHLEY_COUNCIL], START_OF_V1);
		let proposal = submit_proposal(123);
		run_to_block(START_OF_S1);
		assert_ok!(Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Abstain));
		assert_eq!(Committee::get_votes_for(&proposal.hash()), Some(expected_votes));
	});
}

#[test]
fn member_cannot_vote_after_voting_period() {
	new_test_ext(ASHLEY_RANGE).execute_with(|| {
		let proposal = submit_proposal(123);

		run_to_block(START_OF_V1 - 1);
		// last block in voting period
		assert_ok!(Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye));

		run_to_block(START_OF_V1);
		// first block after voting period
		assert_noop!(
			Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye),
			pallet::Error::<Test>::NotInVotingPeriod
		);
	});
}

#[test]
fn member_cannot_vote_multiple_times() {
	new_test_ext(ASHLEY_RANGE).execute_with(|| {
		let proposal = submit_proposal(123);
		let expected_votes = VoteAggregate::<AccountId, u64>::new(vec![ASHLEY_COUNCIL], vec![], vec![], START_OF_V1);

		run_to_block(START_OF_S1);
		assert_ok!(Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye));
		assert_noop!(
			Committee::vote(Origin::signed(ASHLEY), proposal.hash(), VoteKind::Aye),
			pallet::Error::<Test>::DuplicateVote
		);
		assert_eq!(Committee::get_votes_for(&proposal.hash()), Some(expected_votes));
	});
}

//
// Closing/executing a proposal
//

// iterates through accounts and vote a particular way on a proposal
fn vote_with_each<I>(accounts: I, proposal_hash: <Test as system::Config>::Hash, vote: VoteKind)
where
	I: IntoIterator<Item = AccountId>,
{
	for account in accounts {
		assert_ok!(Committee::vote(Origin::signed(account), proposal_hash, vote.clone()));
	}
}

// add a number of new constituent members
fn add_constituents<I>(accounts: I)
where
	I: IntoIterator<Item = AccountId>,
{
	for m in accounts.into_iter() {
		<pallet::Members<Test>>::insert(m, MemberType::Constituent);
	}
}

#[test]
fn non_execution_origin_cannot_close() {
	new_test_ext(0..4).execute_with(|| {
		let non_execution_origin = 5;
		let proposal = submit_proposal(123);
		run_to_block(START_OF_S1);

		vote_with_each(0..4, proposal.hash(), VoteKind::Aye);

		run_to_block(START_OF_V1 + 1);
		assert_noop!(Committee::close(Origin::signed(non_execution_origin), proposal.hash()), BadOrigin);
	});
}

#[test]
fn cannot_close_until_voting_period_elapsed() {
	new_test_ext(0..4).execute_with(|| {
		let proposal = submit_proposal(123);

		run_to_block(START_OF_S1);
		vote_with_each(0..4, proposal.hash(), VoteKind::Aye);

		assert_noop!(
			Committee::close(Origin::signed(EXECUTER_ACCOUNT_ID), proposal.hash()),
			pallet::Error::<Test>::VotingPeriodNotElapsed
		);
	});
}

#[test]
fn cannot_close_if_insufficent_council_votes() {
	new_test_ext(0..4).execute_with(|| {
		let proposal = submit_proposal(123);

		run_to_block(START_OF_S1);
		vote_with_each(0..(MIN_COUNCIL_VOTES - 1).try_into().unwrap(), proposal.hash(), VoteKind::Aye);

		run_to_block(START_OF_V1 + 1);
		assert_noop!(
			Committee::close(Origin::signed(EXECUTER_ACCOUNT_ID), proposal.hash()),
			pallet::Error::<Test>::ProposalNotAcceptedInsufficientVotes
		);
	});
}

#[test]
fn cannot_close_if_council_rejects() {
	new_test_ext(0..4).execute_with(|| {
		let proposal = submit_proposal(123);

		run_to_block(START_OF_S1);
		vote_with_each(0..(MIN_COUNCIL_VOTES).try_into().unwrap(), proposal.hash(), VoteKind::Nay);

		run_to_block(START_OF_V1 + 1);
		assert_noop!(
			Committee::close(Origin::signed(EXECUTER_ACCOUNT_ID), proposal.hash()),
			pallet::Error::<Test>::ProposalNotAcceptedCouncilDeny
		);
	});
}

#[test]
fn cannot_close_if_constituents_veto() {
	new_test_ext(0..4).execute_with(|| {
		add_constituents(4..8);

		let proposal = submit_proposal(123);

		run_to_block(START_OF_S1);
		vote_with_each(0..4, proposal.hash(), VoteKind::Aye);
		vote_with_each(4..8, proposal.hash(), VoteKind::Nay);

		run_to_block(START_OF_V1 + 1);
		assert_noop!(
			Committee::close(Origin::signed(EXECUTER_ACCOUNT_ID), proposal.hash()),
			pallet::Error::<Test>::ProposalNotAcceptedConstituentVeto
		);
	});
}

#[test]
fn executer_can_close_if_voted_for_and_voting_period_elapsed() {
	new_test_ext(0..4).execute_with(|| {
		let proposal = submit_proposal(123);

		run_to_block(START_OF_S1);
		vote_with_each(0..4, proposal.hash(), VoteKind::Aye);

		run_to_block(START_OF_V1 + 1);
		assert_ok!(Committee::close(Origin::signed(EXECUTER_ACCOUNT_ID), proposal.hash()));
	});
}

#[test]
fn cannot_execute_proposal_twice() {
	new_test_ext(0..4).execute_with(|| {
		let proposal = submit_proposal(123);

		run_to_block(START_OF_S1);
		vote_with_each(0..4, proposal.hash(), VoteKind::Aye);

		run_to_block(START_OF_V1 + 1);
		assert_ok!(Committee::close(Origin::signed(EXECUTER_ACCOUNT_ID), proposal.hash()));
		assert_noop!(
			Committee::close(Origin::signed(EXECUTER_ACCOUNT_ID), proposal.hash()),
			pallet::Error::<Test>::ProposalAlreadyExecuted
		);
	});
}

#[test]
fn cannot_execute_proposal_after_closed() {
	new_test_ext(0..4).execute_with(|| {
		let proposal = submit_proposal(123);
		run_to_block(START_OF_S1 + VOTING_PERIOD * 2);

		assert_noop!(
			Committee::close(Origin::signed(EXECUTER_ACCOUNT_ID), proposal.hash()),
			pallet::Error::<Test>::ProposalAlreadyClosed
		);
	});
}

//
// Constituent Committee Council Selection
//

#[test]
fn cannot_add_constituent_if_already_is_council() {
	new_test_ext_without_members().execute_with(|| {
		assert_noop!(
			Committee::add_constituent(Origin::root(), PROPOSER_ACCOUNT_ID),
			<pallet::Error<Test>>::AlreadyCouncilMember
		);
	});
}

#[test]
fn cannot_add_constituent_if_already_is_constituent() {
	new_test_ext_without_members().execute_with(|| {
		assert_ok!(Committee::add_constituent(Origin::root(), CONSTITUENT));
		assert_noop!(
			Committee::add_constituent(Origin::root(), CONSTITUENT),
			<pallet::Error<Test>>::AlreadyConstituentMember
		);
	});
}

#[test]
fn can_remove_member() {
	new_test_ext(0..4).execute_with(|| {
		let another_constituent = CONSTITUENT + 1;

		// adds two constituents
		assert_ok!(Committee::add_constituent(Origin::root(), CONSTITUENT));
		assert_eq!(pallet::Members::<Test>::get(CONSTITUENT), Some(MemberType::Constituent));
		assert_ok!(Committee::add_constituent(Origin::root(), another_constituent));
		assert_eq!(pallet::Members::<Test>::get(another_constituent), Some(MemberType::Constituent));

		// cannot remove account which is not a member
		assert_noop!(Committee::remove_member(Origin::root(), 4), pallet::Error::<Test>::NotMember);

		// can remove constituent
		assert_ok!(Committee::remove_member(Origin::root(), CONSTITUENT));
		assert_eq!(pallet::Members::<Test>::get(CONSTITUENT), None);

		// can remove council
		assert_eq!(pallet::Members::<Test>::get(3), Some(MemberType::Council));
		assert_ok!(Committee::remove_member(Origin::root(), 3));
		assert_eq!(pallet::Members::<Test>::get(3), None);

		// can remove constituent again
		assert_ok!(Committee::remove_member(Origin::root(), another_constituent));
		assert_eq!(pallet::Members::<Test>::get(another_constituent), None);

		// cannot remove council again
		assert_eq!(pallet::Members::<Test>::get(2), Some(MemberType::Council));
		assert_noop!(Committee::remove_member(Origin::root(), 2), pallet::Error::<Test>::MinimalCouncilMembers);
	});
}

#[test]
fn propose_constituent_works() {
	new_test_ext(PROPOSER_ACCOUNT_ID..PROPOSER_ACCOUNT_ID + 4).execute_with(|| {
		System::set_block_number(1);

		// propose a new constituent
		assert_ok!(Committee::propose(
			Origin::signed(PROPOSER_ACCOUNT_ID),
			Box::new(Call::Committee(crate::Call::add_constituent(CONSTITUENT)))
		));

		// test if proposal submitted with event
		if let Event::Committee(crate::Event::Proposed(_, _, hash)) = last_event() {
			assert_eq!(&[hash], Committee::active_proposals().as_slice());

			// vote Aye on adding new constituent
			run_to_block(START_OF_S1);
			vote_with_each(PROPOSER_ACCOUNT_ID..PROPOSER_ACCOUNT_ID + 4, hash, VoteKind::Aye);

			// close proposal
			run_to_block(START_OF_V1 + 1);
			assert_ok!(Committee::close(Origin::signed(EXECUTER_ACCOUNT_ID), hash,));
		} else {
			panic!("Could not get proposal hash from events");
		}

		// check if constituent committee contains new constituent
		assert!(<pallet::Members<Test>>::contains_key(CONSTITUENT));
	});
}
