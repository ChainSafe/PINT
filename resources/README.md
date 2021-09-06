This folder contains:

* [types.json](./types.json): The specific type configuration and custom datatypes the PINT runtime uses. This is
  required for the polkadot JS UI in order to properly connect to a running node. This also includes all custom types
  used by the [chainlink-feed-pallet](https://github.com/smartcontractkit/chainlink-polkadot)
* [pint-dev.json](pint-dev.json) A chain spec file exported via `./target/debug/pint build-spec --dev --disable-default-bootnode > resources/pint-dev.json`. All developer accounts are prefunded with `1 << 60` units. This can easily be adjusted to something else and then started as dev chain with `./target/debug/pint --tmp --chain pint-local-plain.json --instant-sealing `
* [pint-dev-with-chainlink-feed.json](pint-dev-with-chainlink-feed.json) contains a chainlink feed at genesis as described [here](https://github.com/smartcontractkit/chainlink-polkadot/tree/master/substrate-node-example/specs).