// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

mod emulator;
pub mod pint;
pub mod relay;
pub mod statemint;

#[cfg(test)]
mod tests;

pub mod types {
	use sp_runtime::traits::AccountIdLookup;

	pub type AccountId = primitives::AccountId;

	pub type BlockNumber = primitives::BlockNumber;

	pub type Balance = primitives::Balance;

	pub type Amount = i128;

	pub type AssetId = primitives::AssetId;

	pub type Lookup = AccountIdLookup<AccountId, ()>;

	pub type AccountLookupSource = AccountId;

	pub type Header = primitives::Header;
}

use cumulus_primitives_core::ParaId;
use frame_support::{sp_io, sp_runtime::traits::AccountIdConversion, traits::GenesisBuild};
use primitives::AssetId;
use types::*;
use xcm::v1::{Junction, Junctions, MultiLocation};
use xcm_calls::{
	proxy::{ProxyConfig, ProxyWeights},
	staking::{RewardDestination, StakingConfig, StakingWeights},
};
use xcm_executor::traits::Convert;
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

pub const ALICE: AccountId = AccountId::new([0u8; 32]);
pub const ADMIN_ACCOUNT: AccountId = AccountId::new([1u8; 32]);
pub const EMPTY_ACCOUNT: AccountId = AccountId::new([2u8; 32]);
pub const RELAY_CHAIN_ASSET: AssetId = 42;
pub const PROXY_PALLET_INDEX: u8 = 29u8;
pub const STAKING_PALLET_INDEX: u8 = 7u8;
pub const INITIAL_BALANCE: Balance = 10_000_000_000;
pub const PARA_ID: u32 = 1u32;
pub const PARA_ASSET: AssetId = 1;
pub const STATEMINT_PARA_ID: u32 = 200u32;

pub fn sibling_sovereign_account() -> AccountId {
	use statemint::LocationToAccountId;
	LocationToAccountId::convert(MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(PARA_ID)) })
		.expect("Failed to convert para")
}

pub fn relay_sovereign_account() -> AccountId {
	let para: ParaId = PARA_ID.into();
	para.into_account()
}

pub fn pint_ext(parachain_id: u32, balances: Vec<(AccountId, Balance)>) -> sp_io::TestExternalities {
	use pint::{Runtime, System};

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

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	// also fund the parachain's sovereign account on the relay chain
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE), (relay_sovereign_account(), INITIAL_BALANCE)],
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

decl_test_parachain! {
	pub struct Pint {
		Runtime = pint::Runtime,
		XcmpMessageHandler = pint::XcmpQueue,
		DmpMessageHandler = pint::DmpQueue,
		new_ext = pint_ext(PARA_ID, vec![(ALICE, INITIAL_BALANCE)]),
	}
}

decl_test_parachain! {
	pub struct Statemint {
		Runtime = statemint::Runtime,
		XcmpMessageHandler = statemint::XcmpQueue,
		DmpMessageHandler = statemint::DmpQueue,
		new_ext = statemint_ext(STATEMINT_PARA_ID, vec![(ALICE, INITIAL_BALANCE), (sibling_sovereign_account(), INITIAL_BALANCE)]),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay::Runtime,
		XcmConfig = relay::XcmConfig,
		new_ext = relay_ext(),
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
