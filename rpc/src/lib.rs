// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! PINT-specific RPCs implementation.

#![warn(missing_docs)]

use primitives::{AccountId, AssetId, Balance, Block, Nonce};
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use std::sync::Arc;
use jsonrpsee::RpcModule;

pub use sc_rpc::SubscriptionTaskExecutor;

// /// A type representing all RPC extensions.
// pub type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Full client dependencies.
pub struct FullDeps<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P>(deps: FullDeps<C, P>
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: pallet_asset_index_rpc::AssetIndexRuntimeApi<Block, AccountId, AssetId, Balance>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + Sync + Send + 'static,
{
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	// use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
	// use pallet_asset_index_rpc::{AssetIndexApi, AssetIndexBackend};
	// use substrate_frame_rpc_system::{FullSystem, SystemApi};
	// let mut io = jsonrpc_core::IoHandler::default();
	// let FullDeps { client, pool, deny_unsafe } = deps;
	//
	// io.extend_with(SystemApi::to_delegate(FullSystem::new(client.clone(), pool, deny_unsafe)));
	// io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone())));
	// // Making synchronous calls in light client freezes the browser currently,
	// // more context: https://github.com/paritytech/substrate/pull/3480
	// // These RPCs should use an asynchronous caller instead.
	// io.extend_with(AssetIndexApi::to_delegate(AssetIndexBackend::new(client)));
	// io

	let mut module = RpcModule::new(());
	let FullDeps { client, pool, deny_unsafe } = deps;

	module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;


	Ok(module)
}
