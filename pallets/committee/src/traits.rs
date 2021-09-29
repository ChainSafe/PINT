// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

/// A type to abstract the range of voting period
pub trait VotingPeriodRange<BlockNumber> {
	/// The minimum value of the voting period range
	fn min() -> BlockNumber;

	/// The maximum value of the voting period range
	fn max() -> BlockNumber;
}
