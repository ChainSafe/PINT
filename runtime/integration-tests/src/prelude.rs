// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
pub use crate::{Kusama, Net, Shot, Statemint};
pub use primitives::{
	traits::{MultiAssetRegistry, NavProvider},
	AccountId, AssetAvailability, AssetId, Balance, BlockNumber, Header,
};
pub use xcm::{
	v1::{Junction, Junctions, MultiLocation, NetworkId},
	VersionedMultiAssets, VersionedMultiLocation,
};

// constants
pub const ALICE: AccountId = AccountId::new([0u8; 32]);
pub const ADMIN_ACCOUNT: AccountId = AccountId::new([1u8; 32]);
pub const RELAY_CHAIN_ASSET: AssetId = 42;
pub const PROXY_PALLET_INDEX: u8 = 30u8;
pub const STAKING_PALLET_INDEX: u8 = 6u8;
pub const INITIAL_BALANCE: Balance = 10_000_000_000_000;
pub const PARA_ID: u32 = 1u32;
pub const STATEMINT_PARA_ID: u32 = 201u32;

// types
pub type ShotRuntime = shot_runtime::Runtime;
pub type KusamaRuntime = kusama_runtime::Runtime;
pub type RelayChainPalletXcm = pallet_xcm::Pallet<KusamaRuntime>;
