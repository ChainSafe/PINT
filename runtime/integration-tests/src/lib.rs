// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

mod statemint;

#[cfg(test)]
mod tests;

use cumulus_primitives_core::ParaId;
use frame_support::{sp_io, sp_runtime::traits::AccountIdConversion, traits::GenesisBuild};
use polkadot_primitives::v1::{AccountId, Balance};
use primitives::AssetId;
use xcm::v1::{Junction, Junctions, MultiLocation};
use xcm_calls::{
	proxy::{ProxyConfig, ProxyWeights},
	staking::{RewardDestination, StakingConfig, StakingWeights},
};
use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};
use xcm_executor::traits::Convert;

pub const RELAY_CHAIN_ASSET: AssetId = 42;
pub const RELAY_CHAIN_STAKING_PALLET_INDEX: u8 = 6u8;
pub const RELAY_CHAIN_PROXY_PALLET_INDEX: u8 = 30u8;
pub const INITIAL_BALANCE: Balance = 10_000_000_000;
pub const ALICE: AccountId = AccountId::new([0u8; 32]);
pub const PARA_ID: u32 = 1u32;
pub const STATEMINT_PARA_ID: u32 = 200u32;

pub fn sibling_sovereign_account() -> AccountId {
	use pint_runtime_kusama::LocationToAccountId;
	LocationToAccountId::convert(MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(PARA_ID)) })
		.expect("Failed to convert para")
}

pub fn relay_sovereign_account() -> AccountId {
	let para: ParaId = PARA_ID.into();
	para.into_account()
}

pub fn pint_ext(parachain_id: u32, balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
	use pint_runtime_kusama::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: parachain_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> { balances }.assimilate_storage(&mut t).unwrap();

	// add xcm transact configs for the native asset of the relay chain
	// NOTE: weights are raw estimates
	pallet_remote_asset_manager::GenesisConfig::<Runtime> {
		staking_configs: vec![(
			RELAY_CHAIN_ASSET,
			StakingConfig {
				pallet_index: RELAY_CHAIN_STAKING_PALLET_INDEX,
				reward_destination: RewardDestination::Staked,
				minimum_balance: 0,
				weights: StakingWeights {
					bond: 650_000_000,
					bond_extra: 350_000_000,
					unbond: 1000_u64,
					withdraw_unbonded: 1000_u64,
				},
				bonding_duration: 1_000,
				is_frozen: false,
			},
		)],
		proxy_configs: vec![(
			RELAY_CHAIN_ASSET,
			ProxyConfig {
				pallet_index: RELAY_CHAIN_PROXY_PALLET_INDEX,
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

pub fn statemint_ext(parachain_id: u32, balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
	use statemint::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: parachain_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();
	pallet_balances::GenesisConfig::<Runtime> { balances }.assimilate_storage(&mut t).unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
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

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

decl_test_parachain! {
	pub struct Pint {
		Runtime = pint_runtime_kusama::Runtime,
		Origin = pint_runtime_kusama::Origin,
		new_ext = pint_ext(PARA_ID, vec![(ALICE, INITIAL_BALANCE)]),
	}
}

decl_test_parachain! {
	pub struct Statemint {
		Runtime = statemint::Runtime,
		Origin = statemint::Origin,
		new_ext = statemint_ext(STATEMINT_PARA_ID, vec![(ALICE, INITIAL_BALANCE), (sibling_sovereign_account(), INITIAL_BALANCE)]),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = kusama_runtime::Runtime,
		XcmConfig = kusama_runtime::XcmConfig,
		new_ext = kusama_ext(),
	}
}

decl_test_network! {
	pub struct Net {
		relay_chain = Relay,
		parachains = vec! [
			(1, Pint),
			(200, Statemint),
		],
	}
}
