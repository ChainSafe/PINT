// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use frame_support::dispatch::DispatchResult;

/// The trait that provides balances related info about the parachain's various
/// sovereign accounts.
///
/// Definitions:
///  - *Sovereign Account* is an account controlled by a particular Consensus
///    System, within some other Consensus System: The account on the relay
///    chain controlled by the PINT parachain.
///  - *Stash Account* holds funds bonded for staking. If a remote asset (DOT)
///    supports staking then PINT can bond funds that it holds in the sovereign
///    account on the remote chain.
///   Meaning as soon as remote assets are bonded from PINT's sovereign account
///   on a target chain this sovereign account becomes a *stash account*. Both
///   terms now describe one and the same account and are therefore used in the
///   following interchangeably for the same account, even if the remote asset
///   does not support staking.
///
/// Staking rewards are not tracked since it is intended that the generated
/// staking rewards are routinely exchanged via an AMM for PINT. Some of the
/// resulting PINT will be allocated to the Treasury and StakerDAO, with the
/// remainder being burned. This does not affect the staked funds itself, so we
/// only consider two states the funds can have: either free (not bonded), or
/// not free (bonded or unbonded but not withdrawn yet.)
pub trait BalanceMeter<Balance, AssetId> {
    /// The assumed balance of the PINT's parachain sovereign account on the
    /// asset's native chain that is currently not bonded or otherwise locked.
    fn free_stash_balance(asset: AssetId) -> Balance;

    /// Ensures that the given amount can be removed from the parachain's
    /// sovereign account without falling below the configured
    /// `minimum_stash_balance`
    fn ensure_free_stash(asset: AssetId, amount: Balance) -> DispatchResult;

    /// Returns the configured minimum stash balance below which the parachain's
    /// sovereign account balance must not fall.
    fn minimum_free_stash_balance(asset: &AssetId) -> Balance;
}
