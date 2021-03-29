// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// TODO: This will be moved into the AssetIndex pallet when that is implemented
// Required here for mock and testing the SAFT registry

use sp_runtime::DispatchError;

pub enum AssetAvailability {
	Liquid,
	SAFT,
}

pub trait AssetRecorder<AssetId, Balance> {
	fn add_asset(id: AssetId, units: Balance, availability: AssetAvailability, value: Balance) -> Result<(), DispatchError>;
	fn remove_asset(id: AssetId) -> Result<(), DispatchError>;
}
