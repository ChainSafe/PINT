// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;
use frame_support::assert_ok;

#[test]
fn feed_creation_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ChainlinkFeed::create_feed(
            Origin::signed(1),
            20,                           // payment
            10,                           // timeout
            (10, 1_000),                  // value range
            3,                            // min values
            5,                            // decimals
            b"desc".to_vec(),             // desc
            2,                            // restart delay
            vec![(1, 4), (2, 4), (3, 4)], // oracles
        ));
    });
}
