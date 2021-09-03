# Polkadot Index Network Token (PINT :beer:)

[![License: LGPL v3](https://img.shields.io/badge/License-LGPL%20v3-blue.svg)](http://www.gnu.org/licenses/lgpl-3.0)

A Polkadot ecosystem index for investors. A self sustaining auction treasury for parachains.

Organized by the Stateless Money validator, governed by a community that includes Polychain Capital, Hypersphere Ventures, HashKey Capital, Acala, and built by ChainSafe as a StakerDAO product. 

For more information on the project please visit [Polkadot Index Network Token](https://docs.polkadotindex.com/) documentation.

❗**Current development should be considered a work in progress.**

## Upstream

This project is a fork of the
[Substrate Developer Hub Node Template](https://github.com/substrate-developer-hub/substrate-node-template).

## Build & Run

Follow these steps to prepare a local Substrate development environment :hammer_and_wrench:

### Setup

This project currently builds against Rust nightly-2021-01-26. Assuming you have rustup already insatlled set up your local environment:

```shell
rustup install nightly-2021-01-26
rustup target add wasm32-unknown-unknown --toolchain nightly-2021-01-26
rustup override set nightly-2021-01-26
``` 

### Build

Once the development environment is set up, build the node template. This command will build the
[Wasm](https://substrate.dev/docs/en/knowledgebase/advanced/executor#wasm-execution) and
[native](https://substrate.dev/docs/en/knowledgebase/advanced/executor#native-execution) code:

```bash
cargo build --release
```

Note: If the build fails with `(signal: 9, SIGKILL: kill)` it has probably run out of memory. Try freeing some memory or build on another machine.

## Run

### Local Testnet

Polkadot (release-v0.9.x branch)

```
cargo build --release

./target/release/polkadot build-spec --chain rococo-local --raw --disable-default-bootnode > rococo_local.json

./target/release/polkadot --chain ./rococo_local.json -d cumulus_relay0 --validator --alice --port 9844

./target/release/polkadot --chain ./rococo_local.json -d cumulus_relay1 --validator --bob --port 9955
```

PINT Parachain:

```
# this command assumes the chain spec is in a directory named polkadot that is a sibling of the pint directory
./target/release/pint --collator --alice --chain pint-dev --ws-port 9945 --parachain-id 200 --rpc-cors all -- --execution wasm --chain ../polkadot/rococo_local.json --ws-port 9977 --rpc-cors all
```

### Registering on Local Relay Chain

In order to produce blocks you will need to register the parachain as detailed in the [Substrate Cumulus Workshop](https://substrate.dev/cumulus-workshop/#/en/3-parachains/2-register) by going to

Developer -> sudo -> paraSudoWrapper -> sudoScheduleParaInitialize(id, genesis)

Ensure you set the `ParaId` to `200` and the `parachain: Bool` to `Yes`.

```
cargo build --release
# Build the Chain spec
./target/release/pint build-spec --disable-default-bootnode > ./pint-local-plain.json
# Build the raw file
./target/release/pint build-spec --chain=./pint-local-plain.json --raw --disable-default-bootnode > ./pint-local.json


# export genesis state and wasm
./target/release/pint export-genesis-state --parachain-id 200 > ./resources/para-200-genesis
./target/release/pint export-genesis-wasm > ./para-200.wasm
```


* [polkadot-launch](https://github.com/paritytech/polkadot-launch) can be run by dropping the proper polkadot binary in the  `./bin` folder and
    * Run globally
        * `polkadot-launch config.json`
    * Run locally, navigate into polkadot-launch,
        * ``` yarn ```
        * ``` yarn start ```

### Documentation

## Commands

* `mkdocs serve` - Start the live-reloading docs server.
* `mkdocs build` - Build the documentation site.
* `mkdocs -h` - Print help message and exit.