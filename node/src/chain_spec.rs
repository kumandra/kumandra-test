use kumandra_runtime::{
	AccountId, BabeConfig, BalancesConfig, GenesisConfig, GrandpaConfig, Signature, SudoConfig,
	SystemConfig, WASM_BINARY,
};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_consensus_babe::AuthorityId as BabeId;


// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

pub fn kumandra_testnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../res/testnet.json")[..])
}

fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys { grandpa, babe, im_online, authority_discovery }
}


/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
	seed: &str,
) -> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}


pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Propertiesselasela
		None,
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
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
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		None,
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
		vec![
			// get_account_id_from_seed::<sr25519::Public>("Alice"),
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
	});

	let num_endowed_accounts = endowed_accounts.len();
	const ENDOWMENT: Balance = 2021 * DOLLARS;
	const STASH: Balance = 1006 * DOLLARS;


	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENDOWMENT))
				.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
				.collect(),
		},
		indices: IndicesConfig { indices: vec![] },
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					x.0.clone(),
					x.0.clone(),
					session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
				})
				.collect::<Vec<_>>(),
		},

		staking: StakingConfig {
			validator_count: 20, // Validator open up to 20
			minimum_validator_count: initial_authorities.len() as u32,
			staker: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
				.collect(), 
			invulnerables: initial_autho		// TODO! Pallet Demoncracyrities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		},
		democracy: DemocracyConfig::default(),
		elections_phragmen: Default::default(),
		collective_Instance1: Default::default(),
		collective_Instance2: TechnicalCommitteeConfig {
			members: vec![
				AccountId32::from_string("5GWjgushRVRgms7o54wKk8ZGitF6V6yHkmFBHZ8FmQKMdDAP")
					.unwrap(),
				AccountId32::from_string("5FWJYoLowtLAcpPrDbVkg4s79sf44w4r4xwhWQyEYXRHDUDF")
					.unwrap(),
				AccountId32::from_string("5CaZHv97m7iuMPFLd4Bnnt6Av2cPRCHfURqpHb94aVaBteRB")
					.unwrap(),
				AccountId32::from_string("5He9UnqWYVqDgBuxxrqGfxZWnuZ3ke7AhqPnyRkukrEpRrrc")
					.unwrap(),
				AccountId32::from_string("5DCPX1kxcd5qZxmcUoypYCBu9zYvgU8QxL7Ff3CRFKgmMRi9")
					.unwrap(),
				AccountId32::from_string("5Dt43fTPLRDJXcKNAXWtRhcrSA89LLMgZbRrgzaUshUxM8sn")
					.unwrap(),
				AccountId32::from_string("5DPt9hfH5ea8R6A3dGsACj7GsnG9U3LGdykyGjSLRMynqztG")
					.unwrap(),
				AccountId32::from_string("5F1KPuXvHNr3p18yAWMzbvioWg61EucVgRaznx8tDVW9FdX4")
					.unwrap(),
			],
			phantom: Default::default(),
		},
		contract: ContractConfig { 
			current_schedule: pallet_contracts::Schedule {
				enable_println, // this should only be enabled on development chains
				..Default::default()
			},
		},
		babe: BabeConfig { authorities: vec![] }
		grandpa: GrandpaConfig { authorities: vec![] },
		im_online: ImOnlineConfig { keys: vec![] },
		authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		// membership_Instance1: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		transaction_payment: Default::default(),
	}
}

fn get_properties() -> Map<String, Value> {
	let mut properties = Map::new();
	properties.insert("tokenSymbol".into(), "KMD".into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties
}

fn testnet_net_config_genesis() -> GenesisConfig {
	let initial_authorities = get_staging_initial_authorities();

	let root_key = get_root();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	testnet_genesis(initial_authorities, root_key, Some(endowed_accounts), false)
}

fn get_staging_initial_authorities(
) -> Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> {
	let initial_authorities = vec![
		(
			// 5Hgt9miSmGDomtJS72yfmSAYQbJ4Hnah7nNJWAMuzdM28aj5
			hex!["f8c4a8504c1873b53df6a225f99c5cee33926aa3c1e98fa98b34bd1c1f2d7c21"].into(),
			// 5C57EUFwbz3ATtyKJbsqVXprfpd5kXoVLL7CPTYuFMFBUMok
			hex!["004eb0b66eba36cbc8f76e12b733c65cc898cda91d4067e020c4dfca2f8b8f42"].into(),
			// 5FN5tufjQNkvyu8Ym982XGvrKeZmiFJciXLkbLqfZmU19oiP
			hex!["91f60f93f06e1fdc057f56176a726290ae0c63705af42413a54e3ddaf62b8a81"]
				.unchecked_into(),
			// 5F49xULpJFZVCyuPNMbPM1cHNaBemhDgUBeVeju1mY8KqiAB
			hex!["844950ca6191cf1bf094c667a8d8325049b63ff6266004923b83f7cffc355808"]
				.unchecked_into(),
			// 5EFnQ5feZ1WJ2CzwRQSD6VgpAwNjCYm2dWodDwegL9WcSYfq
			hex!["60eb7468ed2e6f3d9202779e4d4f453c7715d750d12fbf952a8e2154d4a85126"]
				.unchecked_into(),
			// 5E7ZbP6SYZVvKHv8Ucg1DZvFjqUsAf2iff3bFBrZpRfSLrcW
			hex!["5aa662ade2ae26abecf68fd36428de6ccc0f90a2752fed0eb89707376b57883f"]
				.unchecked_into(),
		),
		(
			// 5GZ4jLtN3gTuobtZHx6a1QsjHvC5KvpnBZX6HRgT5GgYqYti
			hex!["c6921281a0e03e0f45d4b91ccf24ac58d84665085f43587e439d11d316bbef62"].into(),
			// 5CdTuB9bmhi3Jtc4v8jtKBzMrFVy3kwf6rJiG6khTTqFuzRY
			hex!["18fc1d9b42b69ab24dd4a545754d8cc0e003bdb456fb54d178112154b592dd33"].into(),
			// 5EFG9mVtrwGfPDDAtMyczZtZ2ogp864yytyWuU9iJZ3pHHKt
			hex!["6085a28e9334aa863a7cb7676b4a4eaf1070eadb0cbebfca40cdb400f3c44b45"]
				.unchecked_into(),
			// 5E6m945UX7Verbnf36uDejWy7TQzVnxu4FSocjvHNJaLGeMP
			hex!["5a0a01f22da54bb6dfdc47e43d5d505a9f1470bde22827c9f04aaefa36ad7149"]
				.unchecked_into(),
			// 5GTN2JEJ5KWEwARJKZ3bTesZX87gphCiLzydUS1bKfvXykDR
			hex!["c238cf9c7cff72b36ad7adf4cb74b1d77f50463e9c9520249ecc9c1539a0e076"]
				.unchecked_into(),
			// 5CY3uMvTg2vx7NcxLyH9JeJLHzbJN2WZiUYri6YcSQHwHNT9
			hex!["14db1bb37d60c5c0912be231a8516f47fcce7375de310c9c5c415284fe5f096a"]
				.unchecked_into(),
		),
		(
			// 5D7buehzNYvEDkJCXTQLoAAEmnY8HQirk3QGQUX9EdBTKPeZ
			hex!["2e71f1e84430db366ac13e75b369a21dac6c66e3b7153127ed423fa44cb4850e"].into(),
			//  5DtaDxUfQ3Ds8XMMa4EHLDRDzrBNDFGh26GLVs6jyHvmQ3Pm
			hex!["50be516b3e8c0a74832d2713b91cf148558ed31b51b13b679cde820b02bcd675"].into(),
			// 5HDmeD3jMbU45SWphBVET27qMCScyqwLHSX28C4hntnwLhvK
			hex!["e4171e2fd1c49f82257310cf21837aef11b5b86b8757ec44c081e36b6c44ba2e"]
				.unchecked_into(),
			// 5EExkknokfbq2RFdfR5yuWogCSGr5vhGMo9EPuGkuddSc2ho
			hex!["604b12800205ef5a6c59d27a64bb2eaedc0ba7feea8ef7704d958e285f41d564"]
				.unchecked_into(),
			// 5CnhFw8grMmGEjqcsadUGdPo4CTDJqRwDQE6UsEE8mcr4kYk
			hex!["20064a7ed907cc8494fcc4e3c317eb1ad1be983f7848c12b79cd4c63c0870d37"]
				.unchecked_into(),
			// 5DyPjbBTyB7bLAL1aEK2iQVCas6PM13awBGPwtaK98t6qAjD
			hex!["546b3c530d76f1b312bd1ad098451a35924a26b70793a5ec80bcd4e585078a34"]
				.unchecked_into(),
		),
		(
			//  5E2EsB121e3anyAKhAfDoHCn4BkLQu4UVPGLY4aKPRT3TZcv
			hex!["56971d82ae42c93cbbf9a1a8da7f080a47387448c62dbb877d306ad609a65c50"].into(),
			// 5CS4MzjuPo8P5vwM82ZfxYv2EdrmrUkFX59YhsS3TNSCZMRa
			hex!["10492e80fb6135b3d5447228b275457a7be2ff34cbc91c4b15c80f2133607747"].into(),
			// 5GY9zGr8yGSyXc1PL2ZAZN4uHExKo3aNgPGCpmzvDgsbMCz3
			hex!["c5e086627b2c950eebcc6aba173560f3ffb184818172f10d398437179faf995e"]
				.unchecked_into(),
			// 5FZKDnd6XtfVHR6GMW5je1DEctb5fMfAAFpyrpSHgNGhKFTY
			hex!["9a869e2ee8fed7306841ec2e66763966834c816d5c724e269c431a7aa33eb06f"]
				.unchecked_into(),
			// 5CXnt7ErGmEu9asrx5xmDxQpbhwgKh3ExXPL78XVubkKnVa9
			hex!["14a88a7d420bedda32f6854a758f1352a6ac1357645640b12f67486c7cd11a49"]
				.unchecked_into(),
			// 5FpPVM8dyStQ9RQ1j5XBRo15k7LSdFfSi1r3DsNM1SncuXAd
			hex!["a605aad3dfb8dfdd8036983cfc3265c81e5f621c9698047db1eebe7bf1550935"]
				.unchecked_into(),
		),
	];
	initial_authorities
}

fn get_root() -> AccountId {
	hex!["942b48158d635dd0f7924031f5823cb4142d449533df32c0a5330b0842d7fc4e"].into()
}