// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Substrate Parachain Node Template CLI

#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;

#[cfg(not(any(feature = "kusama", feature = "polkadot")))]
pub use pint_runtime_dev as pint_runtime;
#[cfg(feature = "kusama")]
pub use pint_runtime_kusama as pint_runtime;
#[cfg(feature = "polkadot")]
pub use pint_runtime_polkadot as pint_runtime;

fn main() -> sc_cli::Result<()> {
    command::run()
}
