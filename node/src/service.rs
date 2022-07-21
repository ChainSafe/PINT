// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
// Cumulus Imports
use cumulus_client_consensus_aura::{AuraConsensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_service::{
	prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use sc_consensus_aura::StartAuraParams;
// Substrate Imports
use cumulus_primitives_parachain_inherent::MockValidationDataInherentDataProvider;
use sc_chain_spec::ChainSpec;
use sc_client_api::ExecutorProvider;
use sc_consensus::LongestChain;
use sc_consensus_aura::ImportQueueParams;
use sc_executor::NativeElseWasmExecutor;
use sc_network::NetworkService;
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, Role, TFullBackend, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_consensus_aura::sr25519::{AuthorityId as AuraId, AuthorityPair as AuraPair};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use substrate_prometheus_endpoint::Registry;

use std::sync::Arc;

use crate::client::*;

// Runtime type overrides
type BlockNumber = u32;
type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, sp_runtime::OpaqueExtrinsic>;
type Hash = sp_core::H256;

pub fn default_mock_parachain_inherent_data_provider() -> MockValidationDataInherentDataProvider {
	MockValidationDataInherentDataProvider { current_para_block: 0, relay_offset: 1000, relay_blocks_per_para_block: 2 }
}

pub struct DevExecutorDispatch;
impl sc_executor::NativeExecutionDispatch for DevExecutorDispatch {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		dev_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		dev_runtime::native_version()
	}
}

#[cfg(feature = "shot")]
mod shot_executor {
	pub use shot_runtime;

	pub struct ShotExecutorDispatch;
	impl sc_executor::NativeExecutionDispatch for ShotExecutorDispatch {
		type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

		fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
			shot_runtime::api::dispatch(method, data)
		}

		fn native_version() -> sc_executor::NativeVersion {
			shot_runtime::native_version()
		}
	}
}

#[cfg(feature = "pint")]
mod pint_executor {
	pub use pint_runtime;

	pub struct PintExecutorDispatch;
	impl sc_executor::NativeExecutionDispatch for PintExecutorDispatch {
		type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

		fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
			pint_runtime::api::dispatch(method, data)
		}

		fn native_version() -> sc_executor::NativeVersion {
			pint_runtime::native_version()
		}
	}
}

#[cfg(feature = "pint")]
pub use pint_executor::*;
#[cfg(feature = "shot")]
pub use shot_executor::*;

pub trait IdentifyVariant {
	fn is_shot(&self) -> bool;
	fn is_pint(&self) -> bool;
	fn is_dev(&self) -> bool;
}

impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_shot(&self) -> bool {
		self.id().contains("shot")
	}

	fn is_pint(&self) -> bool {
		self.id().contains("pint")
	}

	fn is_dev(&self) -> bool {
		self.id().starts_with("dev")
	}
}

/// PINT's full backend.
pub type FullBackend = TFullBackend<Block>;

/// PINT's full client.
pub type FullClient<RuntimeApi, ExecutorDispatch> =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;

/// Maybe PINT Dev full select chain.
type MaybeFullSelectChain = Option<LongestChain<FullBackend, Block>>;

pub fn new_partial<RuntimeApi, Executor>(
	config: &Configuration,
	dev: bool,
	instant_sealing: bool,
) -> Result<
	PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		MaybeFullSelectChain,
		sc_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(Option<Telemetry>, Option<TelemetryWorkerHandle>),
	>,
	sc_service::Error,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
{
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = NativeElseWasmExecutor::<Executor>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let registry = config.prometheus_registry();

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		registry,
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let select_chain = if dev { Some(LongestChain::new(backend.clone())) } else { None };

	let import_queue = if dev {
		if instant_sealing {
			// instance sealing
			sc_consensus_manual_seal::import_queue(
				Box::new(client.clone()),
				&task_manager.spawn_essential_handle(),
				registry,
			)
		} else {
			// aura import queue
			let slot_duration = sc_consensus_aura::slot_duration(&*client)?.slot_duration();

			sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _, _>(ImportQueueParams {
				block_import: client.clone(),
				justification_import: None,
				client: client.clone(),
				create_inherent_data_providers: move |_, ()| async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
						*timestamp,
						slot_duration,
					);

					Ok((timestamp, slot, default_mock_parachain_inherent_data_provider()))
				},
				spawner: &task_manager.spawn_essential_handle(),
				registry,
				can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
				check_for_equivocation: Default::default(),
				telemetry: telemetry.as_ref().map(|x| x.handle()),
			})?
		}
	} else {
		let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

		cumulus_client_consensus_aura::import_queue::<AuraPair, _, _, _, _, _, _>(
			cumulus_client_consensus_aura::ImportQueueParams {
				block_import: client.clone(),
				client: client.clone(),
				create_inherent_data_providers: move |_, _| async move {
					let time = sp_timestamp::InherentDataProvider::from_system_time();

					let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
						*time,
						slot_duration.slot_duration(),
					);

					Ok((time, slot))
				},
				registry,
				can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
				spawner: &task_manager.spawn_essential_handle(),
				telemetry: telemetry.as_ref().map(|telemetry| telemetry.handle()),
			},
		)?
	};

	Ok(PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		other: (telemetry, telemetry_worker_handle),
	})
}

/// Start a node with the given parachain `Configuration` and relay chain
/// `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the
/// runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RB, RuntimeApi, Executor, BIC>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	id: ParaId,
	_rpc_ext_builder: RB,
	build_consensus: BIC,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
	RB: Fn(
			Arc<FullClient<RuntimeApi, Executor>>,
		) -> Result<jsonrpc_core::IoHandler<sc_rpc::Metadata>, sc_service::Error>
		+ Send
		+ 'static,
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
	BIC: FnOnce(
		Arc<FullClient<RuntimeApi, Executor>>,
		Option<&Registry>,
		Option<TelemetryHandle>,
		&TaskManager,
		&polkadot_service::NewFull<polkadot_service::Client>,
		Arc<sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>>,
		Arc<NetworkService<Block, Hash>>,
		SyncCryptoStorePtr,
		bool,
	) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
	if matches!(parachain_config.role, Role::Light) {
		return Err("Light client not supported!".into());
	}

	let parachain_config = prepare_node_config(parachain_config);

	let params = new_partial(&parachain_config, false, false)?;
	let (mut telemetry, telemetry_worker_handle) = params.other;

	let relay_chain_full_node =
		cumulus_client_service::build_polkadot_full_node(polkadot_config, telemetry_worker_handle).map_err(
			|e| match e {
				polkadot_service::Error::Sub(x) => x,
				s => format!("{}", s).into(),
			},
		)?;

	let client = params.client.clone();
	let backend = params.backend.clone();
	let block_announce_validator = BlockAnnounceValidator::new(
		relay_chain_full_node.client.clone(),
		id,
	);

	let force_authoring = parachain_config.force_authoring;
	let validator = parachain_config.role.is_authority();
	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let mut task_manager = params.task_manager;
	let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);
	let (network, system_rpc_tx, start_network) = sc_service::build_network(sc_service::BuildNetworkParams {
		config: &parachain_config,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		spawn_handle: task_manager.spawn_handle(),
		import_queue: import_queue.clone(),
		block_announce_validator_builder: Some(Box::new(|_| block_announce_validator)),
		warp_sync: None,
	})?;

	let rpc_extensions_builder = {
		let client = client.clone();
		let transaction_pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = pint_rpc::FullDeps { client: client.clone(), pool: transaction_pool.clone(), deny_unsafe };

			Ok(pint_rpc::create_full(deps))
		})
	};

	if parachain_config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&parachain_config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_extensions_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.sync_keystore(),
		backend: backend.clone(),
		network: network.clone(),
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	let announce_block = {
		let network = network.clone();
		Arc::new(move |hash, data| network.announce_block(hash, data))
	};

	if validator {
		let parachain_consensus = build_consensus(
			client.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			&relay_chain_full_node,
			transaction_pool,
			network,
			params.keystore_container.sync_keystore(),
			force_authoring,
		)?;

		let spawner = task_manager.spawn_handle();

		let params = StartCollatorParams {
			para_id: id,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			task_manager: &mut task_manager,
			relay_chain_full_node,
			spawner,
			parachain_consensus,
			import_queue,
		};

		start_collator(params).await?;
	} else {
		let params = StartFullNodeParams {
			client: client.clone(),
			announce_block,
			task_manager: &mut task_manager,
			para_id: id,
			relay_chain_full_node,
		};

		start_full_node(params)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node<RuntimeApi, Executor>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	id: ParaId,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
{
	start_node_impl(
		parachain_config,
		polkadot_config,
		id,
		|_| Ok(Default::default()),
		|client,
		 prometheus_registry,
		 telemetry,
		 task_manager,
		 relay_chain_node,
		 transaction_pool,
		 sync_oracle,
		 keystore,
		 force_authoring| {
			let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

			let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
				task_manager.spawn_handle(),
				client.clone(),
				transaction_pool,
				prometheus_registry,
				telemetry.clone(),
			);

			let relay_chain_backend = relay_chain_node.backend.clone();
			let relay_chain_client = relay_chain_node.client.clone();
			Ok(AuraConsensus::build::<
				AuraPair, _, _, _, _, _, _
			>(BuildAuraConsensusParams {
				proposer_factory,
				create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
					let parachain_inherent =
						cumulus_primitives_parachain_inherent::ParachainInherentData::create_at_with_client(
							relay_parent,
							&relay_chain_client,
							&*relay_chain_backend,
							&validation_data,
							id,
						);
					async move {
						let time = sp_timestamp::InherentDataProvider::from_system_time();

						let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
							*time,
							slot_duration.slot_duration(),
						);

						let parachain_inherent = parachain_inherent.ok_or_else(|| {
							Box::<dyn std::error::Error + Send + Sync>::from("Failed to create parachain inherent")
						})?;
						Ok((time, slot, parachain_inherent))
					}
				},
				block_import: client.clone(),
				relay_chain_client: relay_chain_node.client.clone(),
				relay_chain_backend: relay_chain_node.backend.clone(),
				para_client: client,
				backoff_authoring_blocks: Option::<()>::None,
				sync_oracle,
				keystore,
				force_authoring,
				slot_duration,
				// We got around 500ms for proposing
				block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
				// And a maximum of 750ms if slots are skipped
				max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
				telemetry,
			}))
		},
	)
	.await
}

#[allow(dead_code)]
pub const SHOT_RUNTIME_NOT_AVAILABLE: &str =
	"shot runtime is not available. Please compile the node with `--features shot` to enable it.";
#[allow(dead_code)]
pub const PINT_RUNTIME_NOT_AVAILABLE: &str =
	"pint runtime is not available. Please compile the node with `--features pint` to enable it.";

/// Builds a new object suitable for chain operations.
pub fn new_chain_ops(
	mut config: &mut Configuration,
) -> Result<
	(
		Arc<Client>,
		Arc<FullBackend>,
		sc_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
	),
	ServiceError,
> {
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	if config.chain_spec.is_shot() {
		#[cfg(feature = "shot")]
		{
			let PartialComponents { client, backend, import_queue, task_manager, .. } =
				new_partial::<shot_runtime::RuntimeApi, ShotExecutorDispatch>(config, false, false)?;
			Ok((Arc::new(Client::Shot(client)), backend, import_queue, task_manager))
		}

		#[cfg(not(feature = "shot"))]
		Err(SHOT_RUNTIME_NOT_AVAILABLE.into())
	} else if config.chain_spec.is_pint() {
		#[cfg(feature = "pint")]
		{
			let PartialComponents { client, backend, import_queue, task_manager, .. } =
				new_partial::<pint_runtime::RuntimeApi, PintExecutorDispatch>(config, false, false)?;
			Ok((Arc::new(Client::Pint(client)), backend, import_queue, task_manager))
		}

		#[cfg(not(feature = "pint"))]
		Err(PINT_RUNTIME_NOT_AVAILABLE.into())
	} else {
		let PartialComponents { client, backend, import_queue, task_manager, .. } =
			new_partial(config, config.chain_spec.is_dev(), false)?;
		Ok((Arc::new(Client::Dev(client)), backend, import_queue, task_manager))
	}
}

fn inner_pint_dev(config: Configuration, instant_sealing: bool) -> Result<TaskManager, ServiceError> {
	use futures::stream::StreamExt;

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain: maybe_select_chain,
		transaction_pool,
		other: (mut telemetry, _),
	} = new_partial::<dev_runtime::RuntimeApi, DevExecutorDispatch>(&config, true, instant_sealing)?;

	let (network, system_rpc_tx, network_starter) = sc_service::build_network(sc_service::BuildNetworkParams {
		config: &config,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		spawn_handle: task_manager.spawn_handle(),
		import_queue,
		block_announce_validator_builder: None,
		warp_sync: None,
	})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(&config, task_manager.spawn_handle(), client.clone(), network.clone());
	}

	let prometheus_registry = config.prometheus_registry().cloned();

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks: Option<()> = None;

	let select_chain =
		maybe_select_chain.expect("In pint dev mode, `new_partial` will return some `select_chain`; qed");

	if role.is_authority() {
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		if instant_sealing {
			let pool = transaction_pool.pool().clone();
			let commands_stream = pool.validated_pool().import_notification_stream().map(|_| {
				sc_consensus_manual_seal::rpc::EngineCommand::SealNewBlock {
					create_empty: false,
					finalize: true,
					parent_hash: None,
					sender: None,
				}
			});

			let authorship_future =
				sc_consensus_manual_seal::run_manual_seal(sc_consensus_manual_seal::ManualSealParams {
					block_import: client.clone(),
					env: proposer_factory,
					client: client.clone(),
					pool: transaction_pool.clone(),
					commands_stream,
					select_chain,
					consensus_data_provider: None,
					create_inherent_data_providers: |_, _| async {
						Ok((
							sp_timestamp::InherentDataProvider::from_system_time(),
							default_mock_parachain_inherent_data_provider(),
						))
					},
				});
			// we spawn the future on a background thread managed by service.
			task_manager.spawn_essential_handle().spawn_blocking("instant-seal", None, authorship_future);
		} else {
			// aura
			let can_author_with = sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());
			let slot_duration = sc_consensus_aura::slot_duration(&*client)?.slot_duration();
			let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _, _>(StartAuraParams {
				slot_duration: sc_consensus_aura::slot_duration(&*client)?,
				client: client.clone(),
				select_chain,
				block_import: crate::instant_finalize::InstantFinalizeBlockImport::new(client.clone()),
				proposer_factory,
				create_inherent_data_providers: move |_, ()| async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
						*timestamp,
						slot_duration,
					);

					Ok((timestamp, slot, default_mock_parachain_inherent_data_provider()))
				},
				force_authoring,
				backoff_authoring_blocks,
				keystore: keystore_container.sync_keystore(),
				can_author_with,
				sync_oracle: network.clone(),
				justification_sync_link: network.clone(),
				// We got around 500ms for proposing
				block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
				// And a maximum of 750ms if slots are skipped
				max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
				telemetry: telemetry.as_ref().map(|x| x.handle()),
			})?;

			// the AURA authoring task is considered essential, i.e. if it
			// fails we take down the service with it.
			task_manager.spawn_essential_handle().spawn_blocking("aura", Some("block-authoring"), aura);
		}
	}

	let rpc_extensions_builder = {
		let client = client.clone();
		let transaction_pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = pint_rpc::FullDeps { client: client.clone(), pool: transaction_pool.clone(), deny_unsafe };

			Ok(pint_rpc::create_full(deps))
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_extensions_builder,
		client,
		transaction_pool,
		task_manager: &mut task_manager,
		config,
		keystore: keystore_container.sync_keystore(),
		backend,
		network,
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	network_starter.start_network();

	Ok(task_manager)
}

pub fn pint_dev(config: Configuration, instant_sealing: bool) -> Result<TaskManager, ServiceError> {
	inner_pint_dev(config, instant_sealing)
}
