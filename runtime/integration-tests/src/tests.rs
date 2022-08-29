// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate::{prelude::*, statemint, util::*};
use frame_support::{
	assert_noop, assert_ok,
	traits::tokens::fungibles::Inspect,
};
use kusama_runtime::ProxyType as RelayProxyType;
use orml_traits::MultiCurrency;
use pallet_remote_asset_manager::types::StatemintConfig;
use xcm_calls::proxy::ProxyType as ParaProxyType;
use xcm_emulator::TestExt;

#[test]
fn para_account_funded_on_relay() {
	Net::reset();

	Kusama::execute_with(|| {
		let para_balance_on_relay = pallet_balances::Pallet::<KusamaRuntime>::free_balance(&relay_sovereign_account());
		assert_eq!(para_balance_on_relay, INITIAL_BALANCE);
	});
}

#[test]
fn can_deposit_from_relay() {
	use pallet_price_feed::PriceFeed;

	Net::reset();
	Shot::execute_with(|| register_relay());
	let deposit = 1_000_000_000;
	transfer_to_para(deposit, ALICE);

	Shot::execute_with(|| {
		let initial_index_tokens = pallet_asset_index::Pallet::<ShotRuntime>::index_token_issuance();
		let index_token_balance = pallet_asset_index::Pallet::<ShotRuntime>::index_token_balance(&ALICE);

		// create feed
		create_and_submit_feed(ADMIN_ACCOUNT, RELAY_CHAIN_ASSET, 1);
		
		let nav = pallet_asset_index::Pallet::<ShotRuntime>::nav().unwrap();
		
		// alice has 1000 units of relay chain currency in her account on the parachain
		assert_ok!(pallet_asset_index::Pallet::<ShotRuntime>::deposit(
			committee_origin(ALICE).into(),
			RELAY_CHAIN_ASSET,
			deposit
		));
		// no more relay chain assets
		assert!(orml_tokens::Pallet::<ShotRuntime>::balance(RELAY_CHAIN_ASSET, &ALICE).is_zero());
		
		let deposit_value = pallet_price_feed::Pallet::<ShotRuntime>::get_price(RELAY_CHAIN_ASSET)
			.unwrap()
			.checked_mul_int(deposit)
			.unwrap();
		let received = nav.reciprocal().unwrap().saturating_mul_int(deposit_value);
		assert_eq!(
			pallet_asset_index::Pallet::<ShotRuntime>::index_token_balance(&ALICE),
			received + index_token_balance
		);
		assert_eq!(pallet_asset_index::Pallet::<ShotRuntime>::index_token_issuance(), received + initial_index_tokens);
	});
}

#[test]
fn can_transact_register_proxy() {
	Net::reset();

	Shot::execute_with(|| {
		register_relay();
		assert_ok!(pallet_remote_asset_manager::Pallet::<ShotRuntime>::send_add_proxy(
			shot_runtime::Origin::signed(ADMIN_ACCOUNT),
			RELAY_CHAIN_ASSET,
			ParaProxyType(RelayProxyType::Staking as u8),
			Option::None
		));

		assert_noop!(
			pallet_remote_asset_manager::Pallet::<ShotRuntime>::send_add_proxy(
				shot_runtime::Origin::signed(ADMIN_ACCOUNT),
				RELAY_CHAIN_ASSET,
				ParaProxyType(RelayProxyType::Staking as u8),
				Option::None
			),
			pallet_remote_asset_manager::Error::<ShotRuntime>::AlreadyProxy
		);
	});

	Kusama::execute_with(|| {
		// verify the proxy is registered
		let proxy =
			pallet_proxy::Pallet::<KusamaRuntime>::find_proxy(&relay_sovereign_account(), &ADMIN_ACCOUNT, Option::None)
				.unwrap();
		assert_eq!(proxy.proxy_type, RelayProxyType::Staking);
	});
}

#[test]
fn tcan_transact_staking() {
	env_logger::init();
	Net::reset();
	// `- 1` for avoiding dust account issue
	//
	// see also https://github.com/open-web3-stack/open-runtime-module-library/issues/427
	let bond = 1_000_000_000 - 1;
	let deposit = 2_000_000_000;

	Shot::execute_with(|| {
		register_relay();
		// mint some funds first to cover the transfer
		assert_ok!(shot_runtime::Currencies::deposit(RELAY_CHAIN_ASSET, &ADMIN_ACCOUNT, deposit));

		// fails to bond extra, no initial bond
		assert_noop!(
			pallet_remote_asset_manager::Pallet::<ShotRuntime>::do_send_bond_extra(RELAY_CHAIN_ASSET, bond),
			pallet_remote_asset_manager::Error::<ShotRuntime>::NotBonded
		);

		// transact a bond call that adds `ADMIN_ACCOUNT` as controller
		assert_ok!(pallet_remote_asset_manager::Pallet::<ShotRuntime>::send_bond(
			shot_runtime::Origin::signed(ADMIN_ACCOUNT),
			RELAY_CHAIN_ASSET,
			ADMIN_ACCOUNT.into(),
			bond,
			xcm_calls::staking::RewardDestination::Staked
		));

		assert_noop!(
			pallet_remote_asset_manager::Pallet::<ShotRuntime>::send_bond(
				shot_runtime::Origin::signed(ADMIN_ACCOUNT),
				RELAY_CHAIN_ASSET,
				ADMIN_ACCOUNT.into(),
				bond,
				xcm_calls::staking::RewardDestination::Staked
			),
			pallet_remote_asset_manager::Error::<ShotRuntime>::AlreadyBonded
		);
	});

	Kusama::execute_with(|| {
		// make sure `ADMIN_ACCOUNT` is now registered as controller
		let ledger = pallet_staking::Ledger::<KusamaRuntime>::get(&ADMIN_ACCOUNT).unwrap();
		assert_eq!(ledger.total, bond);
	});

	Shot::execute_with(|| {
		// bond extra
		assert_ok!(pallet_remote_asset_manager::Pallet::<ShotRuntime>::do_send_bond_extra(RELAY_CHAIN_ASSET, bond));
	});

	Kusama::execute_with(|| {
		let ledger = pallet_staking::Ledger::<KusamaRuntime>::get(&ADMIN_ACCOUNT).unwrap();
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
			sibling_sovereign_account().into(),
			100
		));

		// mint some units
		assert_ok!(pallet_assets::Pallet::<statemint::Runtime>::mint(
			statemint::Origin::signed(sibling_sovereign_account()),
			spint_id,
			sibling_sovereign_account().into(),
			initial_supply
		));
		assert_eq!(pallet_assets::Pallet::<statemint::Runtime>::total_issuance(spint_id), initial_supply);
	});

	let transfer_amount = 1_000;
	Shot::execute_with(|| {
		// try to send PINT, but no config yet
		assert_noop!(
			pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
				shot_runtime::Origin::signed(ALICE),
				transfer_amount
			),
			pallet_remote_asset_manager::Error::<ShotRuntime>::NoStatemintConfigFound
		);

		let config = StatemintConfig { parachain_id: STATEMINT_PARA_ID, enabled: false };

		assert_ok!(pallet_remote_asset_manager::Pallet::<ShotRuntime>::set_statemint_config(
			shot_runtime::Origin::signed(ADMIN_ACCOUNT),
			config
		));

		// not enabled yet
		assert_noop!(
			pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
				shot_runtime::Origin::signed(ALICE),
				transfer_amount
			),
			pallet_remote_asset_manager::Error::<ShotRuntime>::StatemintDisabled
		);

		assert_ok!(pallet_remote_asset_manager::Pallet::<ShotRuntime>::enable_statemint_xcm(
			shot_runtime::Origin::signed(ADMIN_ACCOUNT)
		));

		// // no funds to transfer from empty account
		// assert_noop!(
		// 	pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
		// 		shot_runtime::Origin::signed(EMPTY_ACCOUNT),
		// 		transfer_amount
		// 	),
		// 	pallet_balances::Error::<ShotRuntime>::InsufficientBalance
		// );
		//
		// pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
		// 	shot_runtime::Origin::signed(ALICE),
		// 	transfer_amount
		// );
		//
		// // transfer from pint -> statemint to mint SPINT
		// assert_ok!(pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
		// 	shot_runtime::Origin::signed(ALICE),
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
