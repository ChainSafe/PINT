---
layout: default
title: Architecture
permalink: /architecture/
---

# Architecture

This document describes the high-level architecture of the PINT codebase.

You might also find the higher-level conceptual documentation of PINT
helpful: [docs.polkadotindex.com](https://docs.polkadotindex.com/)

See also these implementation-related community blog posts:

* https://polkadotindex.substack.com/p/pint-update-1
* https://polkadotindex.substack.com/p/pint-community-update-2

## Terminology

__SAFT__ (_Simple Agreement for Future Token_) represent asset contributions (from project) that may become liquid
tokens in the future. Their value will be updated regularly

__NAV__ (_Net Asset Value_) represents:

* the net value of an asset in the index and is calculated by multiplying the per-unit price with the total amount of
  tokens held in the index.
* In the context of the index token itself (PINT) this however represents the unit value of the token and is calculated
  by taking the value of each of the underlying tokens over the total supply of PINT:

<div style="text-align:center"><img src="./imgs/nav-formula.png"  alt="NAV formula"/></div>

## Pallets

All custom PINT pallets are located in the [pallets](../pallets) folder in the root of the repository which includes.

### Committee Pallet

This includes the core governance mechanics. PINT Governance will be administered in two layers: the PINT Council, and
the Constituent Committee. However, we treat both holistically as a `Committee` within the code base. The pallet
includes a set of privileged accounts that are either of type `MemberType::Council` or `MemberType::Constituent`.

#### Voting

All proposals are described as dispatchable calls, which means that any method on any other pallet can be executed as
the result of a proposal. For example, a proposal to transfer 100 PINT to a community member from the treasury would
take the form of a call to the Treasury pallet: `Treasury::withdraw(recipient, 100)`. Proposals need to reach a majority
within a certain voting period for them to be executable.

### Local Treasury Pallet

Manages PINT exclusively. The treasury is a single account which is derived from the configured `PalletId`. It maintains
ownership of various assets and is controlled by the Governance Committee. Deposits to the Treasury can be done by
simply transferring funds to its AccountId. The committee can execute proposals to withdraw funds from the Treasury.

### Remote Treasury Pallet

Similar to the local treasury, but this pallet will be responsible for DOT (or other liquid assets in the Network). It
will execute withdrawals via XCM.

### Price feed Pallet

In order to exchange assets for PINT, the value of those assets as well as PINT’s NAV needs to be computed. This pallet
is an abstraction over the [`chainlink-feed-pallet`](https://github.com/smartcontractkit/chainlink-polkadot) which
provides oracle data from the chainlink network. It provides a lookup table for mapping PINT internal asset identifiers
with the chainlink pallet internal price feed identifiers (`AssetId -> FeedId`). Since the NAV calculation requires that
all asset values are measured in the same currency. All price feeds are expected to be configured with the same quote
currency (e.g. USD). This currency will then also be the currency the NAV is measured in, since the NAV of the index is
then `PINT/USD`, effectively the on-chain price of PINT.

### SAFT Registry Pallet

This pallet consists of records of off-chain SAFTs. An SAFT asset can have multiple SAFT Records. Each `SAFTRecord`
holds the number of units of its asset and their value. This value is expected to in the same currency the liquid assets
use for their price feeds, so that the NAV can easily be calculated according to the NAV formula. The SAFT registry
pallet requires the `AssetRecorder` trait which is an abstraction over the features for adding/removing assets, which is
implemented for the `AssetIndex`. Adding a SAFT record will call into the `AssetRecorder::add_saft` function, in mints
new PINT according to the value of the SAFT record. SAFTs can be converted to liquid tokens once they're available in
the network with a location.

### Asset Index Pallet

This pallet provides all user facing extrinsics for depositing assets (`AssetIndex::deposit`) and redeeming
PINT (`AssetIndex::withdraw`). It provides all NAV related calculations which is abstracted over the `NavProvider`
trait. Assets are distinguished by their kind, either they are liquid, or SAFT. The value of liquid assets will be
calculated using the price feeds whereas the value of a SAFT asset is determined by the total value of all SAFT records.
Since the SAFT records are stored in the `SaftRegistry` the `AssetIndex` requires the `SaftRegistry` trait abstraction
for that.

#### Depositing

Any user may deposit liquid assets. A new liquid asset class is created either by registering it with its native
location in the network  (`AssetIndex::register`) or by being added via  (`AssetIndex::add_asset`). In order to be able
to deposit, a user must first send liquid assets (e.g.) from its native location (e.g. relay chain) to PINT, upon which
it gets stored in the user
personal [`MultiCurrency`](https://github.com/open-web3-stack/open-runtime-module-library/tree/master/currencies)
balance. This process will be handled by the `LocalAssetTransactor` configured in the runtime which transacts
incoming `Xcm::Deposit` messages into deposit into the sender's `MultiCurrency` balance for that very asset.
Since `deposit` is fallible this is a 2-step process:

1. send x amount of liquid asset A from chain A to PINT.
2. call  `AssetIndex::deposit(A, x)`

The deposit call determines how much PINT the amount is worth and transfers the asset amount to the index account and
mints PINT in exchange into the caller's PINT balance. The total value of the index increases by the value of the
deposited amount, but the NAV will stay consistent since the newly minted PINT will counteract that.

#### Withdrawing

All new deposits (received PINT) are locked for a certain period and timestamped to calculate withdrawal fees based on
time spent in the index. After the `LockupPeriod` is over PINT can be redeemed by the user for a distribution of the
underlying _liquid_ assets. The liquid assets in the index are awarded to the redeemer, totalling the value of the
redeemed PINT, but in proportion to their representation in the index. This will lock the pending withdrawals for a
certain `WithdrawalPeriod` kick off the unbonding process in which the `RemoteAssetManager` will ensure that the pending
withdrawals are available on their native location. If for example a pending withdrawal exceeds the amount held as a
reserve (e.g. _not_ staked) then they need to be unbonded first. After the `WithdrawalPeriod` a user may try to complete
their withdrawals (`AssetIndex::complete_withdrawal`), which will try to complete every single `AssetWithdrawal` but it
will only get closed entirely after the last `AssetWithdrawal` was completed.

### Remote Asset Manager Pallet

This pallet provides all the cross chain capabilities PINT relies on. Most importantly bonding and unbonding liquid
assets. Remote Staking is still experimental and rather complex endeavor. The remote asset manager comes with bindings
for the substrate FRAME `pallet_staking`. Assets that were sent to PINT using XCM reserved based transfer are kept in an
account derived from PINT's parachain ID (sovereign account) to which only the PINT parachain has access to during XCM
execution. In other words, all liquid assets that were deposited to the index (e.g. DOT) are sitting in an account on
the asset's native location (e.g. relay chain) to which the PINT parachain has access via XCM. Withdrawing an amount of
liquid assets from PINT, will then transfer the amount from this account back to recipient included in the destination
of the XCM message. XCM also supports dispatching calls on other chains whose origin then will be the sovereign account
of the parachain. This means funds held in PINT's sovereign account can be staked by calling the necessary functions of
the staking pallet, staked assets are called `active`, the default is `idle`. In order to execute a `Xcm::Transact` the
payload (dispatchable) must be encoded as it would be represented by the `Call` enum of the runtime of the xcm's
destination. The encoding of a dispatchable call in the `Runtime::Call` enum is further dependent on the index of the
pallet as it is configured in the `construct_runtime!` macro. To make things worse, the encoding of course depends on
the runtime specific types. So in order to generalize remote staking it is required that:

* the index of `pallet_staking` in the runtime is known
* the remote asset manager can encode the runtime specific types. For the `pallet_staking` these are: `AccountId`
  , `Balance`, `LookupSource`.

The remote handles this by requiring the `StakingCallEncoder` type, which takes care of asset specific encoding
of `pallet_staking` dispatchables.

It also has experimental support for transferring PINT
to [Statemint parachain](https://medium.com/polkadot-network/statemint-generic-assets-chain-proposing-a-common-good-parachain-to-polkadot-governance-d318071b238)