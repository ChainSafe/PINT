// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
//! This crate is a re-export of PINT runtimes
//!
//! Please refactor this to `pint-client` in the future if it's needed.
#[cfg(feature = "dev")]
pub use pint_runtime_dev::*;
#[cfg(feature = "kusama")]
pub use pint_runtime_kusama::*;
#[cfg(feature = "polkadot")]
pub use pint_runtime_polkadot::*;
