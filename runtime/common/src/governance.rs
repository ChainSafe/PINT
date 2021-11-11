// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Governance related commonly used config types

use frame_system::{EnsureOneOf, EnsureRoot};
use primitives::AccountId;
use sp_core::u32_trait::{_1, _2};

pub type CommitteeInstance = pallet_collective::Instance1;
pub type CouncilInstance = pallet_collective::Instance2;

pub type CouncilMembershipInstance = pallet_membership::Instance1;
pub type ConstituentMembershipInstance = pallet_membership::Instance2;

// General Council
pub type EnsureRootOrAllGeneralCouncil = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilInstance>,
>;

pub type EnsureRootOrHalfGeneralCouncil = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilInstance>,
>;
