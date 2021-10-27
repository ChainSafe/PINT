// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use crate::constants::DAYS;
use primitives::{Balance, BlockNumber};
use sp_std::marker::PhantomData;

/// Origin either `Root` or `CommitteeOrigin`
pub type GovernanceOrigin<AccountId, Runtime> = frame_system::EnsureOneOf<
	AccountId,
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

/// Redemption fee
pub struct RedemptionFee;

impl primitives::traits::RedemptionFee<BlockNumber, Balance> for RedemptionFee {
	fn redemption_fee(time_spent: BlockNumber, amount: Balance) -> Balance {
		if time_spent < 7 * DAYS {
			amount.checked_div(10).unwrap_or_default()
		} else if time_spent < 30 * DAYS {
			amount.checked_div(20).unwrap_or_default()
		} else {
			amount.checked_div(100).unwrap_or_default()
		}
	}
}
