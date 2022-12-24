// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Kumandra.

// Kumandra is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Kumandra is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Kumandra.  If not, see <http://www.gnu.org/licenses/>.

use crate::cli::{Cli, Subcommand};
use frame_benchmarking_cli::{BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE};
use futures::future::TryFutureExt;
use kumandra_client::benchmarking::{
	benchmark_inherent_data, ExistentialDepositProvider, RemarkBuilder, TransferKeepAliveBuilder,
};
use log::info;
use sc_cli::{RuntimeVersion, SubstrateCli};
use service::{self, HeaderBackend, IdentifyVariant};
use sp_core::crypto::Ss58AddressFormatRegistry;
use sp_keyring::Sr25519Keyring;
use std::net::ToSocketAddrs;

pub use crate::{error::Error, service::BlockId};

impl From<String> for Error {
	fn from(s: String) -> Self {
		Self::Other(s)
	}
}

type Result<T> = std::result::Result<T, Error>;

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"SmallWorld Kumandra".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/kumandra/kumandra/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn executable_name() -> String {
		"kumandra".into()
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		let id = if id == "" {
			let n = get_exec_name().unwrap_or_default();
			["kumandra"]
				.iter()
				.cloned()
				.find(|&chain| n.starts_with(chain))
				.unwrap_or("kumandra")
		} else {
			id
		};
		Ok(match id {
			"kumandra" => Box::new(service::chain_spec::kumandra_config()?),
			#[cfg(feature = "kumandra-native")]
			"kumandra-dev" | "dev" => Box::new(service::chain_spec::kumandra_development_config()?),
			#[cfg(feature = "kumandra-native")]
			"kumandra-local" => Box::new(service::chain_spec::kumandra_local_testnet_config()?),
			#[cfg(feature = "kumandra-native")]
			"kumandra-staging" => Box::new(service::chain_spec::kumandra_staging_testnet_config()?),
			path => {
				let path = std::path::PathBuf::from(path);

				let chain_spec = Box::new(service::KumandraChainSpec::from_json_file(path.clone())?)
					as Box<dyn service::ChainSpec>;
			}
		})
	}

	fn native_runtime_version(spec: &Box<dyn service::ChainSpec>) -> &'static RuntimeVersion {
		let _ = spec;

		#[cfg(feature = "kumandra-native")]
		{
			return &service::kumandra_runtime::VERSION;
		}

		#[cfg(not(feature = "kumandra-native"))]
		panic!("No runtime feature is enabled")
	}
}

fn set_default_ss58_version(spec: &Box<dyn service::ChainSpec>) {
	let ss58_version = if spec.is_kumandra() {
		Ss58AddressFormatRegistry::SubstrateAccount
	} else {
		Ss58AddressFormatRegistry::KumandraAccount
	}
	.into();

	sp_core::crypto::set_default_ss58_version(ss58_version);
}

const DEV_ONLY_ERROR_PATTERN: &'static str =
	"can only use subcommand with --chain [kumandra-dev], got ";

fn ensure_dev(spec: &Box<dyn service::ChainSpec>) -> std::result::Result<(), String> {
	if spec.is_dev() {
		Ok(())
	} else {
		Err(format!("{}{}", DEV_ONLY_ERROR_PATTERN, spec.id()))
	}
}

/// Unwraps a [`kumandra_client::Client`] into the concrete runtime client.
macro_rules! unwrap_client {
	(
		$client:ident,
		$code:expr
	) => {
		match $client.as_ref() {
			#[cfg(feature = "kumandra-native")]
			kumandra_client::Client::Kumandra($client) => $code,
			#[allow(unreachable_patterns)]
			_ => Err(Error::CommandNotImplemented),
		}
	};
}

/// Launch a node, accepting arguments just like a regular node,
/// accepts an alternative overseer generator, to adjust behavior
/// for integration tests as needed.
/// `malus_finality_delay` restrict finality votes of this node
/// to be at most `best_block - malus_finality_delay` height.
#[cfg(feature = "malus")]
pub fn run_node(run: Cli) -> Result<()> {
	run_node_inner(run, |_logger_builder, _config| {})
}

fn run_node_inner<F>(cli: Cli, logger_hook: F) -> Result<()>
where
	F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
{
	let runner = cli
		.create_runner_with_logger_hook::<sc_cli::RunCmd, F>(&cli.run.base, logger_hook)
		.map_err(Error::from)?;
	let chain_spec = &runner.config().chain_spec;

	set_default_ss58_version(chain_spec);

	runner.run_node_until_exit(move |config| async move {
		let hwbench = if !cli.run.no_hardware_benchmarks {
			config.database.path().map(|database_path| {
				let _ = std::fs::create_dir_all(&database_path);
				sc_sysinfo::gather_hwbench(Some(database_path))
			})
		} else {
			None
		};

		service::build_full(config, service::IsCollator::No, None, false, hwbench)
			.map(|full| full.task_manager)
			.map_err(Into::into)
	})
}

/// Parses kumandra specific CLI arguments and run the service.
pub fn run() -> Result<()> {
	let cli: Cli = Cli::from_args();

	match &cli.subcommand {
		None => run_node_inner(cli, None),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run(config.chain_spec, config.network))?)
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd).map_err(Error::SubstrateCli)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) =
					service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) =
					service::new_chain_ops(&mut config, None).map_err(Error::KumandraService)?;
				Ok((cmd.run(client, config.database).map_err(Error::SubstrateCli), task_manager))
			})?)
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, config.chain_spec).map_err(Error::SubstrateCli), task_manager))
			})?)
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) =
					service::new_chain_ops(&mut config, None)?;
				Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
			})?)
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run(config.database))?)
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = service::new_chain_ops(&mut config, None)?;
				let aux_revert = Box::new(|client, backend, blocks| {
					service::revert_backend(client, backend, blocks, config).map_err(|err| {
						match err {
							service::Error::Blockchain(err) => err.into(),
							// Generic application-specific error.
							err => sc_cli::Error::Application(err.into()),
						}
					})
				});
				Ok((
					cmd.run(client, backend, Some(aux_revert)).map_err(Error::SubstrateCli),
					task_manager,
				))
			})?)
		}
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			match cmd {
				#[cfg(not(feature = "runtime-benchmarks"))]
				BenchmarkCmd::Storage(_) => {
					return Err(sc_cli::Error::Input(
						"Compile with --features=runtime-benchmarks \
						to enable storage benchmarks."
							.into(),
					)
					.into())
				}
				#[cfg(feature = "runtime-benchmarks")]
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|mut config| {
					let (client, backend, _, _) = service::new_chain_ops(&mut config, None)?;
					let db = backend.expose_db();
					let storage = backend.expose_storage();

					unwrap_client!(
						client,
						cmd.run(config, client.clone(), db, storage).map_err(Error::SubstrateCli)
					)
				}),
				BenchmarkCmd::Block(cmd) => runner.sync_run(|mut config| {
					let (client, _, _, _) = service::new_chain_ops(&mut config, None)?;

					unwrap_client!(client, cmd.run(client.clone()).map_err(Error::SubstrateCli))
				}),
				// These commands are very similar and can be handled in nearly the same way.
				BenchmarkCmd::Extrinsic(_) | BenchmarkCmd::Overhead(_) => {
					ensure_dev(chain_spec).map_err(Error::Other)?;
					runner.sync_run(|mut config| {
						let (client, _, _, _) = service::new_chain_ops(&mut config, None)?;
						let header = client.header(BlockId::Number(0_u32.into())).unwrap().unwrap();
						let inherent_data = benchmark_inherent_data(header)
							.map_err(|e| format!("generating inherent data: {:?}", e))?;
						let remark_builder = RemarkBuilder::new(client.clone());

						match cmd {
							BenchmarkCmd::Extrinsic(cmd) => {
								let tka_builder = TransferKeepAliveBuilder::new(
									client.clone(),
									Sr25519Keyring::Alice.to_account_id(),
									client.existential_deposit(),
								);

								let ext_factory = ExtrinsicFactory(vec![
									Box::new(remark_builder),
									Box::new(tka_builder),
								]);

								unwrap_client!(
									client,
									cmd.run(
										client.clone(),
										inherent_data,
										Vec::new(),
										&ext_factory
									)
									.map_err(Error::SubstrateCli)
								)
							}
							BenchmarkCmd::Overhead(cmd) => unwrap_client!(
								client,
								cmd.run(
									config,
									client.clone(),
									inherent_data,
									Vec::new(),
									&remark_builder
								)
								.map_err(Error::SubstrateCli)
							),
							_ => unreachable!("Ensured by the outside match; qed"),
						}
					})
				}
				BenchmarkCmd::Pallet(cmd) => {
					set_default_ss58_version(chain_spec);
					ensure_dev(chain_spec).map_err(Error::Other)?;

					#[cfg(feature = "kumandra-native")]
					{
						return runner.sync_run(|config| {
							cmd.run::<service::kumandra_runtime::Block, service::KumandraExecutorDispatch>(config)
								.map_err(|e| Error::SubstrateCli(e))
						});
					}

					#[cfg(not(feature = "kumandra-native"))]
					#[allow(unreachable_code)]
					Err(service::Error::NoRuntime.into())
				}
				BenchmarkCmd::Machine(cmd) => runner.sync_run(|config| {
					cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())
						.map_err(Error::SubstrateCli)
				}),
				// NOTE: this allows the Kumandra client to leniently implement
				// new benchmark commands.
				#[allow(unreachable_patterns)]
				_ => Err(Error::CommandNotImplemented),
			}
		}
		Some(Subcommand::Key(cmd)) => Ok(cmd.run(&cli)?),
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;
			set_default_ss58_version(chain_spec);

			use sc_service::TaskManager;
			let registry = &runner.config().prometheus_config.as_ref().map(|cfg| &cfg.registry);
			let task_manager = TaskManager::new(runner.config().tokio_handle.clone(), *registry)
				.map_err(|e| Error::SubstrateService(sc_service::Error::Prometheus(e)))?;

			ensure_dev(chain_spec).map_err(Error::Other)?;

			#[cfg(feature = "kumandra-native")]
			{
				return runner.async_run(|config| {
					Ok((
						cmd.run::<service::kumandra_runtime::Block, service::KumandraExecutorDispatch>(
							config,
						)
						.map_err(Error::SubstrateCli),
						task_manager,
					))
				});
			}
			#[cfg(not(feature = "kumandra-native"))]
			panic!("No runtime feature is enabled")
		}
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err(Error::Other(
			"TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
				.into(),
		)
		.into()),
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run::<service::Block>(&config))?)
		}
	}?;

	Ok(())
}
