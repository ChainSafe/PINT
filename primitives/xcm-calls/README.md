# Primitives for cross chain Messages

This module contains bindings for calls of various FRAME pallets:

* [Assets Pallet](https://crates.parity.io/pallet_assets/pallet/index.html)
* [Proxy Pallet](https://crates.parity.io/pallet_proxy/pallet/index.html)
* [Staking Pallet](https://crates.parity.io/pallet_staking/index.html)

Since the generic datatypes of a pallet are dependent on their runtime configuration of a parachains, the encoding to use when sending a [`Xcm::Transact`](https://github.com/paritytech/xcm-format#transact) is depending on the destination of a cross chain message.

In order for the call to be decodable on the target chain (see Polkadot's [`XcmExecutor`](https://github.com/paritytech/polkadot/tree/master/xcm/xcm-executor)), it must be encoded with the corresponding index of the pallet, which also depends on the runtime configuration of the parachain.

This provides module provides an excerpt from each pallet's call variants that are used for PINT's cross chain operations.
Each Pallet includes their own encoder type that expects encoders for every generic datatype of the pallet.