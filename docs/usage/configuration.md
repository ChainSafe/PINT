---
layout: default title: Configuration permalink: /usage/configuration/
---

# Configuration

## Polkadot JS UI

In order to use [polkadot.js UI](https://polkadot.js.org/apps/#/explorer) to interact with the chain you need to specify
the custom PINT is using by copying the [types.json](../../resources/types.json) object into the input
at `Settings > Devoloper` in the polkadot js UI menu.

## Chain Spec

By default, PINT uses the [dev chain spec](../../node/src/chain_spec/dev.rs).

The `GenesisConfig` configures the initial chain state at genesis.

Excerpt:

```rust
GenesisConfig {
    system: SystemConfig {
        code: WASM_BINARY.expect("WASM binary was not build, please build it!").to_vec(),
        changes_trie_config: Default::default (),
    },
    // This ensures the `endowed_accounts` have funds in their accounts
    balances: BalancesConfig { balances: endowed_accounts.iter().cloned().map( | k| (k, 1 < < 12)).collect() },
    // This configures the comittee
    committee: CommitteeConfig { council_members: council_members.clone(), ..Default::default () },
    // all council members can create feeds
    chainlink_feed: ChainlinkFeedConfig { feeds: Default::default(), pallet_admin: Some(root_key.clone()), feed_creators: council_members },
    sudo: SudoConfig { key: root_key },
    parachain_info: ParachainInfoConfig { parachain_id: id },
}
```

To run the chain with a custom chainspec we need to provide the path to your chainspec json file:

*NOTE:* the id of your custom chain spec should contain `dev` in order to run it as standalone chain. 

```
./target/release/pint --tmp --chain=<custom-chainspec.json>
```

Read more about substrate's [Chain Specification](https://substrate.dev/docs/en/knowledgebase/integrate/chain-spec)
and [creating private networks](https://substrate.dev/docs/en/tutorials/start-a-private-network)

#### Build the chainspec

./target/release/pint build-spec \
--disable-default-bootnode > pint-local-plain.json

#### Build the raw chainspec file

./target/release/pint build-spec \
--chain=./pint-local-plain.json \
--raw --disable-default-bootnode > pint-local-raw.json