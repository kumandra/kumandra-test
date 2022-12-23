// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Kumandra chain configurations.

use frame_support::weights::Weight;
use grandpa::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_staking::Forcing;
use kumandra_primitives::{AccountId, AccountPublic, AssignmentId, ValidatorId};
#[cfg(feature = "kumandra-native")]
use kumandra_runtime as kumandra;
#[cfg(feature = "kumandra-native")]
use kumandra_runtime::constant::currency::DOLLARS as KMD;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;

use sc_chain_spec::{ChainSpecExtension, ChainType};
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::{traits::IdentifyAccount, Perbill};
use telemetry::TelemetryEndpoints;

#[cfg(feature = "kumandra-native")]
const KUMANDRA_STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const VERSI_STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const DEFAULT_PROTOCOL_ID: &str = "kmd";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<kumandra_primitives::Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<kumandra_primitives::Block>,
	/// The light sync state.
	///
	/// This value will be set by the `sync-state rpc` implementation.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// The `ChainSpec` parameterized for the polkadot runtime.
#[cfg(feature = "kumandra-native")]
pub type KumandraChainSpec = service::GenericChainSpec<polkadot::GenesisConfig, Extensions>;

// Dummy chain spec, in case when we don't have the native runtime.
pub type DummyChainSpec = service::GenericChainSpec<(), Extensions>;

// Dummy chain spec, but that is fine when we don't have the native runtime.
#[cfg(not(feature = "kumandra-native"))]
pub type KumandraChainSpec = DummyChainSpec;

/// The `ChainSpec` parameterized for the `versi` runtime.
///
/// As of now `Versi` will just be a clone of `Rococo`, until we need it to differ.
pub type VersiChainSpec = RococoChainSpec;

pub fn kumandra_config() -> Result<KumandraChainSpec, String> {
	KumandraChainSpec::from_json_bytes(&include_bytes!("../chain-specs/kumandra.json")[..])
}

/// The default parachains host configuration.
#[cfg(any(
	feature = "kumandra-native"
))]
#[cfg(any(
	feature = "kumandra-native"
))]
#[test]

#[cfg(feature = "kumandra-native")]
fn kumandra_session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> kumandra::SessionKeys {
	kumandra::SessionKeys {
		babe,
		grandpa,
		im_online,
		authority_discovery,
	}
}

#[cfg(feature = "kumandra-native")]
fn kumandra_staging_testnet_config_genesis(wasm_binary: &[u8]) -> kumandra::GenesisConfig {
	// subkey inspect "$SECRET"
	let endowed_accounts = vec![];

	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)> = vec![];

	const ENDOWMENT: u128 = 1_000_000 * DOT;
	const STASH: u128 = 100 * DOT;

	kumandra::GenesisConfig {
		system: kumandra::SystemConfig { code: wasm_binary.to_vec() },
		balances: kumandra::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.map(|k: &AccountId| (k.clone(), ENDOWMENT))
				.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
				.collect(),
		},
		indices: kumandra::IndicesConfig { indices: vec![] },
		session: kumandra::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						kumandra(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
		},
		staking: kumandra::StakingConfig {
			validator_count: 50,
			minimum_validator_count: 4,
			stakers: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.1.clone(), STASH, kumandra::StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			force_era: Forcing::ForceNone,
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		},
		phragmen_election: Default::default(),
		democracy: Default::default(),
		council: polkadot::CouncilConfig { members: vec![], phantom: Default::default() },
		technical_committee: polkadot::TechnicalCommitteeConfig {
			members: vec![],
			phantom: Default::default(),
		},
		technical_membership: Default::default(),
		babe: kumandra::BabeConfig {
			authorities: Default::default(),
			epoch_config: Some(kumandra::BABE_GENESIS_EPOCH_CONFIG),
		},
		grandpa: Default::default(),
		im_online: Default::default(),
		authority_discovery: polkadot::AuthorityDiscoveryConfig { keys: vec![] },
		// claims: polkadot::ClaimsConfig { claims: vec![], vesting: vec![] },
		vesting: kumandra::VestingConfig { vesting: vec![] },
		treasury: Default::default(),
		nomination_pools: Default::default(),
	}
}

/// Returns the properties for the [`KumandraChainSpec`].
pub fn kumandra_chain_spec_properties() -> serde_json::map::Map<String, serde_json::Value> {
	serde_json::json!({
		"tokenDecimals": 12,
	})
	.as_object()
	.expect("Map given; qed")
	.clone()
}

/// Kumandra staging testnet config.
#[cfg(feature = "kumandra-native")]
pub fn kumandra_staging_testnet_config() -> Result<KumandraChainSpec, String> {
	let wasm_binary = kumandra::WASM_BINARY.ok_or("Kumandra development wasm not available")?;
	let boot_nodes = vec![];

	Ok(KumandraChainSpec::from_genesis(
		"Kumandra Staging Testnet",
		"kumandra_staging_testnet",
		ChainType::Live,
		move || kumandra_staging_testnet_config_genesis(wasm_binary),
		boot_nodes,
		Some(
			TelemetryEndpoints::new(vec![(KUMANDRA_STAGING_TELEMETRY_URL.to_string(), 0)])
				.expect("Kumandra Staging telemetry url is valid; qed"),
		),
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Some(kumandra_chain_spec_properties()),
		Default::default(),
	))
}


pub fn versi_chain_spec_properties() -> serde_json::map::Map<String, serde_json::Value> {
	serde_json::json!({
		"ss58Format": 42,
		"tokenDecimals": 12,
		"tokenSymbol": "VRS",
	})
	.as_object()
	.expect("Map given; qed")
	.clone()
}


/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(
	seed: &str,
) -> (
	AccountId,
	AccountId,
	BabeId,
	GrandpaId,
	ImOnlineId,
	AuthorityDiscoveryId,
) {
	let keys = get_authority_keys_from_seed_no_beefy(seed);
	(keys.0, keys.1, keys.2, keys.3, keys.4, keys.5)
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed_no_beefy(
	seed: &str,
) -> (
	AccountId,
	AccountId,
	BabeId,
	GrandpaId,
	ImOnlineId,
	AuthorityDiscoveryId,
) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

fn testnet_accounts() -> Vec<AccountId> {
	vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
		get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
		get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
	]
}

/// Helper function to create polkadot `GenesisConfig` for testing
#[cfg(feature = "kumandra-native")]
pub fn kumandra_testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	_root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> kumandra::GenesisConfig {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);

	const ENDOWMENT: u128 = 1_000_000 * KMD;
	const STASH: u128 = 100 * KMD;

	kumandra::GenesisConfig {
		system: kumandra::SystemConfig { code: wasm_binary.to_vec() },
		indices: kumandra::IndicesConfig { indices: vec![] },
		balances: kumandra::BalancesConfig {
			balances: endowed_accounts.iter().map(|k| (k.clone(), ENDOWMENT)).collect(),
		},
		session: kumandra::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						kumandra_session_keys(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
		},
		staking: kumandra::StakingConfig {
			minimum_validator_count: 1,
			validator_count: initial_authorities.len() as u32,
			stakers: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.1.clone(), STASH, kumandra::StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			force_era: Forcing::NotForcing,
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		},
		phragmen_election: Default::default(),
		democracy: kumandra::DemocracyConfig::default(),
		council: kumandra::CouncilConfig { members: vec![], phantom: Default::default() },
		technical_committee: kumandra::TechnicalCommitteeConfig {
			members: vec![],
			phantom: Default::default(),
		},
		technical_membership: Default::default(),
		babe: kumandra::BabeConfig {
			authorities: Default::default(),
			epoch_config: Some(polkadot::BABE_GENESIS_EPOCH_CONFIG),
		},
		grandpa: Default::default(),
		im_online: Default::default(),
		authority_discovery: kumandra::AuthorityDiscoveryConfig { keys: vec![] },
		// claims: polkadot::ClaimsConfig { claims: vec![], vesting: vec![] },
		vesting: kumandra::VestingConfig { vesting: vec![] },
		treasury: Default::default(),
		// hrmp: Default::default(),
		nomination_pools: Default::default(),
	}
}

#[cfg(feature = "kumandra-native")]
fn kumandr_development_config_genesis(wasm_binary: &[u8]) -> kumandra::GenesisConfig {
	kumandra_testnet_genesis(
		wasm_binary,
		vec![get_authority_keys_from_seed_no_beefy("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}


/// Kumandra development config (single validator Alice)
#[cfg(feature = "kumandra-native")]
pub fn kumandra_development_config() -> Result<KumandraChainSpec, String> {
	let wasm_binary = kumandra::WASM_BINARY.ok_or("Kumandra development wasm not available")?;

	Ok(KumandraChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		move || kumandra_development_config_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Some(kumandra_chain_spec_properties()),
		Default::default(),
	))
}

#[cfg(feature = "kumandra-native")]
fn kumandra_local_testnet_genesis(wasm_binary: &[u8]) -> kumandra::GenesisConfig {
	kumandra_testnet_genesis(
		wasm_binary,
		vec![
			get_authority_keys_from_seed_no_beefy("Alice"),
			get_authority_keys_from_seed_no_beefy("Bob"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Kumandra local testnet config (multivalidator Alice + Bob)
#[cfg(feature = "kumandra-native")]
pub fn kumandra_local_testnet_config() -> Result<KumandraChainSpec, String> {
	let wasm_binary = kumandra::WASM_BINARY.ok_or("Kumandra development wasm not available")?;

	Ok(KumandraChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		move || kumandra_local_testnet_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Some(kumandra_chain_spec_properties()),
		Default::default(),
	))
}