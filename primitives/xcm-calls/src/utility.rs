// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Xcm support for dispatching `pallet_utility` pallet calls

use codec::{Decode, Encode, MaxEncodedLen, Output};
use frame_support::{sp_std::vec::Vec, weights::Weight, RuntimeDebug};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use crate::{CallEncoder, EncodeWith, PalletCall, PalletCallEncoder};

/// The index of `pallet_utility` in the polkadot runtime
pub const POLKADOT_PALLET_UTILITY_INDEX: u8 = 29u8;

/// The identifier the `ProxyType::Staking` variant encodes to
pub const POLKADOT_PALLET_UTILITY_TYPE_STAKING_INDEX: u8 = 3u8;

pub trait UtilityCallEncoder: PalletCallEncoder {}

impl<'a, 'b, Config> Encode for CallEncoder<'a, 'b, UtilityCall, Config>
where
	Config: UtilityCallEncoder,
{
	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		// include the pallet identifier
		dest.push_byte(self.call.pallet_call_index());
		self.call.encode_to(dest)
	}
}

/// Represents dispatchable calls of the FRAME `pallet_utility` pallet.
///
/// This is a generic version of the `pallet_utility::Call` enum generated by the substrate pallet
/// macros
#[derive(Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub enum UtilityCall {
	#[codec(index = 1)]
	AsDerivative(u16, Vec<u8>),
	#[codec(index = 2)]
	BatchAll(Vec<Vec<u8>>),
}

impl PalletCall for UtilityCall {
	/// the indices of the corresponding calls within the `pallet_utility`
	fn pallet_call_index(&self) -> u8 {
		match self {
			UtilityCall::AsDerivative(_, _) => 1,
			UtilityCall::BatchAll(_) => 2,
		}
	}
}
