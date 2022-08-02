// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate::{
	chain_spec,
	cli::{Cli, RelayChainCli, Subcommand},
	service::{self, IdentifyVariant, new_partial},
};
use codec::Encode;
use cumulus_client_service::genesis::generate_genesis_block;
use cumulus_primitives_core::ParaId;
use log::info;
// use polkadot_parachain::primitives::AccountIdConversion;
use primitives::Block;
use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams, NetworkParams, Result,
	RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::Block as BlockT;
use std::{io::Write, net::SocketAddr};
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};

fn load_spec(id: &str, para_id: ParaId) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
	Ok(match id {
		"dev-local" => Box::new(chain_spec::dev::pint_local_config(para_id)),
		"dev" => Box::new(chain_spec::dev::pint_development_config(para_id)),
		#[cfg(feature = "shot")]
		"shot-local" => Box::new(chain_spec::shot::pint_local_config(para_id)),
		#[cfg(feature = "shot")]
		"shot-dev" => Box::new(chain_spec::shot::pint_development_config(para_id)),
		#[cfg(feature = "pint")]
		"pint-local" => Box::new(chain_spec::pint::pint_local_config(para_id)),
		#[cfg(feature = "pint")]
		"pint-dev" => Box::new(chain_spec::pint::pint_development_config(para_id)),
		path => {
			let path = std::path::PathBuf::from(path);
			let starts_with = |prefix: &str| {
				path.file_name().map(|f| f.to_str().map(|s| s.starts_with(&prefix))).flatten().unwrap_or(false)
			};

			if starts_with("shot") {
				#[cfg(feature = "shot")]
				{
					Box::new(chain_spec::shot::ChainSpec::from_json_file(path)?)
				}
				#[cfg(not(feature = "shot"))]
				return Err(service::SHOT_RUNTIME_NOT_AVAILABLE.into());
			} else if starts_with("pint") {
				#[cfg(feature = "pint")]
				{
					Box::new(chain_spec::pint::ChainSpec::from_json_file(path)?)
				}
				#[cfg(not(feature = "pint"))]
				return Err(service::PINT_RUNTIME_NOT_AVAILABLE.into());
			} else {
				Box::new(chain_spec::dev::ChainSpec::from_json_file(path)?)
			}
		}
	})
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"PINT Collator".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		format!(
			"PINT Collator\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		{} [parachain-args] -- [relaychain-args]",
			Self::executable_name()
		)
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/ChainSafe/PINT/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2021
	}

	// FIXME:
	//
	// using fixed parachain_id 200 now
	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		load_spec(id, ParaId::from(200))
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		if chain_spec.is_shot() {
			#[cfg(feature = "shot")]
			return &shot_runtime::VERSION;
			#[cfg(not(feature = "shot"))]
			panic!("{}", service::SHOT_RUNTIME_NOT_AVAILABLE);
		} else if chain_spec.is_pint() {
			#[cfg(feature = "pint")]
			return &pint_runtime::VERSION;
			#[cfg(not(feature = "pint"))]
			panic!("{}", service::PINT_RUNTIME_NOT_AVAILABLE);
		} else {
			return &dev_runtime::VERSION;
		}
	}
}

impl SubstrateCli for RelayChainCli {
	fn impl_name() -> String {
		"PINT Collator".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		"PINT Collator\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		parachain-collator [parachain-args] -- [relaychain-args]"
			.into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/ChainSafe/PINT/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2021
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		polkadot_cli::Cli::from_iter([RelayChainCli::executable_name().to_string()].iter()).load_spec(id)
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		polkadot_cli::Cli::native_runtime_version(chain_spec)
	}
}

fn set_default_ss58_version(spec: &Box<dyn sc_chain_spec::ChainSpec>) {
	use sp_core::crypto::Ss58AddressFormatRegistry;

	let ss58_version = if spec.is_shot() {
		Ss58AddressFormatRegistry::KusamaAccount
	} else if spec.is_pint() {
		Ss58AddressFormatRegistry::PolkadotAccount
	} else {
		Ss58AddressFormatRegistry::SubstrateAccount
	};

	sp_core::crypto::set_default_ss58_version(ss58_version.into());
}

fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
	let mut storage = chain_spec.build_storage()?;

	storage
		.top
		.remove(sp_core::storage::well_known_keys::CODE)
		.ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

macro_rules! with_runtime {
	($chain_spec:expr, { $( $code:tt )* }) => {
		if $chain_spec.is_shot() {
            #[allow(unused_imports)]
            #[cfg(feature = "shot")]
            use shot_runtime::{Block, RuntimeApi};
            #[cfg(feature = "shot")]
            use service::{ShotExecutorDispatch as Executor};
            #[cfg(feature = "shot")]
            $( $code )*

            #[cfg(not(feature = "shot"))]
            return Err(service::SHOT_RUNTIME_NOT_AVAILABLE.into());
		} else if $chain_spec.is_pint() {
			#[allow(unused_imports)]
            #[cfg(feature = "pint")]
            use pint_runtime::{Block, RuntimeApi};
            #[cfg(feature = "pint")]
            use service::{PintExecutorDispatch as Executor};
            #[cfg(feature = "pint")]
            $( $code )*

            #[cfg(not(feature = "pint"))]
            return Err(service::PINT_RUNTIME_NOT_AVAILABLE.into());
		} else {
			#[allow(unused_imports)]
            use dev_runtime::{Block, RuntimeApi};
            use service::{DevExecutorDispatch as Executor};
            $( $code )*
		}
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => cli.create_runner(cmd)?.async_run(|mut config| {
			let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config)?;
			Ok((cmd.run(client, import_queue), task_manager))
		}),
		Some(Subcommand::ExportBlocks(cmd)) => cli.create_runner(cmd)?.async_run(|mut config| {
			let (client, _, _, task_manager) = service::new_chain_ops(&mut config)?;
			Ok((cmd.run(client, config.database), task_manager))
		}),
		Some(Subcommand::ExportState(cmd)) => cli.create_runner(cmd)?.async_run(|mut config| {
			let (client, _, _, task_manager) = service::new_chain_ops(&mut config)?;
			Ok((cmd.run(client, config.chain_spec), task_manager))
		}),
		Some(Subcommand::ImportBlocks(cmd)) => cli.create_runner(cmd)?.async_run(|mut config| {
			let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config)?;
			Ok((cmd.run(client, import_queue), task_manager))
		}),
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relaychain_args.iter()),
				);

				let polkadot_config =
					SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, config.tokio_handle.clone())
						.map_err(|err| format!("Relay chain argument error: {}", err))?;

				cmd.run(config, polkadot_config)
			})
		}
		Some(Subcommand::Revert(cmd)) => cli.create_runner(cmd)?.async_run(|mut config| {
			let (client, backend, _, task_manager) = service::new_chain_ops(&mut config)?;
			Ok((cmd.run(client, backend, None), task_manager))
		}),
		Some(Subcommand::ExportGenesisState(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();
			let chain_spec = cli.load_spec(&params.chain.clone().unwrap_or_default())?;
			let state_version = Cli::native_runtime_version(&chain_spec).state_version();
			let output_buf = with_runtime!(chain_spec, {
				{
					let block: Block =
						generate_genesis_block(&chain_spec, state_version).map_err(|e| format!("{:?}", e))?;
					let raw_header = block.header().encode();
					if params.raw {
						raw_header
					} else {
						format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
					}
				}
			});
			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		}

		Some(Subcommand::ExportGenesisWasm(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let raw_wasm_blob = extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
			let output_buf = if params.raw {
				raw_wasm_blob
			} else {
				format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
			};

			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		}
		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;
				let chain_spec = &runner.config().chain_spec;

				set_default_ss58_version(chain_spec);

				with_runtime!(chain_spec, {
					match cmd {
						BenchmarkCmd::Pallet(cmd) => {
							if cfg!(feature = "runtime-benchmarks") {
								runner.sync_run(|config| cmd.run::<Block, Executor>(config))
							} else {
								Err("Benchmarking wasn't enabled when building the node. \
						You can enable it with `--features runtime-benchmarks`."
									.into())
							}
						}
						BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
							let partials = new_partial::<RuntimeApi>(&config, true, false)?;
							cmd.run(partials.client)
						}),
						BenchmarkCmd::Storage(cmd) => runner.sync_run(|config| {
							let partials = new_partial::<RuntimeApi>(&config, true, false)?;
							let db = partials.backend.expose_db();
							let storage = partials.backend.expose_storage();

							cmd.run(config, partials.client.clone(), db, storage)
						}),
						BenchmarkCmd::Overhead(_) => Err("Unsupported benchmarking command".into()),
						BenchmarkCmd::Machine(cmd) => {
							runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
						}
					}
				})
			} else {
				Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
					.into())
			}
		}
		None => {
			let runner = cli.create_runner(&cli.run.normalize())?;
			let chain_spec = &runner.config().chain_spec;
			let is_pint_dev = cli.run.base.shared_params.dev || cli.instant_sealing;
			let collator_options = cli.run.collator_options();

			set_default_ss58_version(chain_spec);

			runner.run_node_until_exit(|config| async move {
				let para_id = chain_spec::Extensions::try_get(&*config.chain_spec).map(|e| e.para_id)
					.ok_or("Could not find parachain extension for chain-spec.")?;
				let id = ParaId::from(para_id);

				if is_pint_dev {
					return service::pint_dev(config, cli.instant_sealing).map_err(Into::into);
				} else if cli.instant_sealing {
					return Err("Instant sealing can be turned on only in `--dev` mode".into());
				}

				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relaychain_args.iter()),
				);

				// let parachain_account = AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account_truncating(&id);

				// let block: Block =
				// 	generate_genesis_block(&config.chain_spec, state_version).map_err(|e| format!("{:?}", e))?;
				// let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

				let polkadot_config =
					SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, config.tokio_handle.clone())
						.map_err(|err| format!("Relay chain argument error: {}", err))?;

				info!("Parachain id: {:?}", id);
				// info!("Parachain Account: {}", parachain_account);
				// info!("Parachain genesis state: {}", genesis_state);
				// info!("Is collating: {}", if config.role.is_authority() { "yes" } else { "no" });

				with_runtime!(config.chain_spec, {
					{
						service::start_node::<RuntimeApi>(config, polkadot_config, collator_options, id)
							.await
							.map(|r| r.0)
							.map_err(Into::into)
					}
				})
			})
		}
	}
}

impl DefaultConfigurationValues for RelayChainCli {
	fn p2p_listen_port() -> u16 {
		30334
	}

	fn rpc_ws_listen_port() -> u16 {
		9945
	}

	fn rpc_http_listen_port() -> u16 {
		9934
	}

	fn prometheus_listen_port() -> u16 {
		9616
	}
}

impl CliConfiguration<Self> for RelayChainCli {
	fn shared_params(&self) -> &SharedParams {
		self.base.base.shared_params()
	}

	fn import_params(&self) -> Option<&ImportParams> {
		self.base.base.import_params()
	}

	fn network_params(&self) -> Option<&NetworkParams> {
		self.base.base.network_params()
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		self.base.base.keystore_params()
	}

	fn base_path(&self) -> Result<Option<BasePath>> {
		Ok(self.shared_params().base_path().or_else(|| self.base_path.clone().map(Into::into)))
	}

	fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_http(default_listen_port)
	}

	fn rpc_ipc(&self) -> Result<Option<String>> {
		self.base.base.rpc_ipc()
	}

	fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_ws(default_listen_port)
	}

	fn prometheus_config(
		&self,
		default_listen_port: u16,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<PrometheusConfig>> {
		self.base.base.prometheus_config(default_listen_port, chain_spec)
	}

	fn init<F>(
		&self,
		_support_url: &String,
		_impl_version: &String,
		_logger_hook: F,
		_config: &sc_service::Configuration,
	) -> Result<()>
		where
			F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
	{
		unreachable!("PolkadotCli is never initialized; qed");
	}

	fn chain_id(&self, is_dev: bool) -> Result<String> {
		let chain_id = self.base.base.chain_id(is_dev)?;

		Ok(if chain_id.is_empty() { self.chain_id.clone().unwrap_or_default() } else { chain_id })
	}

	fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
		self.base.base.role(is_dev)
	}

	fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.base.transaction_pool()
	}

	fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
		self.base.base.state_cache_child_ratio()
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.base.rpc_methods()
	}

	fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
		self.base.base.rpc_ws_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
		self.base.base.rpc_cors(is_dev)
	}

	fn default_heap_pages(&self) -> Result<Option<u64>> {
		self.base.base.default_heap_pages()
	}

	fn force_authoring(&self) -> Result<bool> {
		self.base.base.force_authoring()
	}

	fn disable_grandpa(&self) -> Result<bool> {
		self.base.base.disable_grandpa()
	}

	fn max_runtime_instances(&self) -> Result<Option<usize>> {
		self.base.base.max_runtime_instances()
	}

	fn announce_block(&self) -> Result<bool> {
		self.base.base.announce_block()
	}
}
