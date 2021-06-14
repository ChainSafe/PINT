// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, One};

// Proposal submissions and voting follow set cycles e.g.
//
// ---v0--||-----s1----|--v1--||-----s2----|--v2--|...
//
// Proposals submitted in s1 are voted upon in v1 and after that
// they may or may not be executed and then are dropped from the ActiveProposals set.
//
// Proposals submitted during v0 fall into the next submission period and
// should be voted on in v1. To simplify implementation we assume the cycle begins with
// an initial dummy voting period.
//
// Will return an None if any of the arithmetic operations fail due to overflow/underflow
//
pub fn get_vote_end<T: CheckedAdd + CheckedMul + CheckedDiv + One>(
    current_block: &T,
    voting_period: &T,
    proposal_period: &T,
) -> Option<T> {
    let epoch_period = voting_period.checked_add(proposal_period)?;

    // [(current_block // period) + 1] * period + voting_period
    // return the block at the end of the next voting period after the current one
    current_block
        .checked_div(&epoch_period)?
        .checked_add(&T::one())?
        .checked_mul(&epoch_period)?
        .checked_add(&voting_period)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter;

    const VOTE_P: i32 = 2;
    const PROPOSAL_P: i32 = 3;

    #[test]
    // A proposal made during the start dummpy period must have votes submitted before
    // the end of v1
    fn test_proposal_in_v0() {
        assert_eq!(
            get_vote_end(&0, &VOTE_P, &PROPOSAL_P),
            Some(VOTE_P + PROPOSAL_P + VOTE_P)
        )
    }

    #[test]
    // A proposal made during s1 must have votes submitted before
    // the end of v1
    fn test_proposal_in_s1() {
        assert_eq!(
            get_vote_end(&4, &VOTE_P, &PROPOSAL_P),
            Some(VOTE_P + PROPOSAL_P + VOTE_P)
        )
    }

    #[test]
    // A proposal made during v1 must have votes submitted before
    // the end of v2
    fn test_proposal_in_v1() {
        assert_eq!(
            get_vote_end(&9, &VOTE_P, &PROPOSAL_P),
            Some(VOTE_P + 2 * (PROPOSAL_P + VOTE_P))
        )
    }

    #[test]
    // Check a range of blocks are as expected
    fn test_proposal_range() {
        let result: Vec<i32> = (0..15)
            .map(|i| get_vote_end(&i, &VOTE_P, &PROPOSAL_P).unwrap())
            .collect();

        let expected: Vec<i32> = iter::empty()
            .chain(iter::repeat(VOTE_P + PROPOSAL_P + VOTE_P).take(5))
            .chain(iter::repeat(VOTE_P + 2 * (PROPOSAL_P + VOTE_P)).take(5))
            .chain(iter::repeat(VOTE_P + 3 * (PROPOSAL_P + VOTE_P)).take(5))
            .collect();

        assert_eq!(result, expected)
    }
}
