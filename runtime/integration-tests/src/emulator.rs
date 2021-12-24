use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt};
pub mod types {
	pub type AccountId = primitives::AccountId;

	pub type Balance = primitives::Balance;
}

// use crate::statemint;
use cumulus_primitives_core::ParaId;
use frame_support::{
	assert_noop, assert_ok, sp_io,
	sp_runtime::{
		traits::{AccountIdConversion, Zero},
		FixedPointNumber,
	},
	traits::{tokens::fungibles::Inspect, GenesisBuild},
};
use kusama_runtime::ProxyType as RelayProxyType;
use orml_traits::MultiCurrency;
use pallet_remote_asset_manager::types::StatemintConfig;
use primitives::{
	traits::{MultiAssetRegistry, NavProvider},
	AssetAvailability, AssetId,
};
use types::*;
use xcm::{
	v1::{Junction, Junctions, MultiLocation, NetworkId},
	VersionedMultiAssets, VersionedMultiLocation,
};
use xcm_calls::proxy::ProxyType as ParaProxyType;
use xcm_calls::{
	proxy::{ProxyConfig, ProxyWeights},
	staking::{RewardDestination, StakingConfig, StakingWeights},
};
use xcm_executor::traits::Convert;

pub const ALICE: AccountId = AccountId::new([0u8; 32]);
pub const ADMIN_ACCOUNT: AccountId = AccountId::new([1u8; 32]);
pub const RELAY_CHAIN_ASSET: AssetId = 42;
pub const PROXY_PALLET_INDEX: u8 = 30u8;
pub const STAKING_PALLET_INDEX: u8 = 6u8;
pub const INITIAL_BALANCE: Balance = 10_000_000_000_000;
pub const PARA_ID: u32 = 1u32;
pub const STATEMINT_PARA_ID: u32 = 201u32;

decl_test_relay_chain! {
	pub struct Kusama {
		Runtime = kusama_runtime::Runtime,
		XcmConfig = kusama_runtime::XcmConfig,
		new_ext = kusama_ext(),
	}
}

decl_test_parachain! {
	pub struct Shot {
		Runtime = shot_runtime::Runtime,
		Origin = shot_runtime::Origin,
		new_ext = shot_ext(PARA_ID, vec![(ALICE, INITIAL_BALANCE)]),
	}
}

// decl_test_parachain! {
// 	pub struct Statemint {
// 		Runtime = crate::statemint::Runtime,
// 		Origin = crate::statemint::Origin,
// 		new_ext = statemint_ext(STATEMINT_PARA_ID, vec![(ALICE, INITIAL_BALANCE), (sibling_sovereign_account(), INITIAL_BALANCE)]),
// 	}
// }

decl_test_network! {
	pub struct Net {
		relay_chain = Kusama,
		parachains = vec![
			(1, Shot),
		],
	}
}

fn default_parachains_host_configuration(
) -> polkadot_runtime_parachains::configuration::HostConfiguration<polkadot_primitives::v1::BlockNumber> {
	use polkadot_primitives::v1::{MAX_CODE_SIZE, MAX_POV_SIZE};

	polkadot_runtime_parachains::configuration::HostConfiguration {
		validation_upgrade_frequency: 1u32,
		validation_upgrade_delay: 1,
		code_retention_period: 1200,
		max_code_size: MAX_CODE_SIZE,
		max_pov_size: MAX_POV_SIZE,
		max_head_data_size: 32 * 1024,
		group_rotation_frequency: 20,
		chain_availability_period: 4,
		thread_availability_period: 4,
		max_upward_queue_count: 8,
		max_upward_queue_size: 1024 * 1024,
		max_downward_message_size: 1024,
		ump_service_total_weight: 4 * 1_000_000_000,
		max_upward_message_size: 1024 * 1024,
		max_upward_message_num_per_candidate: 5,
		hrmp_sender_deposit: 0,
		hrmp_recipient_deposit: 0,
		hrmp_channel_max_capacity: 8,
		hrmp_channel_max_total_size: 8 * 1024,
		hrmp_max_parachain_inbound_channels: 4,
		hrmp_max_parathread_inbound_channels: 4,
		hrmp_channel_max_message_size: 1024 * 1024,
		hrmp_max_parachain_outbound_channels: 4,
		hrmp_max_parathread_outbound_channels: 4,
		hrmp_max_message_num_per_candidate: 5,
		dispute_period: 6,
		no_show_slots: 2,
		n_delay_tranches: 25,
		needed_approvals: 2,
		relay_vrf_modulo_samples: 2,
		zeroth_delay_tranche_width: 0,
		..Default::default()
	}
}

pub fn kusama_ext() -> sp_io::TestExternalities {
	use kusama_runtime::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE), (relay_sovereign_account(), INITIAL_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	polkadot_runtime_parachains::configuration::GenesisConfig::<Runtime> {
		config: default_parachains_host_configuration(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	// configure safe xcm version to `1`
	GenesisBuild::<Runtime>::assimilate_storage(&pallet_xcm::GenesisConfig { safe_xcm_version: Some(1) }, &mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn shot_ext(parachain_id: u32, balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
	use shot_runtime::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: parachain_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> { balances }.assimilate_storage(&mut t).unwrap();

	// configure safe xcm version to `1`
	GenesisBuild::<Runtime>::assimilate_storage(&pallet_xcm::GenesisConfig { safe_xcm_version: Some(1) }, &mut t)
		.unwrap();

	// add xcm transact configs for the native asset of the relay chain
	// NOTE: weights are raw estimates
	pallet_remote_asset_manager::GenesisConfig::<Runtime> {
		staking_configs: vec![(
			RELAY_CHAIN_ASSET,
			StakingConfig {
				pallet_index: STAKING_PALLET_INDEX,
				reward_destination: RewardDestination::Staked,
				minimum_balance: 0,
				weights: StakingWeights {
					bond: 650_000_000,
					bond_extra: 1_350_000_000u64,
					unbond: 1_350_000_000u64,
					withdraw_unbonded: 1000_u64,
				},
				bonding_duration: 1_000,
				is_frozen: false,
			},
		)],
		proxy_configs: vec![(
			RELAY_CHAIN_ASSET,
			ProxyConfig {
				pallet_index: PROXY_PALLET_INDEX,
				weights: ProxyWeights { add_proxy: 180_000_000, remove_proxy: 1000_u64 },
			},
		)],
		statemint_config: None,
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

// pub fn statemint_ext(parachain_id: u32, balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
// 	use statemint::{Runtime, System};
//
// 	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
// 	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: parachain_id.into() };
//
// 	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
// 		.unwrap();
// 	pallet_balances::GenesisConfig::<Runtime> { balances }.assimilate_storage(&mut t).unwrap();
//
// 	let mut ext = sp_io::TestExternalities::new(t);
// 	ext.execute_with(|| System::set_block_number(1));
// 	ext
// }

type ShotRuntime = shot_runtime::Runtime;
type KusamaRuntime = kusama_runtime::Runtime;
type RelayChainPalletXcm = pallet_xcm::Pallet<KusamaRuntime>;

// pub fn sibling_sovereign_account() -> AccountId {
// 	use crate::statemint::LocationToAccountId;
// 	LocationToAccountId::convert(MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(PARA_ID)) })
// 		.expect("Failed to convert para")
// }

pub fn relay_sovereign_account() -> AccountId {
	let para: ParaId = PARA_ID.into();
	para.into_account()
}

/// registers the relay chain as liquid asset
fn register_relay() {
	// prepare index fund so NAV is available
	let deposit = 1_000;
	assert_ok!(orml_tokens::Pallet::<ShotRuntime>::deposit(RELAY_CHAIN_ASSET, &ADMIN_ACCOUNT, 1_000));
	assert_ok!(pallet_asset_index::Pallet::<ShotRuntime>::register_asset(
		shot_runtime::Origin::signed(ADMIN_ACCOUNT),
		RELAY_CHAIN_ASSET,
		AssetAvailability::Liquid(MultiLocation::parent()),
	));
	assert_ok!(pallet_asset_index::Pallet::<ShotRuntime>::add_asset(
		shot_runtime::Origin::signed(ADMIN_ACCOUNT),
		RELAY_CHAIN_ASSET,
		deposit,
		deposit
	));
	assert!(pallet_asset_index::Pallet::<ShotRuntime>::is_liquid_asset(&RELAY_CHAIN_ASSET));
}

/// transfer the given amount of relay chain currency into the account on the
/// parachain
fn transfer_to_para(relay_deposit_amount: Balance, who: AccountId) {
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
	let deposit = 1_000;
	transfer_to_para(deposit, ALICE);

	// Shot::execute_with(|| {
	// 	let initial_index_tokens = pallet_asset_index::Pallet::<ShotRuntime>::index_token_issuance();
	// 	let index_token_balance = pallet_asset_index::Pallet::<ShotRuntime>::index_token_balance(&ALICE);
	// 	let nav = pallet_asset_index::Pallet::<ShotRuntime>::nav().unwrap();
	//
	// 	// alice has 1000 units of relay chain currency in her account on the parachain
	// 	assert_ok!(pallet_asset_index::Pallet::<ShotRuntime>::deposit(
	// 		shot_runtime::Origin::signed(ALICE),
	// 		RELAY_CHAIN_ASSET,
	// 		deposit
	// 	));
	// 	// no more relay chain assets
	// 	assert!(orml_tokens::Pallet::<ShotRuntime>::balance(RELAY_CHAIN_ASSET, &ALICE).is_zero());
	//
	// 	let deposit_value =
	// 		crate::pint::MockPriceFeed::get_price(RELAY_CHAIN_ASSET).unwrap().checked_mul_int(deposit).unwrap();
	// 	let received = nav.reciprocal().unwrap().saturating_mul_int(deposit_value);
	// 	assert_eq!(
	// 		pallet_asset_index::Pallet::<ShotRuntime>::index_token_balance(&ALICE),
	// 		received + index_token_balance
	// 	);
	// 	assert_eq!(pallet_asset_index::Pallet::<ShotRuntime>::index_token_issuance(), received + initial_index_tokens);
	// });
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
	let bond = 10_000 - 1;

	Shot::execute_with(|| {
		register_relay();
		// mint some funds first to cover the transfer
		assert_ok!(shot_runtime::Currencies::deposit(RELAY_CHAIN_ASSET, &ADMIN_ACCOUNT, 1_000_000));

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

// #[test]
// fn tcan_transfer_to_statemint() {
// 	Net::reset();
// 	let spint_id = 1u32;
// 	let initial_supply = 5_000;
// 	Statemint::execute_with(|| {
// 		assert_ok!(pallet_assets::Pallet::<statemint::Runtime>::create(
// 			statemint::Origin::signed(ALICE),
// 			spint_id,
// 			sibling_sovereign_account().into(),
// 			100
// 		));
//
// 		// mint some units
// 		assert_ok!(pallet_assets::Pallet::<statemint::Runtime>::mint(
// 			statemint::Origin::signed(sibling_sovereign_account()),
// 			spint_id,
// 			sibling_sovereign_account().into(),
// 			initial_supply
// 		));
// 		assert_eq!(pallet_assets::Pallet::<statemint::Runtime>::total_issuance(spint_id), initial_supply);
// 	});
//
// 	let transfer_amount = 1_000;
// 	Shot::execute_with(|| {
// 		// try to send PINT, but no config yet
// 		assert_noop!(
// 			pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
// 				shot_runtime::Origin::signed(ALICE),
// 				transfer_amount
// 			),
// 			pallet_remote_asset_manager::Error::<ShotRuntime>::NoStatemintConfigFound
// 		);
//
// 		let config = StatemintConfig { parachain_id: STATEMINT_PARA_ID, enabled: false };
//
// 		assert_ok!(pallet_remote_asset_manager::Pallet::<ShotRuntime>::set_statemint_config(
// 			shot_runtime::Origin::signed(ADMIN_ACCOUNT),
// 			config
// 		));
//
// 		// not enabled yet
// 		assert_noop!(
// 			pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
// 				shot_runtime::Origin::signed(ALICE),
// 				transfer_amount
// 			),
// 			pallet_remote_asset_manager::Error::<ShotRuntime>::StatemintDisabled
// 		);
//
// 		assert_ok!(pallet_remote_asset_manager::Pallet::<ShotRuntime>::enable_statemint_xcm(
// 			shot_runtime::Origin::signed(ADMIN_ACCOUNT)
// 		));
//
// 		// // no funds to transfer from empty account
// 		// assert_noop!(
// 		// 	pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
// 		// 		shot_runtime::Origin::signed(EMPTY_ACCOUNT),
// 		// 		transfer_amount
// 		// 	),
// 		// 	pallet_balances::Error::<ShotRuntime>::InsufficientBalance
// 		// );
// 		//
// 		// pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
// 		// 	shot_runtime::Origin::signed(ALICE),
// 		// 	transfer_amount
// 		// );
//
// 		// transfer from pint -> statemint to mint SPINT
// 		// assert_ok!(pallet_remote_asset_manager::Pallet::<ShotRuntime>::transfer_to_statemint(
// 		// 	shot_runtime::Origin::signed(ALICE),
// 		// 	transfer_amount
// 		// ));
// 	});
//
// 	// Reserve based transfers are not yet fully implemented https://github.com/paritytech/cumulus/pull/552
// 	// Statemint::execute_with(|| {
// 	// // SPINT should be minted into ALICE account
// 	// assert_eq!(
// 	// 	pallet_assets::Pallet::<statemint::Runtime>::total_issuance(spint_id),
// 	// 	initial_supply + transfer_amount
// 	// );
// 	// assert_eq!(pallet_assets::Pallet::<statemint::Runtime>::balance(spint_id, &ALICE),
// 	// transfer_amount); })
// }
