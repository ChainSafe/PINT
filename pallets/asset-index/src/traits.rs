// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

/// A type to abstract the range of lockup period
pub trait LockupPeriodRange<BlockNumber> {
	/// The minimum value of the lockup period range
	fn min() -> BlockNumber;

	/// The maximum value of the lockup period range
	fn max() -> BlockNumber;
}
