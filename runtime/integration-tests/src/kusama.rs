// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate::pint::{ALICE, INITIAL_BALANCE, PARA_ID};
use cumulus_primitives_core::ParaId;
use frame_support::{sp_io, sp_runtime::traits::AccountIdConversion};
use kusama_runtime::{BuildStorage, GenesisConfig, Runtime, System, XcmConfig};
use xcm_simulator::decl_test_relay_chain;

pub fn kusama_ext() -> sp_io::TestExternalities {
	let mut t = GenesisConfig::default().build_storage().unwrap();
	let para: ParaId = PARA_ID.into();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE), (para.into_account(), INITIAL_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

decl_test_relay_chain! {
	pub struct Kusama {
		Runtime = Runtime,
		XcmConfig = XcmConfig,
		new_ext = kusama_ext(),
	}
}
