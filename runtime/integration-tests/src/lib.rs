// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
#![cfg(test)]

mod ext;
mod prelude;
mod statemint;
mod tests;
mod util;

use crate::{
	ext::{kusama_ext, shot_ext, statemint_ext},
	prelude::*,
	util::sibling_sovereign_account,
};
use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

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

decl_test_parachain! {
	pub struct Statemint {
		Runtime = crate::statemint::Runtime,
		Origin = crate::statemint::Origin,
		new_ext = statemint_ext(STATEMINT_PARA_ID, vec![(ALICE, INITIAL_BALANCE), (sibling_sovereign_account(), INITIAL_BALANCE)]),
	}
}

decl_test_network! {
	pub struct Net {
		relay_chain = Kusama,
		parachains = vec![
			(1, Shot),
			(200, Statemint),
		],
	}
}
