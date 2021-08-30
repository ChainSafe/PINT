// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

/// Origin that approved by committee
pub type EnsureApprovedByCommittee<AccountId, Runtime> = frame_system::EnsureOneOf<
	AccountId,
	frame_system::EnsureRoot<AccountId>,
	pallet_committee::EnsureApprovedByCommittee<Runtime>,
>;
