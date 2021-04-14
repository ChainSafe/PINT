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

Polkadot (rococo-v1 branch):
```
cargo build --release --features real-overseer

./target/release/polkadot build-spec --chain rococo-local --raw --disable-default-bootnode > rococo_local.json

./target/release/polkadot --chain ./rococo_local.json -d cumulus_relay1 --validator --bob --port 50555
./target/release/polkadot --chain ./rococo_local.json -d cumulus_relay0 --validator --alice --port 50556
```

Substrate Parachain Template:
```
# this command assumes the chain spec is in a directory named polkadot that is a sibling of the working directory
./target/release/parachain-collator -d local-test --collator --alice --ws-port 9945 --parachain-id 200 -- --chain ../polkadot/rococo_local.json
```
