// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

// enabled so unused types don't emit a warning
#![allow(dead_code)]

/// Basic relay config
use xcm_simulator::decl_test_relay_chain;

/// Relay chain runtime
pub mod relay;

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay::Runtime,
		XcmConfig = relay::XcmConfig,
		new_ext = super::relay_ext(),
	}
}

/// Common types used for tests
pub mod types {
	use sp_runtime::{traits::IdentityLookup, AccountId32};

	pub type AccountId = AccountId32;

	pub type BlockNumber = u64;

	pub type Balance = u128;

	pub type Amount = i128;

	pub type AssetId = u32;

	pub type Lookup = IdentityLookup<AccountId>;

	pub type AccountLookupSource = AccountId;
}

/// Basic converter types
pub mod convert {
	use super::types::*;
	use xcm::v0::{Junction, MultiLocation, NetworkId};

	pub struct AccountId32Convert;
	impl sp_runtime::traits::Convert<AccountId, [u8; 32]> for AccountId32Convert {
		fn convert(account_id: AccountId) -> [u8; 32] {
			account_id.into()
		}
	}

	impl sp_runtime::traits::Convert<AccountId, MultiLocation> for AccountId32Convert {
		fn convert(account_id: AccountId) -> MultiLocation {
			Junction::AccountId32 { network: NetworkId::Any, id: Self::convert(account_id) }.into()
		}
	}
}

/// Support for call encoders
pub mod calls {
	use frame_support::sp_std::marker::PhantomData;
	use orml_traits::{parameter_type_with_key, GetByKey};

	use xcm_calls::{
		proxy::{ProxyCallEncoder, ProxyType},
		staking::StakingCallEncoder,
		PalletCallEncoder, PassthroughCompactEncoder, PassthroughEncoder,
	};

	use super::types::*;
	use xcm_calls::assets::AssetsCallEncoder;

	// A type that states that all calls to the asset's native location can be
	// encoded
	parameter_type_with_key! {
		pub CanEncodeAll: |_asset_id: AssetId| -> bool {
		   true
		};
	}

	/// The encoder to use when transacting `pallet_proxy` calls
	pub struct PalletProxyEncoder<T>(PhantomData<T>);
	impl<T: GetByKey<AssetId, bool>> ProxyCallEncoder<AccountId, ProxyType, BlockNumber> for PalletProxyEncoder<T> {
		type AccountIdEncoder = PassthroughEncoder<AccountId, AssetId>;
		type ProxyTypeEncoder = PassthroughEncoder<ProxyType, AssetId>;
		type BlockNumberEncoder = PassthroughEncoder<BlockNumber, AssetId>;
	}

	impl<T: GetByKey<AssetId, bool>> PalletCallEncoder for PalletProxyEncoder<T> {
		type Context = AssetId;
		fn can_encode(ctx: &Self::Context) -> bool {
			T::get(ctx)
		}
	}

	/// The encoder to use when transacting `pallet_staking` calls
	pub struct PalletStakingEncoder<T>(PhantomData<T>);
	impl<T: GetByKey<AssetId, bool>> StakingCallEncoder<AccountLookupSource, Balance, AccountId>
		for PalletStakingEncoder<T>
	{
		type CompactBalanceEncoder = PassthroughCompactEncoder<Balance, AssetId>;
		type SourceEncoder = PassthroughEncoder<AccountLookupSource, AssetId>;
		type AccountIdEncoder = PassthroughEncoder<AccountId, AssetId>;
	}

	impl<T: GetByKey<AssetId, bool>> PalletCallEncoder for PalletStakingEncoder<T> {
		type Context = AssetId;
		fn can_encode(ctx: &Self::Context) -> bool {
			T::get(ctx)
		}
	}

	/// The encoder to use when transacting `pallet_assets` calls
	pub struct PalletAssetsEncoder<T>(PhantomData<T>);
	impl<T: GetByKey<AssetId, bool>> AssetsCallEncoder<AssetId, AccountLookupSource, Balance> for PalletAssetsEncoder<T> {
		type CompactAssetIdEncoder = PassthroughCompactEncoder<AssetId, AssetId>;
		type SourceEncoder = PassthroughEncoder<AccountLookupSource, AssetId>;
		type CompactBalanceEncoder = PassthroughCompactEncoder<Balance, AssetId>;
	}

	impl<T: GetByKey<AssetId, bool>> PalletCallEncoder for PalletAssetsEncoder<T> {
		type Context = AssetId;
		fn can_encode(ctx: &Self::Context) -> bool {
			T::get(ctx)
		}
	}
}
