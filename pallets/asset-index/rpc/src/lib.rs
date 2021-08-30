// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! RPC interface for the asset-index pallet.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use primitives::Ratio;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use self::gen_client::Client as AssetIndexClient;
pub use pallet_asset_index_rpc_runtime_api::AssetIndexApi as AssetIndexRuntimeApi;

/// Asset index state API
#[rpc]
pub trait AssetIndexApi<BlockHash, AccountId, AssetId, Balance> {
	#[rpc(name = "assetIndex_getNav")]
	fn get_nav(&self, at: Option<BlockHash>) -> Result<Ratio>;
}

/// A struct that implements the [`AssetIndexApi`].
pub struct AssetIndexBackend<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> AssetIndexBackend<C, B> {
	/// Create new `AssetIndex` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		AssetIndexBackend { client, _marker: Default::default() }
	}
}

pub enum Error {
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

impl<C, Block, AccountId, AssetId, Balance> AssetIndexApi<<Block as BlockT>::Hash, AccountId, AssetId, Balance>
	for AssetIndexBackend<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AssetIndexRuntimeApi<Block, AccountId, AssetId, Balance>,
	AccountId: Codec,
	AssetId: Codec,
	Balance: Codec,
{
	fn get_nav(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Ratio> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or(
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash,
		));
		api.get_nav(&at).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get current NAV.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
