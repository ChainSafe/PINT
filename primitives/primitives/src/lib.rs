// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Primitive types used within PINT

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::sp_runtime::app_crypto::sp_core;
use frame_support::sp_runtime::traits::{IdentifyAccount, Verify};
use frame_support::sp_runtime::{generic, MultiSignature};

pub mod traits;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Signed version of Balance
pub type Amount = i128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Identifier for an asset.
pub type AssetId = u32;

/// Identifier for price feeds.
pub type FeedId = u64;

/// Value type for price feeds.
pub type Value = u128;

pub mod fee {
    use codec::{Decode, Encode};

    /// Represents the fee rate where fee_rate = numerator / denominator
    #[derive(Debug, Encode, Decode, Copy, Clone, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub struct FeeRate {
        pub numerator: u32,
        pub denominator: u32,
    }

    impl Default for FeeRate {
        fn default() -> Self {
            // 0.3%
            Self {
                numerator: 3,
                denominator: 1_000,
            }
        }
    }

    pub trait BaseFee
    where
        Self: Sized,
    {
        /// Returns the given amount after applying the fee rate: `self - fee`
        fn without_fee(&self, rate: FeeRate) -> Option<Self>;

        /// Returns the fees only.
        fn fee(&self, rate: FeeRate) -> Option<Self>;
    }

    impl BaseFee for super::Balance {
        fn without_fee(&self, rate: FeeRate) -> Option<Self> {
            self.checked_mul(rate.denominator as Self)?
                .checked_div(rate.denominator as Self + rate.numerator as Self)
        }

        fn fee(&self, rate: FeeRate) -> Option<Self> {
            self.checked_mul(rate.numerator as Self)?
                .checked_div(rate.denominator as Self)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_fee_calculations() {
            let rate = FeeRate {
                numerator: 3,
                denominator: 1_000,
            };

            assert_eq!(1_003.without_fee(rate), Some(1_000));
            assert_eq!(1_003.fee(rate), Some(3));
        }
    }
}
