// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::{assert_noop, assert_ok, traits::tokens::fungibles::Inspect};
use orml_traits::MultiCurrency;
use primitives::traits::MultiAssetRegistry;
use sp_runtime::{traits::Zero, FixedPointNumber};
use xcm::v1::{Junction, Junctions, MultiAsset, MultiLocation, NetworkId};
use xcm_calls::proxy::ProxyType as ParaProxyType;
use xcm_simulator::TestExt;

use crate::{
	mock::{para::MockPriceFeed, relay::ProxyType as RelayProxyType, *},
	pallet as pallet_remote_asset_manager,
	types::StatemintConfig,
};
use frame_support::traits::Hooks;
use pallet_price_feed::PriceFeed;
use primitives::{traits::NavProvider, AssetAvailability};

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
	Runtime: pallet_remote_asset_manager::Config<BlockNumber = BlockNumber>,
{
	while frame_system::Pallet::<Runtime>::block_number() < n {
		pallet_remote_asset_manager::Pallet::<Runtime>::on_finalize(frame_system::Pallet::<Runtime>::block_number());
		frame_system::Pallet::<Runtime>::on_finalize(frame_system::Pallet::<Runtime>::block_number());
		frame_system::Pallet::<Runtime>::set_block_number(frame_system::Pallet::<Runtime>::block_number() + 1);
		frame_system::Pallet::<Runtime>::on_initialize(frame_system::Pallet::<Runtime>::block_number());
		pallet_remote_asset_manager::Pallet::<Runtime>::on_initialize(frame_system::Pallet::<Runtime>::block_number());
	}
}

/// registers the relay chain as liquid asset
fn register_relay() {
	// prepare index fund so NAV is available
	let deposit = 1_000;
	assert_ok!(orml_tokens::Pallet::<para::Runtime>::deposit(RELAY_CHAIN_ASSET, &ADMIN_ACCOUNT, 1_000));
	assert_ok!(pallet_asset_index::Pallet::<para::Runtime>::register_asset(
		para::Origin::signed(ADMIN_ACCOUNT),
		RELAY_CHAIN_ASSET,
		AssetAvailability::Liquid(MultiLocation::parent()),
	));
	assert_ok!(pallet_asset_index::Pallet::<para::Runtime>::add_asset(
		para::Origin::signed(ADMIN_ACCOUNT),
		RELAY_CHAIN_ASSET,
		deposit,
		deposit
	));
	assert!(pallet_asset_index::Pallet::<para::Runtime>::is_liquid_asset(&RELAY_CHAIN_ASSET));
}

/// transfer the given amount of relay chain currency into the account on the
/// parachain
fn transfer_to_para(relay_deposit_amount: Balance, who: AccountId) {
	Relay::execute_with(|| {
		// transfer from relay to parachain
		assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
			relay::Origin::signed(who.clone()),
			Junctions::X1(Junction::Parachain(PARA_ID)),
			Junctions::X1(Junction::AccountId32 { network: NetworkId::Any, id: who.clone().into() }),
			vec![ConcreteFungible { id: Null, amount: relay_deposit_amount }],
			relay_deposit_amount as u64,
		));
	});
	Para::execute_with(|| {
		// ensure deposit arrived
		assert_eq!(orml_tokens::Pallet::<para::Runtime>::balance(RELAY_CHAIN_ASSET, &who), relay_deposit_amount);
	});
}

#[test]
fn para_account_funded_on_relay() {
	MockNet::reset();

	Relay::execute_with(|| {
		let para_balance_on_relay = pallet_balances::Pallet::<relay::Runtime>::free_balance(&relay_sovereign_account());
		assert_eq!(para_balance_on_relay, INITIAL_BALANCE);
	});
}

#[test]
fn can_deposit_from_relay() {
	MockNet::reset();
	Para::execute_with(|| register_relay());
	let deposit = 1_000;
	transfer_to_para(deposit, ALICE);

	Para::execute_with(|| {
		let initial_index_tokens = pallet_asset_index::Pallet::<para::Runtime>::index_token_issuance();
		let index_token_balance = pallet_asset_index::Pallet::<para::Runtime>::index_token_balance(&ALICE);
		let nav = pallet_asset_index::Pallet::<para::Runtime>::nav().unwrap();

		// alice has 1000 units of relay chain currency in her account on the parachain
		assert_ok!(pallet_asset_index::Pallet::<para::Runtime>::deposit(
			para::Origin::signed(ALICE),
			RELAY_CHAIN_ASSET,
			deposit
		));
		// no more relay chain assets
		assert!(orml_tokens::Pallet::<para::Runtime>::balance(RELAY_CHAIN_ASSET, &ALICE).is_zero());

		let deposit_value = MockPriceFeed::get_price(RELAY_CHAIN_ASSET).unwrap().checked_mul_int(deposit).unwrap();
		let received = nav.reciprocal().unwrap().saturating_mul_int(deposit_value);
		assert_eq!(
			pallet_asset_index::Pallet::<para::Runtime>::index_token_balance(&ALICE),
			received + index_token_balance
		);
		assert_eq!(
			pallet_asset_index::Pallet::<para::Runtime>::index_token_issuance(),
			received + initial_index_tokens
		);
	});
}

#[test]
fn can_transact_register_proxy() {
	MockNet::reset();

	Para::execute_with(|| {
		register_relay();
		assert_ok!(pallet_remote_asset_manager::Pallet::<para::Runtime>::send_add_proxy(
			para::Origin::signed(ADMIN_ACCOUNT),
			RELAY_CHAIN_ASSET,
			ParaProxyType(RelayProxyType::Staking as u8),
			Option::None
		));

		assert_noop!(
			pallet_remote_asset_manager::Pallet::<para::Runtime>::send_add_proxy(
				para::Origin::signed(ADMIN_ACCOUNT),
				RELAY_CHAIN_ASSET,
				ParaProxyType(RelayProxyType::Staking as u8),
				Option::None
			),
			pallet_remote_asset_manager::Error::<para::Runtime>::AlreadyProxy
		);
	});

	Relay::execute_with(|| {
		// verify the proxy is registered
		let proxy = pallet_proxy::Pallet::<relay::Runtime>::find_proxy(
			&relay_sovereign_account(),
			&ADMIN_ACCOUNT,
			Option::None,
		)
		.unwrap();
		assert_eq!(proxy.proxy_type, RelayProxyType::Staking);
	});
}

#[test]
fn can_transact_staking() {
	MockNet::reset();
	// `- 1` for avoiding dust account issue
	//
	// see also https://github.com/open-web3-stack/open-runtime-module-library/issues/427
	let bond = 1_000 - 1;

	Para::execute_with(|| {
		register_relay();

		// fails to bond extra, no initial bond
		assert_noop!(
			pallet_remote_asset_manager::Pallet::<para::Runtime>::do_send_bond_extra(RELAY_CHAIN_ASSET, bond,),
			pallet_remote_asset_manager::Error::<para::Runtime>::NotBonded
		);

		// transact a bond call that adds `ADMIN_ACCOUNT` as controller
		assert_ok!(pallet_remote_asset_manager::Pallet::<para::Runtime>::send_bond(
			para::Origin::signed(ADMIN_ACCOUNT),
			RELAY_CHAIN_ASSET,
			ADMIN_ACCOUNT,
			bond,
			xcm_calls::staking::RewardDestination::Staked
		));

		assert_noop!(
			pallet_remote_asset_manager::Pallet::<para::Runtime>::send_bond(
				para::Origin::signed(ADMIN_ACCOUNT),
				RELAY_CHAIN_ASSET,
				ADMIN_ACCOUNT,
				bond,
				xcm_calls::staking::RewardDestination::Staked
			),
			pallet_remote_asset_manager::Error::<para::Runtime>::AlreadyBonded
		);
	});

	Relay::execute_with(|| {
		// make sure `ADMIN_ACCOUNT` is now registered as controller
		let ledger = pallet_staking::Ledger::<relay::Runtime>::get(&ADMIN_ACCOUNT).unwrap();
		assert_eq!(ledger.total, bond);
	});

	Para::execute_with(|| {
		// bond extra
		assert_ok!(pallet_remote_asset_manager::Pallet::<para::Runtime>::do_send_bond_extra(RELAY_CHAIN_ASSET, bond));
	});

	Relay::execute_with(|| {
		let ledger = pallet_staking::Ledger::<relay::Runtime>::get(&ADMIN_ACCOUNT).unwrap();
		// bond + 1x bond_extra
		assert_eq!(ledger.total, 2 * bond);
	});
}

#[test]
fn can_transfer_to_statemint() {
	MockNet::reset();
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
	Para::execute_with(|| {
		// try to send PINT, but no config yet
		assert_noop!(
			pallet_remote_asset_manager::Pallet::<para::Runtime>::transfer_to_statemint(
				para::Origin::signed(ALICE),
				transfer_amount
			),
			pallet_remote_asset_manager::Error::<para::Runtime>::NoStatemintConfigFound
		);

		let config = StatemintConfig { parachain_id: STATEMINT_PARA_ID, enabled: false };

		assert_ok!(pallet_remote_asset_manager::Pallet::<para::Runtime>::set_statemint_config(
			para::Origin::signed(ADMIN_ACCOUNT),
			config
		));

		// not enabled yet
		assert_noop!(
			pallet_remote_asset_manager::Pallet::<para::Runtime>::transfer_to_statemint(
				para::Origin::signed(ALICE),
				transfer_amount
			),
			pallet_remote_asset_manager::Error::<para::Runtime>::StatemintDisabled
		);
		assert_ok!(pallet_remote_asset_manager::Pallet::<para::Runtime>::enable_statemint_xcm(para::Origin::signed(
			ADMIN_ACCOUNT
		)));

		// // no funds to transfer from empty account
		// assert_noop!(
		// 	pallet_remote_asset_manager::Pallet::<para::Runtime>::transfer_to_statemint(
		// 		para::Origin::signed(EMPTY_ACCOUNT),
		// 		transfer_amount
		// 	),
		// 	pallet_balances::Error::<para::Runtime>::InsufficientBalance
		// );
		//
		// // transfer from pint -> statemint to mint SPINT
		// assert_ok!(pallet_remote_asset_manager::Pallet::<para::Runtime>::transfer_to_statemint(
		// 	para::Origin::signed(ALICE),
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
