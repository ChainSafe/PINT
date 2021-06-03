// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::Origin;

benchmarks! {
    transfer {

    }: _(
        <Origin<T>>::Signed(whitelisted_caller()),
        10_000_u32.into()
    ) verify {

    }
}
