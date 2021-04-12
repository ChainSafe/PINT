// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;
use crate::{Vote, VoteAggregate};
use frame_support::{assert_noop, assert_ok, codec::Encode, traits::InitializeMembers};
use frame_system as system;
use sp_runtime::traits::BadOrigin;
use std::convert::TryFrom;

const ASHLEY: AccountId = 0;

/// value is used to make unique actions
fn make_action(value: u64) -> Call {
    Call::System(system::Call::remark(value.encode()))
}

fn submit_proposal(action_value: u64) -> pallet::Proposal<Test> {
    let action = make_action(action_value);
    let expected_nonce = pallet::ProposalCount::<Test>::get();
    assert_ok!(Committee::propose(
        Origin::signed(PROPOSER_ACCOUNT_ID),
        Box::new(action.clone())
    ));
    pallet::Proposal::<Test>::new(expected_nonce, action)
}

//
// Creating a proposal
//

#[test]
fn proposer_can_create_a_proposal() {
    new_test_ext().execute_with(|| {
        let proposal = submit_proposal(123);
        assert!(Committee::active_proposals().contains(&proposal.hash()));
        assert!(Committee::get_proposal(&proposal.hash()) == Some(proposal));
    });
}

#[test]
fn non_proposer_cannot_create_a_proposal() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Committee::propose(Origin::signed(ASHLEY), Box::new(make_action(123))),
            BadOrigin
        );
        assert!(Committee::active_proposals().is_empty());
    });
}

#[test]
fn can_create_multiple_proposals_from_same_action() {
    // Each should get a unique nonce and there should be no hash collisions
    new_test_ext().execute_with(|| {
        let action = make_action(123);
        let repeats = 3;

        for _ in 0..repeats {
            submit_proposal(123);
        }

        assert!(Committee::active_proposals().len() == repeats);

        for i in 0..repeats {
            let nonce = u32::try_from(i).unwrap();
            let proposal = pallet::Proposal::<Test>::new(nonce, action.clone());
            assert!(Committee::active_proposals().contains(&proposal.hash()));
            assert!(Committee::get_proposal(&proposal.hash()) == Some(proposal));
        }
    });
}

#[test]
fn cannot_exceed_max_nonce() {
    new_test_ext().execute_with(|| {
        super::ProposalCount::<Test>::set(<Test as pallet::Config>::ProposalNonce::max_value() - 1);

        // should work, uses last nonce
        submit_proposal(123);

        assert_noop!(
            Committee::propose(
                Origin::signed(PROPOSER_ACCOUNT_ID),
                Box::new(make_action(123))
            ),
            pallet::Error::<Test>::ProposalNonceExhausted
        );
    });
}

#[test]
fn upkeep_drops_proposal_from_active_list() {
    new_test_ext().execute_with(|| {
        let proposal = submit_proposal(123);

        assert!(Committee::active_proposals().contains(&proposal.hash()));
        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD + VOTING_PERIOD - 1);
        assert!(Committee::active_proposals().contains(&proposal.hash()));
        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD + VOTING_PERIOD);
        assert!(!Committee::active_proposals().contains(&proposal.hash())); // Dropped
    });
}

//
// Voting on a proposal
//

#[test]
fn non_member_cannot_vote() {
    new_test_ext().execute_with(|| {
        let proposal = submit_proposal(123);
        let expected_votes =
            VoteAggregate::new_with_end(2 * VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD);
        assert_noop!(
            Committee::vote(Origin::signed(ASHLEY), proposal.hash(), Vote::Aye),
            pallet::Error::<Test>::NotMember
        );
        assert_eq!(
            Committee::get_votes_for(&proposal.hash()),
            Some(expected_votes)
        );
    });
}

#[test]
fn cannot_vote_for_non_existent_proposal() {
    new_test_ext().execute_with(|| {
        Committee::initialize_members(&[ASHLEY]);
        let action = make_action(123);
        let proposal = pallet::Proposal::<Test>::new(0, action);
        assert_noop!(
            Committee::vote(Origin::signed(ASHLEY), proposal.hash(), Vote::Aye),
            pallet::Error::<Test>::NoProposalWithHash
        );
    });
}

#[test]
fn member_cannot_vote_before_voting_period() {
    new_test_ext().execute_with(|| {
        Committee::initialize_members(&[ASHLEY]);
        let proposal = submit_proposal(123);
        assert_noop!(
            Committee::vote(Origin::signed(ASHLEY), proposal.hash(), Vote::Aye),
            pallet::Error::<Test>::NotInVotingPeriod
        );
    });
}

#[test]
fn member_can_vote_in_voting_period() {
    new_test_ext().execute_with(|| {
        Committee::initialize_members(&[ASHLEY]);
        let expected_votes = VoteAggregate::<AccountId, u64>::new(
            vec![ASHLEY],
            vec![],
            vec![],
            2 * VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD,
        );
        let proposal = submit_proposal(123);

        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD - 1);
        // still not in voting period
        assert_noop!(
            Committee::vote(Origin::signed(ASHLEY), proposal.hash(), Vote::Aye),
            pallet::Error::<Test>::NotInVotingPeriod
        );

        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD);
        // first block in voting period
        assert_ok!(Committee::vote(
            Origin::signed(ASHLEY),
            proposal.hash(),
            Vote::Aye
        ));
        assert_eq!(
            Committee::get_votes_for(&proposal.hash()),
            Some(expected_votes)
        );
    });
}

#[test]
fn member_can_vote_aye() {
    new_test_ext().execute_with(|| {
        Committee::initialize_members(&[ASHLEY]);
        let expected_votes = VoteAggregate::<AccountId, u64>::new(
            vec![ASHLEY],
            vec![],
            vec![],
            2 * VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD,
        );
        let proposal = submit_proposal(123);
        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD);
        // first block in voting period
        assert_ok!(Committee::vote(
            Origin::signed(ASHLEY),
            proposal.hash(),
            Vote::Aye
        ));
        assert_eq!(
            Committee::get_votes_for(&proposal.hash()),
            Some(expected_votes)
        );
    });
}

#[test]
fn member_can_vote_nay() {
    new_test_ext().execute_with(|| {
        Committee::initialize_members(&[ASHLEY]);
        let expected_votes = VoteAggregate::<AccountId, u64>::new(
            vec![],
            vec![ASHLEY],
            vec![],
            2 * VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD,
        );
        let proposal = submit_proposal(123);
        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD);
        assert_ok!(Committee::vote(
            Origin::signed(ASHLEY),
            proposal.hash(),
            Vote::Nay
        ));
        assert_eq!(
            Committee::get_votes_for(&proposal.hash()),
            Some(expected_votes)
        );
    });
}

#[test]
fn member_can_vote_abstain() {
    new_test_ext().execute_with(|| {
        Committee::initialize_members(&[ASHLEY]);
        let expected_votes = VoteAggregate::<AccountId, u64>::new(
            vec![],
            vec![],
            vec![ASHLEY],
            2 * VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD,
        );
        let proposal = submit_proposal(123);
        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD);
        assert_ok!(Committee::vote(
            Origin::signed(ASHLEY),
            proposal.hash(),
            Vote::Abstain
        ));
        assert_eq!(
            Committee::get_votes_for(&proposal.hash()),
            Some(expected_votes)
        );
    });
}

#[test]
fn member_cannot_vote_after_voting_period() {
    new_test_ext().execute_with(|| {
        Committee::initialize_members(&[ASHLEY]);
        let proposal = submit_proposal(123);

        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD + VOTING_PERIOD - 1);
        // last block in voting period
        assert_ok!(Committee::vote(
            Origin::signed(ASHLEY),
            proposal.hash(),
            Vote::Aye
        ));

        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD + VOTING_PERIOD);
        // first block after voting period
        assert_noop!(
            Committee::vote(Origin::signed(ASHLEY), proposal.hash(), Vote::Aye),
            pallet::Error::<Test>::NotInVotingPeriod
        );
    });
}

#[test]
fn member_cannot_vote_multiple_times() {
    new_test_ext().execute_with(|| {
        Committee::initialize_members(&[ASHLEY]);
        let proposal = submit_proposal(123);
        let expected_votes = VoteAggregate::<AccountId, u64>::new(
            vec![ASHLEY],
            vec![],
            vec![],
            2 * VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD,
        );

        run_to_block(VOTING_PERIOD + PROPOSAL_SUBMISSION_PERIOD);
        assert_ok!(Committee::vote(
            Origin::signed(ASHLEY),
            proposal.hash(),
            Vote::Aye
        ));
        assert_noop!(
            Committee::vote(Origin::signed(ASHLEY), proposal.hash(), Vote::Aye),
            pallet::Error::<Test>::DuplicateVote
        );
        assert_eq!(
            Committee::get_votes_for(&proposal.hash()),
            Some(expected_votes)
        );
    });
}
