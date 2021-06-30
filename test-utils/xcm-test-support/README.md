# Test runtime for relay chain

Import this directly with

```rust
#[path="../../../test-utils/xcm-test-support/src/lib.rs"] mod xcm_test_support;
```

Importing module must implement:

```rust
pub fn relay_ext() -> sp_io::TestExternalities {..}
```