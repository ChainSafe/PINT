// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate::{
	chain_spec,
	cli::{Cli, RelayChainCli, Subcommand},
	service::{self, IdentifyVariant},
};
use codec::Encode;
use cumulus_client_service::genesis::generate_genesis_block;
use cumulus_primitives_core::ParaId;
use log::info;
use polkadot_parachain::primitives::AccountIdConversion;
use primitives::Block;
use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams, NetworkParams, Result,
	RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::Block as BlockT;
use std::{io::Write, net::SocketAddr};

fn load_spec(id: &str, para_id: ParaId) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
	Ok(match id {
		"pint-local" => Box::new(chain_spec::dev::pint_local_config(para_id)),
		"pint-dev" => Box::new(chain_spec::dev::pint_development_config(para_id)),
		#[cfg(feature = "kusama")]
		"pint-kusama-local" => Box::new(chain_spec::kusama::pint_local_config(para_id)),
		#[cfg(feature = "kusama")]
		"pint-kusama-dev" => Box::new(chain_spec::kusama::pint_development_config(para_id)),
		#[cfg(feature = "polkadot")]
		"pint-polkadot-local" => Box::new(chain_spec::polkadot::pint_local_config(para_id)),
		#[cfg(feature = "polkadot")]
		"pint-polkadot-dev" => Box::new(chain_spec::polkadot::pint_development_config(para_id)),
		path => {
			let path = std::path::PathBuf::from(path);
			let starts_with = |prefix: &str| {
				path.file_name().map(|f| f.to_str().map(|s| s.starts_with(&prefix))).flatten().unwrap_or(false)
			};

			if starts_with("pint_kusama") {
				#[cfg(feature = "kusama")]
				{
					Box::new(chain_spec::kusama::ChainSpec::from_json_file(path)?)
				}
				#[cfg(not(feature = "kusama"))]
				return Err(service::KUSAMA_RUNTIME_NOT_AVAILABLE.into())
			} else if starts_with("pint_polkadot") {
				#[cfg(feature = "polkadot")]
				{
					Box::new(chain_spec::polkadot::ChainSpec::from_json_file(path)?)
				}
				#[cfg(not(feature = "polkadot"))]
				return Err(service::POLKADOT_RUNTIME_NOT_AVAILABLE.into())
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

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		load_spec(id, self.run.parachain_id.unwrap_or(200).into())
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		if chain_spec.is_kusama() {
			#[cfg(feature = "kusama")]
			return &pint_runtime_kusama::VERSION
			#[cfg(not(feature = "kusama"))]
			panic!("{}", service::KUSAMA_RUNTIME_NOT_AVAILABLE);
		} else if chain_spec.is_polkadot() {
			#[cfg(feature = "polkadot")]
			return &pint_runtime_polkadot::VERSION
			#[cfg(not(feature = "polkadot"))]
			panic!("{}", service::POLKADOT_RUNTIME_NOT_AVAILABLE);
		} else {
			return &pint_runtime_dev::VERSION
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
		2017
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		polkadot_cli::Cli::from_iter([RelayChainCli::executable_name().to_string()].iter()).load_spec(id)
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		polkadot_cli::Cli::native_runtime_version(chain_spec)
	}
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
		if $chain_spec.is_kusama() {
            #[allow(unused_imports)]
            #[cfg(feature = "kusama")]
            use pint_runtime_kusama::{Block, RuntimeApi};
            #[cfg(feature = "kusama")]
            use service::{KusamaExecutor as Executor};
            #[cfg(feature = "kusama")]
            $( $code )*

            #[cfg(not(feature = "kusama"))]
            return Err(service::KUSAMA_RUNTIME_NOT_AVAILABLE.into());
		} else if $chain_spec.is_polkadot() {
			#[allow(unused_imports)]
            #[cfg(feature = "polkadot")]
            use pint_runtime_polkadot::{Block, RuntimeApi};
            #[cfg(feature = "polkadot")]
            use service::{PolkadotExecutor as Executor};
            #[cfg(feature = "polkadot")]
            $( $code )*

            #[cfg(not(feature = "polkadot"))]
            return Err(service::POLKADOT_RUNTIME_NOT_AVAILABLE.into());
		} else {
			#[allow(unused_imports)]
            use pint_runtime_dev::{Block, RuntimeApi};
            use service::{DevExecutor as Executor};
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
					[RelayChainCli::executable_name().to_string()].iter().chain(cli.relaychain_args.iter()),
				);

				let polkadot_config =
					SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, config.task_executor.clone())
						.map_err(|err| format!("Relay chain argument error: {}", err))?;

				cmd.run(config, polkadot_config)
			})
		}
		Some(Subcommand::Revert(cmd)) => cli.create_runner(cmd)?.async_run(|mut config| {
			let (client, backend, _, task_manager) = service::new_chain_ops(&mut config)?;
			Ok((cmd.run(client, backend), task_manager))
		}),
		Some(Subcommand::ExportGenesisState(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let block: Block = generate_genesis_block(&load_spec(
				&params.chain.clone().unwrap_or_default(),
				params.parachain_id.unwrap_or(200).into(),
			)?)?;
			let raw_header = block.header().encode();
			let output_buf = if params.raw {
				raw_header
			} else {
				format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
			};

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
				with_runtime!(chain_spec, { return runner.sync_run(|config| cmd.run::<Block, Executor>(config)) })
			} else {
				Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
					.into())
			}
		}
		None => {
			let runner = cli.create_runner(&cli.run.normalize())?;

			runner.run_node_until_exit(|config| async move {
				let para_id = chain_spec::Extensions::try_get(&*config.chain_spec).map(|e| e.para_id);

				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name().to_string()].iter().chain(cli.relaychain_args.iter()),
				);

				let id = ParaId::from(cli.run.parachain_id.or(para_id).unwrap_or(200));

				let parachain_account = AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(&id);

				let block: Block = generate_genesis_block(&config.chain_spec).map_err(|e| format!("{:?}", e))?;
				let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

				let task_executor = config.task_executor.clone();
				let polkadot_config = SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, task_executor)
					.map_err(|err| format!("Relay chain argument error: {}", err))?;

				info!("Parachain id: {:?}", id);
				info!("Parachain Account: {}", parachain_account);
				info!("Parachain genesis state: {}", genesis_state);
				info!("Is collating: {}", if config.role.is_authority() { "yes" } else { "no" });

				with_runtime!(config.chain_spec, {
					{
						service::start_node::<RuntimeApi, Executor>(config, polkadot_config, id)
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

	fn prometheus_config(&self, default_listen_port: u16) -> Result<Option<PrometheusConfig>> {
		self.base.base.prometheus_config(default_listen_port)
	}

	fn init<C: SubstrateCli>(&self) -> Result<()> {
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

	fn telemetry_external_transport(&self) -> Result<Option<sc_service::config::ExtTransport>> {
		self.base.base.telemetry_external_transport()
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

	fn telemetry_endpoints(&self, chain_spec: &Box<dyn ChainSpec>) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
		self.base.base.telemetry_endpoints(chain_spec)
	}
}
