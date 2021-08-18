// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
//! PINT common traits

/// Weights of Xcm calls for different runtimes
pub trait XcmRuntimeCallWeights {
	fn polkadot() -> Self;
	fn kusama() -> Self;
}
