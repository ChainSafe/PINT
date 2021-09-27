// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Autogenerated weights for `pallet_committee`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-09-27, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("pint-local"), DB CACHE: 128

// Executed Command:
// ./target/release/pint
// benchmark
// -p
// pallet_committee
// -e
// *
// --execution
// wasm
// --wasm-execution
// compiled
// --raw
// --chain
// pint-local
// --output
// ./runtime/common/src/weights/pallet_committee.rs
// --steps
// 50
// --repeat
// 20
// --heap-pages
// 4096


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_committee.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_committee::WeightInfo for WeightInfo<T> {
	// Storage: Committee Members (r:1 w:0)
	// Storage: Committee ProposalCount (r:1 w:1)
	// Storage: Committee ActiveProposals (r:1 w:1)
	// Storage: Committee Proposals (r:0 w:1)
	// Storage: Committee Votes (r:0 w:1)
	fn propose() -> Weight {
		(30_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: Committee Members (r:1 w:0)
	// Storage: Committee VotingEligibility (r:1 w:0)
	// Storage: Committee Votes (r:1 w:1)
	fn vote() -> Weight {
		(30_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Committee Members (r:1 w:0)
	// Storage: Committee Proposals (r:1 w:1)
	// Storage: Committee Votes (r:1 w:0)
	fn close() -> Weight {
		(29_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Committee Members (r:1 w:1)
	// Storage: Committee VotingEligibility (r:0 w:1)
	fn add_constituent() -> Weight {
		(19_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: Committee Members (r:1 w:1)
	// Storage: Committee VotingEligibility (r:1 w:1)
	fn remove_member() -> Weight {
		(21_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}
