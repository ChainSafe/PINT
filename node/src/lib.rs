// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use pint_runtime_dev as pint_runtime;
#[cfg(feature = "kusama")]
use pint_runtime_kusama as pint_runtime;
#[cfg(feature = "polkadot")]
use pint_runtime_polkadot as pint_runtime;

pub mod chain_spec;
pub mod service;
