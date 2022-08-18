// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use crate::prelude::*;
use cumulus_primitives_core::ParaId;
use frame_support::{
	assert_ok,
	sp_runtime::traits::{AccountIdConversion, Zero},
	traits::tokens::fungibles::Inspect,
};
use orml_traits::MultiCurrency;
use pallet_committee::{
	types::{CommitteeMember, MemberType, MemberVote, VoteAggregate, VoteKind},
	CommitteeOrigin,
};
use xcm_emulator::TestExt;
use xcm_executor::traits::Convert;

pub fn sibling_sovereign_account() -> AccountId {
	use crate::statemint::LocationToAccountId;
	LocationToAccountId::convert(MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(PARA_ID)) })
		.expect("Failed to convert para")
}

pub fn relay_sovereign_account() -> AccountId {
	let para: ParaId = PARA_ID.into();
	para.into_account_truncating()
}

/// registers the relay chain as liquid asset
pub fn register_relay() {
	// prepare index fund so NAV is available
	let deposit = 1_000;
	assert_ok!(orml_tokens::Pallet::<ShotRuntime>::deposit(RELAY_CHAIN_ASSET, &ADMIN_ACCOUNT, 1_000));
	assert_ok!(pallet_asset_index::Pallet::<ShotRuntime>::register_asset(
		committee_origin(ADMIN_ACCOUNT).into(),
		RELAY_CHAIN_ASSET,
		AssetAvailability::Liquid(MultiLocation::parent()),
	));
	assert_ok!(pallet_asset_index::Pallet::<ShotRuntime>::add_asset(
		committee_origin(ADMIN_ACCOUNT).into(),
		RELAY_CHAIN_ASSET,
		deposit,
		deposit
	));
	assert!(pallet_asset_index::Pallet::<ShotRuntime>::is_liquid_asset(&RELAY_CHAIN_ASSET));
}

/// transfer the given amount of relay chain currency into the account on the
/// parachain
pub fn transfer_to_para(relay_deposit_amount: Balance, who: AccountId) {
	Kusama::execute_with(|| {
		// transfer from relay to parachain
		assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
			kusama_runtime::Origin::signed(who.clone()),
			Box::new(VersionedMultiLocation::V1(Junctions::X1(Junction::Parachain(PARA_ID)).into())),
			Box::new(VersionedMultiLocation::V1(
				Junctions::X1(Junction::AccountId32 { network: NetworkId::Any, id: who.clone().into() }).into()
			)),
			Box::new(VersionedMultiAssets::V1((Junctions::Here, relay_deposit_amount).into())),
			0,
		));
	});
	Shot::execute_with(|| {
		// ensure deposit arrived
		assert_eq!(orml_tokens::Pallet::<ShotRuntime>::balance(RELAY_CHAIN_ASSET, &who), relay_deposit_amount);
	});
}

/// create an approved committe origin from account id
pub fn committee_origin(origin: AccountId) -> CommitteeOrigin<AccountId, BlockNumber> {
	CommitteeOrigin::ApprovedByCommittee(
		origin,
		VoteAggregate {
			votes: vec![
				MemberVote {
					member: CommitteeMember { account_id: Default::default(), member_type: MemberType::Council },
					vote: VoteKind::Aye
				};
				<ShotRuntime as pallet_committee::Config>::MinCouncilVotes::get() + 1
			],
			end: <frame_system::Pallet<ShotRuntime>>::block_number() + 1,
		},
	)
}
