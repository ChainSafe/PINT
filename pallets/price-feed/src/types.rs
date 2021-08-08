// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::{
	pallet_prelude::*,
};

/// Represents an answer of a feed at a certain point of time
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct TimestampedValue<Value, Moment> {
	/// The timestamped value
	pub value: Value,
	/// Timestamp when the answer was first received
	pub moment: Moment,
}