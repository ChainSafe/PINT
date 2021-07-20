// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Additional constant values used in the runtime

#![allow(dead_code)]

pub mod time {
    use primitives::BlockNumber;
    /// This determines the average expected block time that we are targeting.
    /// Blocks will be produced at a minimum duration defined by
    /// `SLOT_DURATION`. `SLOT_DURATION` is picked up by `pallet_timestamp`
    /// which is in turn picked up by `pallet_aura` to implement `fn
    /// slot_duration()`.
    ///
    /// Change this to adjust the block time.
    pub const MILLISECS_PER_BLOCK: u64 = 6000;

    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    // Time is measured by number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}

pub mod fee {
    use primitives::Balance;

    // Unit = the base number of indivisible units for balances
    pub const UNIT: Balance = 1_000_000_000_000;
    pub const MILLIUNIT: Balance = 1_000_000_000;
    pub const MICROUNIT: Balance = 1_000_000;
}
