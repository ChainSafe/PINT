// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use crate::{prelude::*, util::relay_sovereign_account};
use frame_support::traits::GenesisBuild;
use xcm_calls::{
	proxy::{ProxyConfig, ProxyWeights},
	staking::{RewardDestination, StakingConfig, StakingWeights},
};

fn default_parachains_host_configuration(
) -> polkadot_runtime_parachains::configuration::HostConfiguration<polkadot_primitives::v2::BlockNumber> {
	use polkadot_primitives::v2::{MAX_CODE_SIZE, MAX_POV_SIZE};

	polkadot_runtime_parachains::configuration::HostConfiguration {
		validation_upgrade_cooldown: 1u32,
		validation_upgrade_delay: 2,
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
		max_upward_message_size: 50 * 1024,
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
		minimum_validation_upgrade_delay: 5,
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

pub fn statemint_ext(parachain_id: u32, balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
	use crate::statemint::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: parachain_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();
	pallet_balances::GenesisConfig::<Runtime> { balances }.assimilate_storage(&mut t).unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
