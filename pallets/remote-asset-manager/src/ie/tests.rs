// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use crate::ie::{
	pint::{self, MockPriceFeed, Runtime as PintRuntime},
	relay::{self, ProxyType as RelayProxyType, Runtime as RelayRuntime},
	relay_sovereign_account, sibling_sovereign_account, statemint,
	types::*,
	Net, Pint, Relay, Statemint, ADMIN_ACCOUNT, ALICE, INITIAL_BALANCE, PARA_ID, RELAY_CHAIN_ASSET, STATEMINT_PARA_ID,
};
use crate::types::StatemintConfig;
use frame_support::{
	assert_noop, assert_ok,
	sp_runtime::{traits::Zero, FixedPointNumber},
	traits::{tokens::fungibles::Inspect, Hooks},
};
use orml_traits::MultiCurrency;
use pallet_price_feed::PriceFeed;
use polkadot_primitives::v1::{AccountId, Balance};
use primitives::{
	traits::{MultiAssetRegistry, NavProvider},
	AssetAvailability,
};
use xcm::{
	v1::{Junction, Junctions, MultiLocation, NetworkId},
	VersionedMultiAssets, VersionedMultiLocation,
};
use xcm_calls::proxy::ProxyType as ParaProxyType;
use xcm_simulator::TestExt;

type RelayChainPalletXcm = pallet_xcm::Pallet<RelayRuntime>;

#[allow(unused)]
fn print_events<T: frame_system::Config>(context: &str) {
	println!("------ {:?} events ------", context);
	frame_system::Pallet::<T>::events().iter().for_each(|r| {
		println!("{:?}", r.event);
	});
}

#[allow(unused)]
fn run_to_block<Runtime>(n: u64)
where
	Runtime: crate::Config<BlockNumber = BlockNumber>,
{
	while frame_system::Pallet::<Runtime>::block_number() < n {
		crate::Pallet::<Runtime>::on_finalize(frame_system::Pallet::<Runtime>::block_number());
		frame_system::Pallet::<Runtime>::on_finalize(frame_system::Pallet::<Runtime>::block_number());
		frame_system::Pallet::<Runtime>::set_block_number(frame_system::Pallet::<Runtime>::block_number() + 1);
		frame_system::Pallet::<Runtime>::on_initialize(frame_system::Pallet::<Runtime>::block_number());
		crate::Pallet::<Runtime>::on_initialize(frame_system::Pallet::<Runtime>::block_number());
	}
}

/// registers the relay chain as liquid asset
fn register_relay() {
	// prepare index fund so NAV is available
	let deposit = 1_000;
	assert_ok!(orml_tokens::Pallet::<PintRuntime>::deposit(RELAY_CHAIN_ASSET, &ADMIN_ACCOUNT, 1_000));
	assert_ok!(pallet_asset_index::Pallet::<PintRuntime>::register_asset(
		pint::Origin::signed(ADMIN_ACCOUNT),
		RELAY_CHAIN_ASSET,
		AssetAvailability::Liquid(MultiLocation::parent()),
	));
	assert_ok!(pallet_asset_index::Pallet::<PintRuntime>::add_asset(
		pint::Origin::signed(ADMIN_ACCOUNT),
		RELAY_CHAIN_ASSET,
		deposit,
		deposit
	));
	assert!(pallet_asset_index::Pallet::<PintRuntime>::is_liquid_asset(&RELAY_CHAIN_ASSET));
}

/// transfer the given amount of relay chain currency into the account on the
/// parachain
fn transfer_to_para(relay_deposit_amount: Balance, who: AccountId) {
	Relay::execute_with(|| {
		// transfer from relay to parachain
		assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
			relay::Origin::signed(who.clone()),
			Box::new(VersionedMultiLocation::V1(Junctions::X1(Junction::Parachain(PARA_ID)).into())),
			Box::new(VersionedMultiLocation::V1(
				Junctions::X1(Junction::AccountId32 { network: NetworkId::Any, id: who.clone().into() }).into()
			)),
			Box::new(VersionedMultiAssets::V1((Junctions::Here, relay_deposit_amount).into())),
			0,
			600_000_000,
		));
	});
	Pint::execute_with(|| {
		// ensure deposit arrived
		assert_eq!(orml_tokens::Pallet::<PintRuntime>::balance(RELAY_CHAIN_ASSET, &who), relay_deposit_amount);
	});
}

#[test]
fn para_account_funded_on_relay() {
	Net::reset();

	Relay::execute_with(|| {
		let para_balance_on_relay = pallet_balances::Pallet::<RelayRuntime>::free_balance(&relay_sovereign_account());
		assert_eq!(para_balance_on_relay, INITIAL_BALANCE);
	});
}

#[test]
fn can_deposit_from_relay() {
	Net::reset();
	Pint::execute_with(|| register_relay());
	let deposit = 1_000;
	transfer_to_para(deposit, ALICE);

	Pint::execute_with(|| {
		let initial_index_tokens = pallet_asset_index::Pallet::<PintRuntime>::index_token_issuance();
		let index_token_balance = pallet_asset_index::Pallet::<PintRuntime>::index_token_balance(&ALICE);
		let nav = pallet_asset_index::Pallet::<PintRuntime>::nav().unwrap();

		// alice has 1000 units of relay chain currency in her account on the parachain
		assert_ok!(pallet_asset_index::Pallet::<PintRuntime>::deposit(
			pint::Origin::signed(ALICE),
			RELAY_CHAIN_ASSET,
			deposit
		));
		// no more relay chain assets
		assert!(orml_tokens::Pallet::<PintRuntime>::balance(RELAY_CHAIN_ASSET, &ALICE).is_zero());

		let deposit_value = MockPriceFeed::get_price(RELAY_CHAIN_ASSET).unwrap().checked_mul_int(deposit).unwrap();
		let received = nav.reciprocal().unwrap().saturating_mul_int(deposit_value);
		assert_eq!(
			pallet_asset_index::Pallet::<PintRuntime>::index_token_balance(&ALICE),
			received + index_token_balance
		);
		assert_eq!(pallet_asset_index::Pallet::<PintRuntime>::index_token_issuance(), received + initial_index_tokens);
	});
}

#[test]
fn can_transact_register_proxy() {
	Net::reset();

	Pint::execute_with(|| {
		register_relay();
		assert_ok!(crate::Pallet::<PintRuntime>::send_add_proxy(
			pint::Origin::signed(ADMIN_ACCOUNT),
			RELAY_CHAIN_ASSET,
			ParaProxyType(RelayProxyType::Staking as u8),
			Option::None
		));

		assert_noop!(
			crate::Pallet::<PintRuntime>::send_add_proxy(
				pint::Origin::signed(ADMIN_ACCOUNT),
				RELAY_CHAIN_ASSET,
				ParaProxyType(RelayProxyType::Staking as u8),
				Option::None
			),
			crate::Error::<PintRuntime>::AlreadyProxy
		);
	});

	Relay::execute_with(|| {
		// verify the proxy is registered
		let proxy =
			pallet_proxy::Pallet::<RelayRuntime>::find_proxy(&relay_sovereign_account(), &ADMIN_ACCOUNT, Option::None)
				.unwrap();
		assert_eq!(proxy.proxy_type, RelayProxyType::Staking);
	});
}

#[test]
fn can_transact_staking() {
	Net::reset();
	// `- 1` for avoiding dust account issue
	//
	// see also https://github.com/open-web3-stack/open-runtime-module-library/issues/427
	let bond = 1_000 - 1;

	Pint::execute_with(|| {
		register_relay();
		// mint some funds first to cover the transfer
		assert_ok!(pint::Currency::deposit(RELAY_CHAIN_ASSET, &ADMIN_ACCOUNT, 1_000_000));

		// fails to bond extra, no initial bond
		assert_noop!(
			crate::Pallet::<PintRuntime>::do_send_bond_extra(RELAY_CHAIN_ASSET, bond,),
			crate::Error::<PintRuntime>::NotBonded
		);

		// transact a bond call that adds `ADMIN_ACCOUNT` as controller
		assert_ok!(crate::Pallet::<PintRuntime>::send_bond(
			pint::Origin::signed(ADMIN_ACCOUNT),
			RELAY_CHAIN_ASSET,
			ADMIN_ACCOUNT,
			bond,
			xcm_calls::staking::RewardDestination::Staked
		));

		assert_noop!(
			crate::Pallet::<PintRuntime>::send_bond(
				pint::Origin::signed(ADMIN_ACCOUNT),
				RELAY_CHAIN_ASSET,
				ADMIN_ACCOUNT,
				bond,
				xcm_calls::staking::RewardDestination::Staked
			),
			crate::Error::<PintRuntime>::AlreadyBonded
		);
	});

	Relay::execute_with(|| {
		// make sure `ADMIN_ACCOUNT` is now registered as controller
		let ledger = pallet_staking::Ledger::<RelayRuntime>::get(&ADMIN_ACCOUNT).unwrap();
		assert_eq!(ledger.total, bond);
	});

	Pint::execute_with(|| {
		// bond extra
		assert_ok!(crate::Pallet::<PintRuntime>::do_send_bond_extra(RELAY_CHAIN_ASSET, bond));
	});

	Relay::execute_with(|| {
		let ledger = pallet_staking::Ledger::<RelayRuntime>::get(&ADMIN_ACCOUNT).unwrap();
		// bond + 1x bond_extra
		assert_eq!(ledger.total, 2 * bond);
	});
}

#[test]
fn can_transfer_to_statemint() {
	Net::reset();
	let spint_id = 1u32;
	let initial_supply = 5_000;
	Statemint::execute_with(|| {
		assert_ok!(pallet_assets::Pallet::<statemint::Runtime>::create(
			statemint::Origin::signed(ALICE),
			spint_id,
			sibling_sovereign_account(),
			100
		));

		// mint some units
		assert_ok!(pallet_assets::Pallet::<statemint::Runtime>::mint(
			statemint::Origin::signed(sibling_sovereign_account()),
			spint_id,
			sibling_sovereign_account(),
			initial_supply
		));
		assert_eq!(pallet_assets::Pallet::<statemint::Runtime>::total_issuance(spint_id), initial_supply);
	});

	let transfer_amount = 1_000;
	Pint::execute_with(|| {
		// try to send PINT, but no config yet
		assert_noop!(
			crate::Pallet::<PintRuntime>::transfer_to_statemint(pint::Origin::signed(ALICE), transfer_amount),
			crate::Error::<PintRuntime>::NoStatemintConfigFound
		);

		let config = StatemintConfig { parachain_id: STATEMINT_PARA_ID, enabled: false };

		assert_ok!(crate::Pallet::<PintRuntime>::set_statemint_config(pint::Origin::signed(ADMIN_ACCOUNT), config));

		// not enabled yet
		assert_noop!(
			crate::Pallet::<PintRuntime>::transfer_to_statemint(pint::Origin::signed(ALICE), transfer_amount),
			crate::Error::<PintRuntime>::StatemintDisabled
		);
		assert_ok!(crate::Pallet::<PintRuntime>::enable_statemint_xcm(pint::Origin::signed(ADMIN_ACCOUNT)));

		// // no funds to transfer from empty account
		// assert_noop!(
		// 	crate::Pallet::<PintRuntime>::transfer_to_statemint(
		// 		pint::Origin::signed(EMPTY_ACCOUNT),
		// 		transfer_amount
		// 	),
		// 	pallet_balances::Error::<PintRuntime>::InsufficientBalance
		// );
		//
		// // transfer from pint -> statemint to mint SPINT
		// assert_ok!(crate::Pallet::<PintRuntime>::transfer_to_statemint(
		// 	pint::Origin::signed(ALICE),
		// 	transfer_amount
		// ));
	});

	// Reserve based transfers are not yet fully implemented https://github.com/paritytech/cumulus/pull/552
	// Statemint::execute_with(|| {
	// // SPINT should be minted into ALICE account
	// assert_eq!(
	// 	pallet_assets::Pallet::<statemint::Runtime>::total_issuance(spint_id),
	// 	initial_supply + transfer_amount
	// );
	// assert_eq!(pallet_assets::Pallet::<statemint::Runtime>::balance(spint_id, &ALICE),
	// transfer_amount); })
}
