// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Autogenerated weights for pallet_remote_asset_manager
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-06-03, STEPS: `[]`, REPEAT: 1, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 128

// Executed Command:
// ./target/release/pint
// benchmark
// --execution
// wasm
// --wasm-execution
// compiled
// -p
// pallet_remote_asset_manager
// -e
// *
// --raw
// --output
// runtime/src/weights/pallet_remote_asset_manager.rs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_remote_asset_manager.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_remote_asset_manager::WeightInfo for WeightInfo<T> {
    fn transfer() -> Weight {
        (2_000_000 as Weight)
    }
}
