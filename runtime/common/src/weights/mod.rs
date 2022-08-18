// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
pub mod pallet_asset_index;
pub mod pallet_committee;
pub mod pallet_local_treasury;
pub mod pallet_price_feed;
pub mod pallet_remote_asset_manager;
pub mod pallet_saft_registry;
pub mod orml_oracle;

use crate::traits::XcmRuntimeCallWeights;
use frame_support::weights::{constants::RocksDbWeight, Weight};
use xcm_calls::{proxy::ProxyWeights, staking::StakingWeights};

impl XcmRuntimeCallWeights for StakingWeights {
	/// The weights as defined in `pallet_staking` on polkadot
	fn polkadot() -> Self {
		#![allow(clippy::unnecessary_cast)]
		let weight = RocksDbWeight::get();
		Self {
			bond: (75_102_000 as Weight)
				.saturating_add(weight.reads(5 as Weight))
				.saturating_add(weight.writes(4 as Weight)),
			bond_extra: (57_637_000 as Weight)
				.saturating_add(weight.reads(3 as Weight))
				.saturating_add(weight.writes(2 as Weight)),
			unbond: (52_115_000 as Weight)
				.saturating_add(weight.reads(4 as Weight))
				.saturating_add(weight.writes(3 as Weight)),
			// Same as unbounded temporarily
			//
			// Check https://github.com/paritytech/substrate/blob/0803f7d953938aa65de36993ed74cecb1f7b5407/frame/staking/src/lib.rs#L1622
			withdraw_unbonded: (52_115_000 as Weight)
				.saturating_add(weight.reads(4 as Weight))
				.saturating_add(weight.writes(3 as Weight)),
		}
	}

	fn kusama() -> Self {
		#![allow(clippy::unnecessary_cast)]
		let weight = RocksDbWeight::get();
		Self {
			bond: (70_648_000 as Weight)
				.saturating_add(weight.reads(5 as Weight))
				.saturating_add(weight.writes(4 as Weight)),
			bond_extra: (54_235_000 as Weight)
				.saturating_add(weight.reads(3 as Weight))
				.saturating_add(weight.writes(2 as Weight)),
			unbond: (57_950_000 as Weight)
				.saturating_add(weight.reads(6 as Weight))
				.saturating_add(weight.writes(3 as Weight)),
			// Same as unbounded temporarily
			//
			// Check https://github.com/paritytech/substrate/blob/0803f7d953938aa65de36993ed74cecb1f7b5407/frame/staking/src/lib.rs#L1622
			withdraw_unbonded: (57_950_000 as Weight)
				.saturating_add(weight.reads(6 as Weight))
				.saturating_add(weight.writes(3 as Weight)),
		}
	}
}

impl XcmRuntimeCallWeights for ProxyWeights {
	/// The weights as defined in `pallet_staking` on polkadot
	///
	/// 32 is from https://github.com/paritytech/polkadot/blob/0c670d826c7ce80b26e6214c411dc7320af58854/runtime/polkadot/src/lib.rs#L871
	fn polkadot() -> Self {
		#![allow(clippy::unnecessary_cast)]
		let weight = RocksDbWeight::get();
		Self {
			add_proxy: (34_650_000 as Weight)
				.saturating_add((212_000 as Weight).saturating_mul(32 as Weight))
				.saturating_add(weight.reads(1 as Weight))
				.saturating_add(weight.writes(1 as Weight)),
			remove_proxy: (34_378_000 as Weight)
				.saturating_add((240_000 as Weight).saturating_mul(32 as Weight))
				.saturating_add(weight.reads(1 as Weight))
				.saturating_add(weight.writes(1 as Weight)),
		}
	}

	/// The weights as defined in `pallet_staking` on polkadot
	///
	/// 32 is from https://github.com/paritytech/polkadot/blob/0c670d826c7ce80b26e6214c411dc7320af58854/runtime/kusama/src/lib.rs#L965
	fn kusama() -> Self {
		#![allow(clippy::unnecessary_cast)]
		let weight = RocksDbWeight::get();
		Self {
			add_proxy: (36_114_000 as Weight)
				.saturating_add((223_000 as Weight).saturating_mul(32 as Weight))
				.saturating_add(weight.reads(1 as Weight))
				.saturating_add(weight.writes(1 as Weight)),
			remove_proxy: (35_456_000 as Weight)
				.saturating_add((246_000 as Weight).saturating_mul(32 as Weight))
				.saturating_add(weight.reads(1 as Weight))
				.saturating_add(weight.writes(1 as Weight)),
		}
	}
}
