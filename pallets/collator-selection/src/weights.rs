// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Autogenerated weights for module_collator_selection
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-05-30, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Native), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/acala
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=module_collator_selection
// --extrinsic=*
// --execution=native
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./modules/collator-selection/src/weights.rs
// --template=templates/module-weight-template.hbs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for module_collator_selection.
pub trait WeightInfo {
	fn set_invulnerables(b: u32, ) -> Weight;
	fn set_desired_candidates() -> Weight;
	fn set_candidacy_bond() -> Weight;
	fn register_as_candidate(c: u32, ) -> Weight;
	fn leave_intent(c: u32, ) -> Weight;
	fn note_author() -> Weight;
	fn new_session() -> Weight;
	fn start_session(r: u32, c: u32, ) -> Weight;
	fn end_session(r: u32, c: u32, ) -> Weight;
}

/// Weight functions for pallet_asset_index.
pub struct PintWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for PintWeight<T> {
	fn set_invulnerables(b: u32, ) -> Weight {
		(8_388_000 as Weight)
			// Standard Error: 0
			.saturating_add((15_000 as Weight).saturating_mul(b as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_desired_candidates() -> Weight {
		(8_000_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_candidacy_bond() -> Weight {
		(8_119_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn register_as_candidate(c: u32, ) -> Weight {
		(28_059_000 as Weight)
			// Standard Error: 0
			.saturating_add((231_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn leave_intent(c: u32, ) -> Weight {
		(18_594_000 as Weight)
			// Standard Error: 0
			.saturating_add((227_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn note_author() -> Weight {
		(6_569_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn new_session() -> Weight {
		(51_397_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn start_session(r: u32, c: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 0
			.saturating_add((428_000 as Weight).saturating_mul(r as Weight))
			// Standard Error: 0
			.saturating_add((648_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(r as Weight)))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
	fn end_session(r: u32, c: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 246_000
			.saturating_add((24_701_000 as Weight).saturating_mul(r as Weight))
			// Standard Error: 246_000
			.saturating_add((43_849_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(r as Weight)))
			.saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(c as Weight)))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(r as Weight)))
			.saturating_add(T::DbWeight::get().writes((2 as Weight).saturating_mul(c as Weight)))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn set_invulnerables(b: u32, ) -> Weight {
		(8_388_000 as Weight)
			// Standard Error: 0
			.saturating_add((15_000 as Weight).saturating_mul(b as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn set_desired_candidates() -> Weight {
		(8_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn set_candidacy_bond() -> Weight {
		(8_119_000 as Weight)
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn register_as_candidate(c: u32, ) -> Weight {
		(28_059_000 as Weight)
			// Standard Error: 0
			.saturating_add((231_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(RocksDbWeight::get().reads(5 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn leave_intent(c: u32, ) -> Weight {
		(18_594_000 as Weight)
			// Standard Error: 0
			.saturating_add((227_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn note_author() -> Weight {
		(6_569_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	fn new_session() -> Weight {
		(51_397_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn start_session(r: u32, c: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 0
			.saturating_add((428_000 as Weight).saturating_mul(r as Weight))
			// Standard Error: 0
			.saturating_add((648_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(r as Weight)))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
	fn end_session(r: u32, c: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 246_000
			.saturating_add((24_701_000 as Weight).saturating_mul(r as Weight))
			// Standard Error: 246_000
			.saturating_add((43_849_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(RocksDbWeight::get().reads((1 as Weight).saturating_mul(r as Weight)))
			.saturating_add(RocksDbWeight::get().reads((2 as Weight).saturating_mul(c as Weight)))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(r as Weight)))
			.saturating_add(RocksDbWeight::get().writes((2 as Weight).saturating_mul(c as Weight)))
	}
}
