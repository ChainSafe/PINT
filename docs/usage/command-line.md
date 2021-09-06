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
./target/release/pint --dev --instant-sealing
```

This will seal blocks instantly. The node will never produce blocks

If the [polkadot.js UI](https://polkadot.js.org/apps/#/explorer) fails to connect try adding `--rpc-cors all`.