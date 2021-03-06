// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use primitives::{AccountId, AssetId, Balance, Block, BlockNumber, Hash, Header, Nonce};
use sc_client_api::{Backend as BackendT, BlockchainEvents, KeyIterator};
use sp_api::{CallApiAt, NumberFor, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_consensus::BlockStatus;
use sp_runtime::{
	generic::{BlockId, SignedBlock},
	traits::{BlakeTwo256, Block as BlockT},
	Justifications,
};
use sp_storage::{ChildInfo, StorageData, StorageKey};
use std::sync::Arc;

/// A set of APIs that polkadot-like runtimes must implement.
pub trait RuntimeApiCollection:
	sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
	+ sp_api::ApiExt<Block>
	+ sp_block_builder::BlockBuilder<Block>
	+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
	+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
	+ pallet_asset_index_rpc::AssetIndexRuntimeApi<Block, AccountId, AssetId, Balance>
	+ sp_api::Metadata<Block>
	+ sp_offchain::OffchainWorkerApi<Block>
	+ sp_session::SessionKeys<Block>
	+ cumulus_primitives_core::CollectCollationInfo<Block>
where
	<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> RuntimeApiCollection for Api
where
	Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::ApiExt<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
		+ pallet_asset_index_rpc::AssetIndexRuntimeApi<Block, AccountId, AssetId, Balance>
		+ sp_api::Metadata<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_session::SessionKeys<Block>
		+ cumulus_primitives_core::CollectCollationInfo<Block>,
	<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

/// Config that abstracts over all available client implementations.
///
/// For a concrete type there exists [`Client`].
pub trait AbstractClient<Block, Backend>:
	BlockchainEvents<Block>
	+ Sized
	+ Send
	+ Sync
	+ ProvideRuntimeApi<Block>
	+ HeaderBackend<Block>
	+ CallApiAt<Block, StateBackend = Backend::State>
where
	Block: BlockT,
	Backend: BackendT<Block>,
	Backend::State: sp_api::StateBackend<BlakeTwo256>,
	Self::Api: RuntimeApiCollection<StateBackend = Backend::State>,
{
}

impl<Block, Backend, Client> AbstractClient<Block, Backend> for Client
where
	Block: BlockT,
	Backend: BackendT<Block>,
	Backend::State: sp_api::StateBackend<BlakeTwo256>,
	Client: BlockchainEvents<Block>
		+ ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ Sized
		+ Send
		+ Sync
		+ CallApiAt<Block, StateBackend = Backend::State>,
	Client::Api: RuntimeApiCollection<StateBackend = Backend::State>,
{
}

/// Execute something with the client instance.
///
/// As there exist multiple chains inside Polkadot, like Polkadot itself,
/// Kusama, Dev etc, there can exist different kinds of client types. As these
/// client types differ in the generics that are being used, we can not easily
/// return them from a function. For returning them from a function there exists
/// [`Client`]. However, the problem on how to use this client instance still
/// exists. This trait "solves" it in a dirty way. It requires a type to
/// implement this trait and than the [`execute_with_client`](ExecuteWithClient:
/// :execute_with_client) function can be called with any possible client
/// instance.
///
/// In a perfect world, we could make a closure work in this way.
pub trait ExecuteWithClient {
	/// The return type when calling this instance.
	type Output;

	/// Execute whatever should be executed with the given client instance.
	fn execute_with_client<Client, Api, Backend>(self, client: Arc<Client>) -> Self::Output
	where
		<Api as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
		Backend: sc_client_api::Backend<Block>,
		Backend::State: sp_api::StateBackend<BlakeTwo256>,
		Api: RuntimeApiCollection<StateBackend = Backend::State>,
		Client: AbstractClient<Block, Backend, Api = Api> + 'static;
}

/// A handle to a Polkadot client instance.
///
/// The Polkadot service supports multiple different runtimes (Kusama, Polkadot
/// itself, etc). As each runtime has a specialized client, we need to hide them
/// behind a trait. This is this trait.
///
/// When wanting to work with the inner client, you need to use `execute_with`.
pub trait ClientHandle {
	/// Execute the given something with the client.
	fn execute_with<T: ExecuteWithClient>(&self, t: T) -> T::Output;
}

/// A client instance of Polkadot.
#[derive(Clone)]
pub enum Client {
	Dev(Arc<crate::service::FullClient<dev_runtime::RuntimeApi, crate::service::DevExecutorDispatch>>),
	#[cfg(feature = "shot")]
	Shot(Arc<crate::service::FullClient<shot_runtime::RuntimeApi, crate::service::ShotExecutorDispatch>>),
	#[cfg(feature = "pint")]
	Pint(Arc<crate::service::FullClient<pint_runtime::RuntimeApi, crate::service::PintExecutorDispatch>>),
}

impl ClientHandle for Client {
	fn execute_with<T: ExecuteWithClient>(&self, t: T) -> T::Output {
		match self {
			Self::Dev(client) => T::execute_with_client::<_, _, crate::service::FullBackend>(t, client.clone()),
			#[cfg(feature = "shot")]
			Self::Shot(client) => T::execute_with_client::<_, _, crate::service::FullBackend>(t, client.clone()),
			#[cfg(feature = "pint")]
			Self::Pint(client) => T::execute_with_client::<_, _, crate::service::FullBackend>(t, client.clone()),
		}
	}
}

impl sc_client_api::UsageProvider<Block> for Client {
	fn usage_info(&self) -> sc_client_api::ClientInfo<Block> {
		match self {
			Self::Dev(client) => client.usage_info(),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.usage_info(),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.usage_info(),
		}
	}
}

impl sc_client_api::BlockBackend<Block> for Client {
	fn block_body(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<Vec<<Block as BlockT>::Extrinsic>>> {
		match self {
			Self::Dev(client) => client.block_body(id),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.block_body(id),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.block_body(id),
		}
	}

	fn block(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<SignedBlock<Block>>> {
		match self {
			Self::Dev(client) => client.block(id),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.block(id),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.block(id),
		}
	}

	fn block_status(&self, id: &BlockId<Block>) -> sp_blockchain::Result<BlockStatus> {
		match self {
			Self::Dev(client) => client.block_status(id),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.block_status(id),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.block_status(id),
		}
	}

	fn justifications(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<Justifications>> {
		match self {
			Self::Dev(client) => client.justifications(id),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.justifications(id),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.justifications(id),
		}
	}

	fn block_hash(&self, number: NumberFor<Block>) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		match self {
			Self::Dev(client) => client.block_hash(number),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.block_hash(number),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.block_hash(number),
		}
	}

	fn indexed_transaction(&self, hash: &<Block as BlockT>::Hash) -> sp_blockchain::Result<Option<Vec<u8>>> {
		match self {
			Self::Dev(client) => client.indexed_transaction(hash),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.indexed_transaction(hash),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.indexed_transaction(hash),
		}
	}

	fn has_indexed_transaction(&self, hash: &<Block as BlockT>::Hash) -> sp_blockchain::Result<bool> {
		match self {
			Self::Dev(client) => client.has_indexed_transaction(hash),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.has_indexed_transaction(hash),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.has_indexed_transaction(hash),
		}
	}

	fn block_indexed_body(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<Vec<Vec<u8>>>> {
		match self {
			Self::Dev(client) => client.block_indexed_body(id),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.block_indexed_body(id),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.block_indexed_body(id),
		}
	}
}

impl sc_client_api::StorageProvider<Block, crate::service::FullBackend> for Client {
	fn storage(&self, id: &BlockId<Block>, key: &StorageKey) -> sp_blockchain::Result<Option<StorageData>> {
		match self {
			Self::Dev(client) => client.storage(id, key),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.storage(id, key),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.storage(id, key),
		}
	}

	fn storage_keys(&self, id: &BlockId<Block>, key_prefix: &StorageKey) -> sp_blockchain::Result<Vec<StorageKey>> {
		match self {
			Self::Dev(client) => client.storage_keys(id, key_prefix),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.storage_keys(id, key_prefix),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.storage_keys(id, key_prefix),
		}
	}

	fn storage_hash(
		&self,
		id: &BlockId<Block>,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		match self {
			Self::Dev(client) => client.storage_hash(id, key),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.storage_hash(id, key),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.storage_hash(id, key),
		}
	}

	fn storage_pairs(
		&self,
		id: &BlockId<Block>,
		key_prefix: &StorageKey,
	) -> sp_blockchain::Result<Vec<(StorageKey, StorageData)>> {
		match self {
			Self::Dev(client) => client.storage_pairs(id, key_prefix),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.storage_pairs(id, key_prefix),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.storage_pairs(id, key_prefix),
		}
	}

	fn storage_keys_iter<'a>(
		&self,
		id: &BlockId<Block>,
		prefix: Option<&'a StorageKey>,
		start_key: Option<&StorageKey>,
	) -> sp_blockchain::Result<
		KeyIterator<'a, <crate::service::FullBackend as sc_client_api::Backend<Block>>::State, Block>,
	> {
		match self {
			Self::Dev(client) => client.storage_keys_iter(id, prefix, start_key),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.storage_keys_iter(id, prefix, start_key),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.storage_keys_iter(id, prefix, start_key),
		}
	}

	fn child_storage(
		&self,
		id: &BlockId<Block>,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<StorageData>> {
		match self {
			Self::Dev(client) => client.child_storage(id, child_info, key),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.child_storage(id, child_info, key),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.child_storage(id, child_info, key),
		}
	}

	fn child_storage_keys(
		&self,
		id: &BlockId<Block>,
		child_info: &ChildInfo,
		key_prefix: &StorageKey,
	) -> sp_blockchain::Result<Vec<StorageKey>> {
		match self {
			Self::Dev(client) => client.child_storage_keys(id, child_info, key_prefix),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.child_storage_keys(id, child_info, key_prefix),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.child_storage_keys(id, child_info, key_prefix),
		}
	}

	fn child_storage_keys_iter<'a>(
		&self,
		id: &BlockId<Block>,
		child_info: ChildInfo,
		prefix: Option<&'a StorageKey>,
		start_key: Option<&StorageKey>,
	) -> sp_blockchain::Result<
		KeyIterator<'a, <crate::service::FullBackend as sc_client_api::Backend<Block>>::State, Block>,
	> {
		match self {
			Self::Dev(client) => client.child_storage_keys_iter(id, child_info, prefix, start_key),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.child_storage_keys_iter(id, child_info, prefix, start_key),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.child_storage_keys_iter(id, child_info, prefix, start_key),
		}
	}

	fn child_storage_hash(
		&self,
		id: &BlockId<Block>,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		match self {
			Self::Dev(client) => client.child_storage_hash(id, child_info, key),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.child_storage_hash(id, child_info, key),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.child_storage_hash(id, child_info, key),
		}
	}
}

impl sp_blockchain::HeaderBackend<Block> for Client {
	fn header(&self, id: BlockId<Block>) -> sp_blockchain::Result<Option<Header>> {
		match self {
			Self::Dev(client) => client.header(&id),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.header(&id),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.header(&id),
		}
	}

	fn info(&self) -> sp_blockchain::Info<Block> {
		match self {
			Self::Dev(client) => client.info(),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.info(),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.info(),
		}
	}

	fn status(&self, id: BlockId<Block>) -> sp_blockchain::Result<sp_blockchain::BlockStatus> {
		match self {
			Self::Dev(client) => client.status(id),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.status(id),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.status(id),
		}
	}

	fn number(&self, hash: Hash) -> sp_blockchain::Result<Option<BlockNumber>> {
		match self {
			Self::Dev(client) => client.number(hash),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.number(hash),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.number(hash),
		}
	}

	fn hash(&self, number: BlockNumber) -> sp_blockchain::Result<Option<Hash>> {
		match self {
			Self::Dev(client) => client.hash(number),
			#[cfg(feature = "shot")]
			Self::Shot(client) => client.hash(number),
			#[cfg(feature = "pint")]
			Self::Pint(client) => client.hash(number),
		}
	}
}
