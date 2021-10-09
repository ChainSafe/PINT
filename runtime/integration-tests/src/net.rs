// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate::{
	kusama::Kusama,
	pint::{Pint, PARA_ID},
	statemint::Statemint,
};
use xcm_simulator::decl_test_network;

pub const STATEMINT_PARA_ID: u32 = 200u32;

decl_test_network! {
	pub struct Net {
		relay_chain = Kusama,
		parachains = vec! [
			(STATEMINT_PARA_ID, Statemint),
			(PARA_ID, Pint),
		],
	}
}
