// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use codec::{Decode, Encode};
use frame_support::sp_runtime::{traits::Saturating, RuntimeDebug};

/// Represents the balance of a single asset.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct AccountBalance<Balance> {
    /// Non-reserved part of the balance. There may still be restrictions on
    /// this, but it is the total pool what may in principle be transferred,
    /// reserved.
    ///
    /// This is the only balance that matters in terms of most operations on
    /// tokens.
    pub available: Balance,
    /// Balance which is currently locked and can't be accessed by the user.
    ///
    /// This is intended to reserve an amount of this asset for PINT related operations, so that it can be spend.
    pub reserved: Balance,
}

impl<Balance: Saturating + Copy + Ord> AccountBalance<Balance> {
    /// The total balance that is currently reserved and available
    pub fn total_balance(&self) -> Balance {
        self.available.saturating_add(self.reserved)
    }
}
