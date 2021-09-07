// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Fee types used in PINT pallets

use codec::{Decode, Encode};

/// Represents the fee rate where fee_rate = numerator / denominator
#[derive(Debug, Encode, Decode, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct FeeRate {
	pub numerator: u32,
	pub denominator: u32,
}

impl Default for FeeRate {
	fn default() -> Self {
		// 0.3%
		Self { numerator: 3, denominator: 1_000 }
	}
}

pub trait BaseFee
where
	Self: Sized,
{
	/// Returns the given amount after applying the fee rate: `self - fee`
	fn without_fee(&self, rate: FeeRate) -> Option<Self>;

	/// Returns the fees only.
	fn fee(&self, rate: FeeRate) -> Option<Self>;
}

impl BaseFee for u128 {
	fn without_fee(&self, rate: FeeRate) -> Option<Self> {
		self.checked_mul(rate.denominator as Self)?.checked_div(rate.denominator as Self + rate.numerator as Self)
	}

	fn fee(&self, rate: FeeRate) -> Option<Self> {
		self.checked_mul(rate.numerator as Self)?.checked_div(rate.denominator as Self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_fee_calculations() {
		let rate = FeeRate { numerator: 3, denominator: 1_000 };

		assert_eq!(1_003.without_fee(rate), Some(1_000));
		assert_eq!(1_003.fee(rate), Some(3));
	}
}
