// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Runtime API definition for the asset-index pallet.

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_mut_passed)]

use codec::Codec;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
use sp_std::prelude::*;

use primitives::Ratio;

sp_api::decl_runtime_apis! {
	pub trait AssetIndexApi<AccountId, AssetId, Balance> where
		AccountId: Codec,
		AssetId: Codec,
		Balance: Codec + MaybeDisplay + MaybeFromStr,
	{
		fn get_nav() -> Ratio;
	}
}
