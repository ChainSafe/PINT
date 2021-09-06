// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use super::{get_account_id_from_seed, get_collator_keys_from_seed, Extensions};
use cumulus_primitives_core::ParaId;
use frame_support::PalletId;
use pint_runtime_common::traits::XcmRuntimeCallWeights;
use pint_runtime_polkadot::*;
use sc_service::ChainType;
use sp_core::sr25519;
use sp_runtime::traits::{AccountIdConversion, Zero};
use xcm_calls::{
	proxy::{ProxyConfig, ProxyWeights},
	staking::{RewardDestination, StakingConfig, StakingWeights},
};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

pub fn pint_development_config(id: ParaId) -> ChainSpec {
	ChainSpec::from_genesis(
		// Name
		"PINT Development",
		// ID
		"pint_polkadot_dev",
		ChainType::Local,
		move || {
			pint_testnet_genesis(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// initial collators.
				vec![(get_account_id_from_seed::<sr25519::Public>("Alice"), get_collator_keys_from_seed("Alice"))],
				vec![
					PalletId(*b"Treasury").into_account(),
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
				],
				id,
			)
		},
		vec![],
		None,
		None,
		None,
		Extensions { relay_chain: "rococo-local".into(), para_id: id.into() },
	)
}

pub fn pint_local_config(id: ParaId) -> ChainSpec {
	ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"pint_polkadot_local_testnet",
		ChainType::Local,
		move || {
			pint_testnet_genesis(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// initial collators.
				vec![
					(get_account_id_from_seed::<sr25519::Public>("Alice"), get_collator_keys_from_seed("Alice")),
					(get_account_id_from_seed::<sr25519::Public>("Bob"), get_collator_keys_from_seed("Bob")),
				],
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
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
				],
				id,
			)
		},
		vec![],
		None,
		None,
		None,
		Extensions { relay_chain: "rococo-local".into(), para_id: id.into() },
	)
}

fn pint_testnet_genesis(
	root_key: AccountId,
	initial_authorities: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	council_members: Vec<AccountId>,
	id: ParaId,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			code: WASM_BINARY.expect("WASM binary was not build, please build it!").to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig { balances: vec![(root_key.clone(), 1 << 60)] },
		committee: CommitteeConfig { council_members: council_members.clone(), ..Default::default() },
		chainlink_feed: ChainlinkFeedConfig {
			feeds: Default::default(),
			pallet_admin: Some(root_key.clone()),
			feed_creators: council_members,
		},
		sudo: SudoConfig { key: root_key },
		parachain_info: ParachainInfoConfig { parachain_id: id },
		collator_selection: CollatorSelectionConfig {
			invulnerables: initial_authorities.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: Zero::zero(),
			..Default::default()
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.cloned()
				.map(|(acc, aura)| {
					(
						acc.clone(),                  // account id
						acc,                          // validator id
						opaque::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		tokens: TokensConfig {
			// TODO:
			//
			// this config is only for tests for now
			balances: vec![
				endowed_accounts.iter().cloned().map(|k| (k, 42, 1 << 60)).collect::<Vec<_>>(),
				endowed_accounts.iter().cloned().map(|k| (k, 43, 1 << 60)).collect::<Vec<_>>(),
			]
			.concat(),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		remote_asset_manager: RemoteAssetManagerConfig {
			staking_configs: vec![(
				42,
				StakingConfig {
					pallet_index: 7,
					reward_destination: RewardDestination::Staked,
					minimum_balance: 0,
					weights: StakingWeights::polkadot(),
					bonding_duration: POLKADOT_BONDING_DURATION_IN_BLOCKS,
				},
			)],
			proxy_configs: vec![(42, ProxyConfig { pallet_index: 29, weights: ProxyWeights::polkadot() })],
			statemint_config: None,
		},
	}
}
