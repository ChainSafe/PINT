// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Fee types used in PINT pallets

use codec::{Decode, Encode};
use frame_support::sp_runtime::traits::AtLeast32Bit;

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

/// Determines the fee upon index token redemptions from range
#[derive(Clone, Decode, Debug, Default, Encode, PartialEq, Eq)]
pub struct RedemptionFeeRange<BlockNumber> {
	pub range: [(BlockNumber, FeeRate); 2],
	pub default_fee: FeeRate,
}

impl<BlockNumber: AtLeast32Bit> RedemptionFeeRange<BlockNumber> {
	/// get fee rate by spent time
	fn get_rate(&self, spent_time: BlockNumber) -> FeeRate {
		if spent_time < self.range[0].0 {
			self.range[0].1
		} else if spent_time <= self.range[1].0 {
			self.range[1].1
		} else {
			self.default_fee
		}
	}

	/// Determines the redemption fee based on how long the given amount were held in the index
	///
	/// Parameters:
	///     - `time_spent`: The number of blocks the amount were held in the index. This is `current
	///       block -  deposit`.
	///     - `amount`: The amount of index tokens withdrawn
	pub fn redemption_fee<Balance: BaseFee>(&self, time_spent: BlockNumber, amount: Balance) -> Option<Balance> {
		amount.fee(self.get_rate(time_spent))
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
