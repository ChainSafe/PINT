// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Autogenerated weights for pallet_committee
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-06-04, STEPS: `[]`, REPEAT: 1, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 128

// Executed Command:
// ./target/release/pint
// benchmark
// --execution
// wasm
// --wasm-execution
// compiled
// -p
// pallet_committee
// -e
// *
// --raw
// --output
// runtime/src/weights/pallet_committee.rs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_committee.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_committee::WeightInfo for WeightInfo<T> {
    fn propose() -> Weight {
        (30_000_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn vote() -> Weight {
        (28_000_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn close() -> Weight {
        (42_000_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn add_constituent() -> Weight {
        (16_000_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}