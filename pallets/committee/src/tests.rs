// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok, codec::Encode};
use frame_system as system;
use sp_runtime::traits::BadOrigin;
use std::convert::TryFrom;

const ASHLEY: AccountId = 0;

fn make_action(value: u64) -> Call {
    Call::System(system::Call::remark(value.encode()))
}

#[test]
fn proposer_can_create_a_proposal() {
    new_test_ext().execute_with(|| {
        let action = make_action(123);
        let expected_nonce = 0;

        assert_ok!(Committee::propose(
            Origin::signed(PROPOSER_ACCOUNT_ID),
            Box::new(action.clone())
        ));

        let proposal = pallet::Proposal::<Test>::new(expected_nonce, action);

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
            assert_ok!(Committee::propose(
                Origin::signed(PROPOSER_ACCOUNT_ID),
                Box::new(action.clone())
            ));
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
        assert_ok!(Committee::propose(
            Origin::signed(PROPOSER_ACCOUNT_ID),
            Box::new(make_action(123))
        ));

        assert_noop!(
            Committee::propose(
                Origin::signed(PROPOSER_ACCOUNT_ID),
                Box::new(make_action(123))
            ),
            pallet::Error::<Test>::ProposalNonceExhausted
        );
    });
}
