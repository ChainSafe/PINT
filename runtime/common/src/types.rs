// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use sp_std::marker::PhantomData;
use frame_support::{sp_runtime, traits::EnsureOneOf, weights::{WeightToFee, Weight}};
use primitives::Balance;
use crate::constants::TransactionByteFee;
use sp_runtime::SaturatedConversion;

/// Origin either `Root` or `CommitteeOrigin`
pub type GovernanceOrigin<AccountId, Runtime> = EnsureOneOf<
	pallet_committee::EnsureApprovedByCommittee<Runtime>,
	frame_system::EnsureRoot<AccountId>,
>;

/// Origin that approved by committee
pub type CommitteeOrigin<Runtime> = pallet_committee::EnsureApprovedByCommittee<Runtime>;

/// Range of voting period
pub struct VotingPeriodRange<T>(PhantomData<T>);

impl<T: frame_system::Config> pallet_committee::traits::VotingPeriodRange<T::BlockNumber> for VotingPeriodRange<T> {
	fn max() -> T::BlockNumber {
		(crate::constants::DAYS * 28).into()
	}

	fn min() -> T::BlockNumber {
		(crate::constants::DAYS * 7).into()
	}
}

/// Range of lockup period
pub struct LockupPeriodRange<T>(PhantomData<T>);

impl<T: frame_system::Config> pallet_asset_index::traits::LockupPeriodRange<T::BlockNumber> for LockupPeriodRange<T> {
	fn min() -> T::BlockNumber {
		crate::constants::DAYS.into()
	}

	fn max() -> T::BlockNumber {
		(crate::constants::DAYS * 28).into()
	}
}

pub struct CustomLengthToFee {}

impl CustomLengthToFee {
	const LENGTH_FEE: Balance = TransactionByteFee::get();
}

impl WeightToFee for CustomLengthToFee {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		Self::Balance::saturated_from(*weight)
			.saturating_mul(CustomLengthToFee::LENGTH_FEE)
	}
}