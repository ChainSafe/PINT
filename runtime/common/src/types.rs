// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

/// Origin that approved by committee
pub type GovernanceOrigin<AccountId, Runtime> = frame_system::EnsureOneOf<
	AccountId,
	pallet_committee::EnsureApprovedByCommittee<Runtime>,
	frame_system::EnsureRoot<AccountId>,
>;

/// committee origin only
pub type CommitteeOrigin<Runtime> = pallet_committee::EnsureApprovedByCommittee<Runtime>;
