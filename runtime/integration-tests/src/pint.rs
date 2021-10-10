// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::{sp_io, traits::GenesisBuild};
use pint_runtime_kusama::{AccountId, AssetId, Balance, DmpQueue, Runtime, System, XcmpQueue};
use xcm_calls::{
	proxy::{ProxyConfig, ProxyWeights},
	staking::{RewardDestination, StakingConfig, StakingWeights},
};
use xcm_simulator::decl_test_parachain;

pub const KUSAMA_ASSET: AssetId = 42;
pub const KUSAMA_STAKING_PALLET_INDEX: u8 = 6u8;
pub const KUSAMA_PROXY_PALLET_INDEX: u8 = 30u8;
pub const INITIAL_BALANCE: Balance = 10_000_000_000;
pub const ALICE: AccountId = AccountId::new([0u8; 32]);
pub const PARA_ID: u32 = 1u32;

pub fn pint_ext(parachain_id: u32, balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: parachain_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> { balances }.assimilate_storage(&mut t).unwrap();

	// add xcm transact configs for the native asset of the relay chain
	// NOTE: weights are raw estimates
	pallet_remote_asset_manager::GenesisConfig::<Runtime> {
		staking_configs: vec![(
			KUSAMA_ASSET,
			StakingConfig {
				pallet_index: KUSAMA_STAKING_PALLET_INDEX,
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
			KUSAMA_ASSET,
			ProxyConfig {
				pallet_index: KUSAMA_PROXY_PALLET_INDEX,
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

decl_test_parachain! {
	pub struct Pint {
		Runtime = Runtime,
		XcmpMessageHandler = XcmpQueue,
		DmpMessageHandler = DmpQueue,
		new_ext = pint_ext(PARA_ID, vec![(ALICE, INITIAL_BALANCE)]),
	}
}
