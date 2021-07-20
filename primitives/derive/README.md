# derive

This module contains `proc-macro`s for developing and testing.

## #[xcm_error]

Provides a `From<xcm::v0::Error>` implementation, could be used for `pallet::Error<T>`. 

For example:

```rust
#[pallet:error]
#[xcm_error]
pub enum Error<T> {
    // ...
}
```
