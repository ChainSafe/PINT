---
layout: default
title: Command-Line
permalink: /usage/command-line/
---

## Commands

### Build

This will build the
[Wasm Runtime](https://substrate.dev/docs/en/knowledgebase/advanced/executor#wasm-execution) and
[native](https://substrate.dev/docs/en/knowledgebase/advanced/executor#native-execution) of PINT:

```
cargo build --release
```

### Test

Run all tests
```
cargo test
```

Run all tests, including benchmarks
```
cargo test --all-features
```


### Start the PINT chain

```
./target/release/pint --tmp --dev --alice --rpc-cors all
```

`--rpc-cors all` ensures that you can connect to the node via the [polkadot.js UI](https://polkadot.js.org/apps/#/explorer)