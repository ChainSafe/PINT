// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use cumulus_primitives_core::ParaId;
use frame_support::PalletId;
use parachain_runtime::{AccountId, AuraId, Signature};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{AccountIdConversion, IdentifyAccount, Verify, Zero};
use xcm_calls::{
    proxy::{ProxyConfig, ProxyWeights},
    staking::{RewardDestination, StakingConfig, StakingWeights},
};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<parachain_runtime::GenesisConfig, Extensions>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
    get_from_seed::<AuraId>(seed)
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn pint_development_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        // Name
        "PINT Development",
        // ID
        "pint_dev",
        ChainType::Local,
        move || {
            pint_testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                // initial collators.
                vec![(
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_collator_keys_from_seed("Alice"),
                )],
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
        Extensions {
            relay_chain: "rococo-dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn pint_local_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        // Name
        "Local Testnet",
        // ID
        "local_testnet",
        ChainType::Local,
        move || {
            pint_testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                // initial collators.
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        get_collator_keys_from_seed("Alice"),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        get_collator_keys_from_seed("Bob"),
                    ),
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
        Extensions {
            relay_chain: "rococo-local".into(),
            para_id: id.into(),
        },
    )
}

fn pint_testnet_genesis(
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    council_members: Vec<AccountId>,
    id: ParaId,
) -> parachain_runtime::GenesisConfig {
    parachain_runtime::GenesisConfig {
        system: parachain_runtime::SystemConfig {
            code: parachain_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        },
        balances: parachain_runtime::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        },
        committee: parachain_runtime::CommitteeConfig {
            council_members: council_members.clone(),
            ..Default::default()
        },
        chainlink_feed: parachain_runtime::ChainlinkFeedConfig {
            pallet_admin: Some(root_key.clone()),
            feed_creators: council_members,
        },
        sudo: parachain_runtime::SudoConfig { key: root_key },
        parachain_info: parachain_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: parachain_runtime::CollatorSelectionConfig {
            invulnerables: initial_authorities
                .iter()
                .cloned()
                .map(|(acc, _)| acc)
                .collect(),
            candidacy_bond: Zero::zero(),
            ..Default::default()
        },
        session: parachain_runtime::SessionConfig {
            keys: initial_authorities
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                                     // account id
                        acc,                                             // validator id
                        parachain_runtime::opaque::SessionKeys { aura }, // session keys
                    )
                })
                .collect(),
        },
        tokens: parachain_runtime::TokensConfig {
            // TODO:
            //
            // this config is only for tests for now
            balances: vec![
                endowed_accounts
                    .iter()
                    .cloned()
                    .map(|k| (k, 42, 1 << 60))
                    .collect::<Vec<_>>(),
                endowed_accounts
                    .iter()
                    .cloned()
                    .map(|k| (k, 43, 1 << 60))
                    .collect::<Vec<_>>(),
            ]
            .concat(),
        },
        // no need to pass anything to aura, in fact it will panic if we do. Session will take care
        // of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        remote_asset_manager: parachain_runtime::RemoteAssetManagerConfig {
            staking_configs: vec![(
                42,
                StakingConfig {
                    pallet_index: 7,
                    max_unlocking_chunks: 42,
                    pending_unbond_calls: 42,
                    reward_destination: RewardDestination::Staked,
                    minimum_balance: 0,
                    weights: StakingWeights {
                        bond: 1000_u64,
                        bond_extra: 1000_u64,
                        unbond: 1000_u64,
                        withdraw_unbonded: 1000_u64,
                    },
                },
            )],
            proxy_configs: vec![(
                42,
                ProxyConfig {
                    pallet_index: 29,
                    weights: ProxyWeights {
                        add_proxy: 1000_u64,
                        remove_proxy: 1000_u64,
                    },
                },
            )],
        },
    }
}
