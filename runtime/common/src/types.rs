// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use sp_std::{cmp::Ordering, marker::PhantomData};

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

/// Identifier for an asset.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, Clone, Ord, Copy, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub enum AssetId {
	Invalid,
	PINT,
	Liquid(u32),
	SAFT(u32),
}

impl Default for AssetId {
	fn default() -> Self {
		AssetId::Invalid
	}
}

impl From<u32> for AssetId {
	fn from(other: u32) -> Self {
		AssetId::Liquid(other)
	}
}

impl PartialOrd for AssetId {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		// match (&self, other) {
		//
		// }
		None
	}
}
