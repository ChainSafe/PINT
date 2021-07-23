// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # AssetIndex Pallet
//!
//! Tracks all the assets in the PINT index, composed of multiple assets

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod traits;
pub mod types;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit, clippy::large_enum_variant)]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::{
            traits::{
                AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, CheckedDiv, CheckedSub,
                Saturating, Zero,
            },
            FixedPointNumber, FixedU128,
        },
        sp_std::{convert::TryInto, prelude::*, result::Result},
        traits::{
            Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency, WithdrawReasons,
        },
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use orml_traits::{MultiCurrency, MultiReservableCurrency};
    use polkadot_parachain::primitives::Id as ParaId;
    use xcm::v0::{Junction, MultiLocation};

    use pallet_price_feed::{AssetPricePair, Price, PriceFeed};

    pub use crate::traits::AssetRecorder;
    use crate::types::{
        AssetAvailability, AssetMetadata, AssetWithdrawal, IndexTokenLock, PendingRedemption,
        RedemptionState,
    };
    use primitives::fee::{BaseFee, FeeRate};
    use primitives::traits::{MultiAssetRegistry, RemoteAssetManager};

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

    type Ratio = FixedU128;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin that is allowed to administer the index
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        /// Currency implementation to use as the index token
        type IndexToken: LockableCurrency<Self::AccountId, Balance = Self::Balance>;
        /// The balance type used within this pallet
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Into<u128>
            + BaseFee;
        /// Period after the minting of the index token for which 100% is locked
        /// up. Only applies to users contributing assets directly to
        /// index
        #[pallet::constant]
        type LockupPeriod: Get<Self::BlockNumber>;
        /// The identifier for the index token lock.
        /// Used to lock up deposits for `T::LockupPeriod`.
        #[pallet::constant]
        type IndexTokenLockIdentifier: Get<LockIdentifier>;
        /// The minimum amount of the index token that can be redeemed for the
        /// underlying asset in the index
        #[pallet::constant]
        type MinimumRedemption: Get<Self::Balance>;
        /// Minimum amount of time between redeeming index tokens
        /// and being able to withdraw the awarded assets
        #[pallet::constant]
        type WithdrawalPeriod: Get<Self::BlockNumber>;
        /// The maximum amount of DOT that can exist in the index
        #[pallet::constant]
        type DOTContributionLimit: Get<Self::Balance>;
        /// Type that handles cross chain transfers
        type RemoteAssetManager: RemoteAssetManager<Self::AccountId, Self::AssetId, Self::Balance>;
        /// Type used to identify assets
        type AssetId: Parameter + Member + AtLeast32BitUnsigned + Copy + MaybeSerializeDeserialize;

        /// The native asset id
        #[pallet::constant]
        type SelfAssetId: Get<Self::AssetId>;

        /// Currency type for deposit/withdraw assets to/from the user's
        /// sovereign account
        type Currency: MultiReservableCurrency<
            Self::AccountId,
            CurrencyId = Self::AssetId,
            Balance = Self::Balance,
        >;

        /// The types that provides the necessary asset price pairs
        type PriceFeed: PriceFeed<Self::AssetId>;

        /// The basic fees that apply when a withdrawal is executed
        #[pallet::constant]
        type BaseWithdrawalFee: Get<FeeRate>;

        /// The treasury's pallet id, used for deriving its sovereign account
        /// ID.
        #[pallet::constant]
        type TreasuryPalletId: Get<PalletId>;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The maximum length of a name or symbol stored on-chain.
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// The weight for this pallet's extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    /// (AssetId) -> AssetAvailability
    pub type Assets<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, AssetAvailability, OptionQuery>;

    #[pallet::storage]
    ///  (AccountId) -> Vec<PendingRedemption>
    pub type PendingWithdrawals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vec<PendingRedemption<T::AssetId, T::Balance, BlockNumberFor<T>>>,
        OptionQuery,
    >;

    #[pallet::storage]
    /// Tracks the locks of the minted index token that are locked up until
    /// their `LockupPeriod` is over  (AccountId) -> Vec<IndexTokenLockInfo>
    pub type IndexTokenLocks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vec<IndexTokenLock<T::BlockNumber, T::Balance>>,
        ValueQuery,
    >;

    #[pallet::storage]
    /// Tracks the amount of the currently locked index token per user.
    /// This is equal to the sum(IndexTokenLocks[AccountId])
    ///  (AccountId) -> Balance
    pub type LockedIndexToken<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance, ValueQuery>;

    #[pallet::storage]
    /// Metadata of an asset ( for reversed usage now ).
    pub(super) type Metadata<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        AssetMetadata<BoundedVec<u8, T::StringLimit>>,
        ValueQuery,
        GetDefault,
        ConstU32<300_000>,
    >;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub liquid_assets: Vec<(T::AssetId, ParaId)>,
        pub saft_assets: Vec<T::AssetId>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                liquid_assets: Default::default(),
                saft_assets: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for (asset, id) in self.liquid_assets.iter().cloned() {
                let availability = AssetAvailability::Liquid(
                    (Junction::Parent, Junction::Parachain(id.into())).into(),
                );
                Assets::<T>::insert(asset, availability)
            }
            for asset in self.saft_assets.iter().cloned() {
                Assets::<T>::insert(asset, AssetAvailability::Saft)
            }
        }
    }

    #[pallet::event]
    #[pallet::metadata(T::AssetId = "AccountId", AccountIdFor < T > = "AccountId", T::Balance = "Balance")]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new asset was added to the index and some index token paid out
        /// \[AssetIndex, AssetUnits, IndexTokenRecipient, IndexTokenPayout\]
        AssetAdded(T::AssetId, T::Balance, AccountIdFor<T>, T::Balance),
        /// An asset was removed from the index and some index token transferred
        /// or burned \[AssetId, AssetUnits, Account, Recipient,
        /// IndexTokenNAV\]
        AssetRemoved(
            T::AssetId,
            T::Balance,
            AccountIdFor<T>,
            Option<AccountIdFor<T>>,
            T::Balance,
        ),
        /// A new asset was registered in the index
        /// \[Asset, Availability\]
        AssetRegistered(T::AssetId, AssetAvailability),
        /// A new deposit of an asset into the index has been performed
        /// \[AssetId, AssetUnits, Account, PINTPayout\]
        Deposited(T::AssetId, T::Balance, AccountIdFor<T>, T::Balance),
        /// Started the withdrawal process
        /// \[Account, PINTAmount\]
        WithdrawalInitiated(AccountIdFor<T>, T::Balance),
        /// Completed a single asset withdrawal
        /// \[Account, AssetId, AssetUnits\]
        Withdrawn(AccountIdFor<T>, T::AssetId, T::Balance),
        /// Completed a pending asset withdrawal
        /// \[Account, Assets\]
        WithdrawalCompleted(
            AccountIdFor<T>,
            Vec<AssetWithdrawal<T::AssetId, T::Balance>>,
        ),
        /// New metadata has been set for an asset. \[asset_id, name, symbol,
        /// decimals\]
        MetadataSet(T::AssetId, Vec<u8>, Vec<u8>, u8),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Thrown if adding units to an asset holding causes its numerical type
        /// to overflow
        AssetUnitsOverflow,
        /// The given asset ID is unknown.
        UnknownAsset,
        /// Thrown if the given asset was the native asset and is disallowed
        NativeAssetDisallowed,
        /// Thrown if a SAFT asset operation was requested for a registered
        /// liquid asset.
        ExpectedSAFT,
        /// Thrown if a liquid asset operation was requested for a registered
        /// SAFT asset.
        ExpectedLiquid,
        /// Thrown when trying to remove liquid assets without recipient
        NoRecipient,
        /// Invalid metadata given.
        BadMetadata,
        /// Thrown if no index could be found for an asset identifier.
        UnsupportedAsset,
        /// Thrown if calculating the volume of units of an asset with it's
        /// price overflows.
        AssetVolumeOverflow,
        /// Thrown if the given amount of PINT to redeem is too low
        MinimumRedemption,
        /// Thrown when the redeemer does not have enough PINT as is requested
        /// for withdrawal.
        InsufficientDeposit,
        /// Thrown when calculating the NAV resulted in a overflow
        NAVOverflow,
        /// Thrown when to withdrawals are available to complete
        NoPendingWithdrawals,
        /// Thrown if the asset that should be added is already registered
        AssetAlreadyExists,
        /// Thrown when adding assets with zero amount or units
        InvalidPrice,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Callable by the governance committee to add new liquid assets to the
        /// index and mint the given amount IndexToken.
        /// The amount of PINT minted and awarded to the LP is specified as part
        /// of the associated proposal
        /// Caller's balance is updated to allocate the correct amount of the
        /// IndexToken. If the asset does not exist yet, it will get
        /// created with the given location.
        #[pallet::weight(T::WeightInfo::add_asset())]
        pub fn add_asset(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            units: T::Balance,
            location: MultiLocation,
            amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin.clone())?;
            let caller = ensure_signed(origin)?;
            if units.is_zero() {
                return Ok(().into());
            }

            let availability = AssetAvailability::Liquid(location);

            // check whether this is a new asset and make sure locations match otherwise
            let is_new_asset = if let Some(asset) = Assets::<T>::get(&asset_id) {
                ensure!(asset == availability, Error::<T>::AssetAlreadyExists);
                false
            } else {
                true
            };

            // transfer the caller's fund into the treasury account
            Self::add_liquid(&caller, asset_id, units, amount)?;

            // Store initial price pair if not exists
            T::PriceFeed::ensure_price(
                asset_id,
                Price::checked_from_rational(amount.into(), units.into())
                    .ok_or(<Error<T>>::InvalidPrice)?,
            )?;

            // register asset if not yet known
            if is_new_asset {
                Assets::<T>::insert(asset_id, availability.clone());
                Self::deposit_event(Event::AssetRegistered(asset_id, availability));
            }

            Self::deposit_event(Event::AssetAdded(asset_id, units, caller, amount));
            Ok(().into())
        }

        #[pallet::weight(10_000)] // TODO: Set weights
        /// Dispatches transfer to move assets out of the index’s account,
        /// if a liquid asset is specified
        /// Callable by an admin.
        ///
        /// Updates the index to reflect the removed assets (units) by burning
        /// index token accordingly. If the given asset is liquid, an
        /// xcm transfer will be dispatched to transfer the given units
        /// into the sovereign account of either:
        /// - the given `recipient` if provided
        /// - the caller's account if `recipient` is `None`
        pub fn remove_asset(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            units: T::Balance,
            recipient: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin.clone())?;
            let caller = ensure_signed(origin)?;
            if units.is_zero() {
                return Ok(().into());
            }
            Self::ensure_not_native_asset(&asset_id)?;

            // calculate current PINT equivalent value
            let value = Self::calculate_pint_equivalent(asset_id, units)?;

            // transfer the caller's fund into the treasury account
            Self::remove_liquid(caller.clone(), asset_id, units, value, recipient.clone())?;

            Self::deposit_event(Event::AssetRemoved(
                asset_id, units, caller, recipient, value,
            ));
            Ok(().into())
        }

        /// Registers a new asset in the index together with its availability
        ///
        /// Only callable by the admin origin and for assets that are not yet
        /// registered.
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn register_asset(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            availability: AssetAvailability,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            Assets::<T>::try_mutate(asset_id, |maybe_available| -> DispatchResult {
                // allow new assets only
                ensure!(
                    maybe_available.replace(availability.clone()).is_none(),
                    Error::<T>::AssetAlreadyExists
                );
                Ok(())
            })?;

            Self::deposit_event(Event::AssetRegistered(asset_id, availability));
            Ok(())
        }

        /// Force the metadata for an asset to some value.
        ///
        /// Origin must be ForceOrigin.
        ///
        /// Any deposit is left alone.
        ///
        /// - `id`: The identifier of the asset to update.
        /// - `name`: The user friendly name of this asset. Limited in length by
        ///   `StringLimit`.
        /// - `symbol`: The exchange symbol for this asset. Limited in length by
        ///   `StringLimit`.
        /// - `decimals`: The number of decimals this asset uses to represent
        ///   one unit.
        ///
        /// Emits `MetadataSet`.
        ///
        /// Weight: `O(N + S)` where N and S are the length of the name and
        /// symbol respectively.
        #[pallet::weight(T::WeightInfo::add_asset())]
        pub fn set_metadata(
            origin: OriginFor<T>,
            #[pallet::compact] id: T::AssetId,
            name: Vec<u8>,
            symbol: Vec<u8>,
            decimals: u8,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let bounded_name: BoundedVec<u8, T::StringLimit> = name
                .clone()
                .try_into()
                .map_err(|_| <Error<T>>::BadMetadata)?;
            let bounded_symbol: BoundedVec<u8, T::StringLimit> = symbol
                .clone()
                .try_into()
                .map_err(|_| <Error<T>>::BadMetadata)?;

            <Metadata<T>>::try_mutate_exists(id, |metadata| {
                *metadata = Some(AssetMetadata {
                    name: bounded_name,
                    symbol: bounded_symbol,
                    decimals,
                });

                Self::deposit_event(Event::MetadataSet(id, name, symbol, decimals));
                Ok(())
            })
        }

        /// Initiate a transfer from the user's sovereign account into the
        /// index.
        ///
        /// This will withdraw the given amount from the user's sovereign
        /// account and mints PINT proportionally using the latest
        /// available price pairs
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn deposit(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            units: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let caller = ensure_signed(origin)?;
            if units.is_zero() {
                return Ok(().into());
            }
            // native asset can't be deposited here
            Self::ensure_not_native_asset(&asset_id)?;

            // only liquid assets can be deposited
            Self::ensure_liquid_asset(&asset_id)?;

            let pint_amount = Self::calculate_pint_equivalent(asset_id, units)?;

            // transfer from the caller's sovereign account into the treasury's account
            T::Currency::transfer(asset_id, &caller, &Self::treasury_account(), units)?;

            // mint index token in caller's account
            Self::do_mint_index_token(&caller, pint_amount);

            Self::deposit_event(Event::Deposited(asset_id, units, caller, pint_amount));
            Ok(().into())
        }

        /// Starts the withdraw process for the given amount of PINT to redeem
        /// for a distribution of underlying assets.
        ///
        /// All withdrawals undergo an unlocking period after which the assets
        /// can be withdrawn. A withdrawal fee will be deducted from the
        /// PINT being redeemed by the LP depending on how long the
        /// assets remained in the index. The remaining PINT will be
        /// burned to match the new NAV after this withdrawal.
        ///
        /// The distribution of the underlying assets will be equivalent to the
        /// ratio of the liquid assets in the index.
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn withdraw(origin: OriginFor<T>, amount: T::Balance) -> DispatchResultWithPostInfo {
            let caller = ensure_signed(origin)?;
            ensure!(
                amount >= T::MinimumRedemption::get(),
                Error::<T>::MinimumRedemption
            );

            // update the locks of prior deposits
            Self::do_update_index_token_locks(&caller);

            let free_balance = T::IndexToken::free_balance(&caller);
            T::IndexToken::ensure_can_withdraw(
                &caller,
                amount,
                WithdrawReasons::TRANSFER,
                free_balance.saturating_sub(amount),
            )?;

            let fee = amount
                .fee(T::BaseWithdrawalFee::get())
                .ok_or(Error::<T>::AssetUnitsOverflow)?;
            let redeem = amount
                .checked_sub(&fee)
                .ok_or(Error::<T>::InsufficientDeposit)?
                .into();

            // NOTE: the ratio of a liquid asset `a` is determined by `sum(nav_asset) /
            // nav_a`
            let mut liquid_assets_vol = T::Balance::zero();
            let mut asset_prices = Vec::new();
            for asset in Assets::<T>::iter()
                .filter(|(_, availability)| availability.is_liquid())
                .map(|(k, _)| k)
            {
                let price = T::PriceFeed::get_price(asset)?;
                let vol = Self::calculate_volume(Self::index_total_asset_balance(asset), &price)?;
                liquid_assets_vol = liquid_assets_vol
                    .checked_add(&vol)
                    .ok_or(Error::<T>::NAVOverflow)?;
                asset_prices.push((price, vol));
            }

            // keep track of the pint units that are actually redeemed, to account for
            // rounding
            let mut redeemed_pint = 0;
            for (price, vol) in &mut asset_prices {
                let ratio = Ratio::checked_from_rational((*vol).into(), liquid_assets_vol.into())
                    .ok_or(Error::<T>::NAVOverflow)?;
                // overwrite the value with the units the user gets for that asset
                *vol = ratio
                    .checked_mul_int(redeem)
                    .and_then(|pint_units| {
                        redeemed_pint += pint_units;
                        price.reciprocal_volume(pint_units)
                    })
                    .ok_or(Error::<T>::AssetVolumeOverflow)
                    .and_then(|units| {
                        units.try_into().map_err(|_| Error::<T>::AssetUnitsOverflow)
                    })?;
            }
            // update the index balance by burning all of the redeemed tokens and the fee
            let effectively_withdrawn = fee
                + redeemed_pint
                    .try_into()
                    .map_err(|_| Error::<T>::AssetUnitsOverflow)?;
            let burned = T::IndexToken::burn(effectively_withdrawn);

            T::IndexToken::settle(
                &caller,
                burned,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::KeepAlive,
            )
            .map_err(|_| ())
            .expect("ensured can withdraw; qed");

            // issue new tokens to compensate the fee and put it into the treasury
            let fee = T::IndexToken::issue(fee);
            T::IndexToken::resolve_creating(&T::TreasuryPalletId::get().into_account(), fee);

            let mut assets = Vec::with_capacity(asset_prices.len());
            // start bonding and locking
            for (price, units) in asset_prices {
                let asset = price.quote;
                // try to start the unbonding process
                let state = if T::RemoteAssetManager::unbond(asset, units).is_ok() {
                    // the XCM call was dispatched successfully, however, this is
                    //  *NOT* synonymous with a successful completion of the unbonding process.
                    //  instead, this state implies that XCM is now being processed on a different
                    // parachain
                    RedemptionState::Unbonding
                } else {
                    // the manager encountered an error before being able to send the XCM call,
                    //  nothing was dispatched to another parachain
                    RedemptionState::Initiated
                };

                // transfer the funds from the index to the user's but reserve it
                T::Currency::transfer(asset, &Self::treasury_account(), &caller, units)?;
                T::Currency::reserve(asset, &caller, units)?;

                assets.push(AssetWithdrawal {
                    asset,
                    state,
                    units,
                });
            }

            // lock the assets for the withdrawal period starting at current block
            PendingWithdrawals::<T>::mutate(&caller, |maybe_redemption| {
                let redemption = maybe_redemption.get_or_insert_with(|| Vec::with_capacity(1));
                redemption.push(PendingRedemption {
                    initiated: frame_system::Pallet::<T>::block_number(),
                    assets,
                })
            });
            Self::deposit_event(Event::WithdrawalInitiated(caller, effectively_withdrawn));
            Ok(().into())
        }

        /// Completes the unbonding process on other parachains and
        /// transfers the redeemed assets into the sovereign account of the
        /// owner.
        ///
        /// Only pending withdrawals that have completed their lockup period
        /// will be withdrawn.
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn complete_withdraw(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let caller = ensure_signed(origin)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            let period = T::WithdrawalPeriod::get();

            PendingWithdrawals::<T>::try_mutate_exists(
                &caller,
                |maybe_pending| -> DispatchResult {
                    let pending = maybe_pending
                        .take()
                        .ok_or(<Error<T>>::NoPendingWithdrawals)?;

                    // try to redeem each redemption, but only close it if all assets could be
                    // redeemed
                    let still_pending: Vec<_> = pending
                        .into_iter()
                        .filter_map(|mut redemption| {
                            // only try to close if the lockup period is over
                            if redemption.initiated + period > current_block {
                                // whether all assets reached state `Transferred`
                                let mut all_withdrawn = true;
                                for asset in &mut redemption.assets {
                                    match asset.state {
                                        RedemptionState::Initiated => {
                                            // unbonding processes failed
                                            // TODO retry or handle this separately?
                                            all_withdrawn = false;
                                        }
                                        RedemptionState::Unbonding => {
                                            // redemption period over and funds are unbonded;
                                            // move to free balance
                                            asset.units = T::Currency::unreserve(
                                                asset.asset,
                                                &caller,
                                                asset.units,
                                            );

                                            if asset.units.is_zero() {
                                                // assets are now transferred completely into the
                                                // user's sovereign account
                                                asset.state = RedemptionState::Transferred;
                                            }
                                        }
                                        RedemptionState::Transferred => {}
                                    }
                                }

                                if all_withdrawn {
                                    // all redemptions completed, remove from storage
                                    Self::deposit_event(Event::WithdrawalCompleted(
                                        caller.clone(),
                                        redemption.assets,
                                    ));
                                    None
                                } else {
                                    Some(redemption)
                                }
                            } else {
                                Some(redemption)
                            }
                        })
                        .collect();

                    if !still_pending.is_empty() {
                        *maybe_pending = Some(still_pending);
                    }
                    Ok(())
                },
            )?;
            Ok(().into())
        }

        /// Updates the index token locks of the caller.
        ///
        /// This removes expired locks and updates the caller's index token
        /// balance accordingly.
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn unlock(origin: OriginFor<T>) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::do_update_index_token_locks(&caller);
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// The account of the treausry that keeps track of all the assets
        /// contributed to the index
        pub fn treasury_account() -> AccountIdFor<T> {
            T::TreasuryPalletId::get().into_account()
        }

        /// The amount of index tokens held by the given user
        pub fn index_token_balance(account: &T::AccountId) -> T::Balance {
            T::IndexToken::total_balance(account)
        }

        /// The amount of index tokens
        pub fn index_token_issuance() -> T::Balance {
            T::IndexToken::total_issuance()
        }

        // The free balance of the given account for the given asset.
        pub fn free_asset_balance(asset: T::AssetId, account: &T::AccountId) -> T::Balance {
            T::Currency::free_balance(asset, account)
        }

        // The combined balance of the given account fo the given asset.
        pub fn total_asset_balance(asset: T::AssetId, account: &T::AccountId) -> T::Balance {
            T::Currency::total_balance(asset, account)
        }

        // The combined balance of the treasury account fo the given asset.
        pub fn index_total_asset_balance(asset: T::AssetId) -> T::Balance {
            T::Currency::total_balance(asset, &Self::treasury_account())
        }

        /// Calculates the total NAV of the Index token: `sum(NAV_asset) / total
        /// pint`
        pub fn total_nav() -> Result<T::Balance, DispatchError> {
            Self::calculate_nav(Assets::<T>::iter().map(|(k, _)| k))
        }

        /// Calculates the NAV of all liquid assets the Index token:
        /// `sum(NAV_liquid) / total pint`
        pub fn liquid_nav() -> Result<T::Balance, DispatchError> {
            Self::calculate_nav(
                Assets::<T>::iter()
                    .filter(|(_, holding)| holding.is_liquid())
                    .map(|(k, _)| k),
            )
        }

        /// Calculates the NAV of all SAFT the Index token: `sum(NAV_saft) /
        /// total pint`
        pub fn saft_nav() -> Result<T::Balance, DispatchError> {
            Self::calculate_nav(
                Assets::<T>::iter()
                    .filter(|(_, holding)| holding.is_saft())
                    .map(|(k, _)| k),
            )
        }

        /// Calculates the total NAV of all holdings
        fn calculate_nav(
            iter: impl Iterator<Item = T::AssetId>,
        ) -> Result<T::Balance, DispatchError> {
            let total_issuance = T::IndexToken::total_issuance();
            if total_issuance.is_zero() {
                return Ok(T::Balance::zero());
            }
            let mut nav = T::Balance::zero();
            for asset in iter {
                nav = nav
                    .checked_add(&Self::calculate_pint_equivalent(
                        asset,
                        Self::index_total_asset_balance(asset),
                    )?)
                    .ok_or(Error::<T>::NAVOverflow)?;
            }
            Ok(nav
                .checked_div(&total_issuance)
                .ok_or(Error::<T>::NAVOverflow)?)
        }

        fn calculate_volume(
            units: T::Balance,
            price: &AssetPricePair<T::AssetId>,
        ) -> Result<T::Balance, DispatchError> {
            let units: u128 = units.into();
            Ok(price
                .volume(units)
                .ok_or(Error::<T>::AssetVolumeOverflow)
                .and_then(|units| units.try_into().map_err(|_| Error::<T>::AssetUnitsOverflow))?)
        }

        /// Calculates the amount of PINT token the given units of the asset are
        /// worth
        fn calculate_pint_equivalent(
            asset: T::AssetId,
            units: T::Balance,
        ) -> Result<T::Balance, DispatchError> {
            Self::calculate_volume(units, &T::PriceFeed::get_price(asset)?)
        }

        /// Calculates the NAV of a single asset
        pub fn asset_nav(asset: T::AssetId) -> Result<T::Balance, DispatchError> {
            ensure!(
                Assets::<T>::contains_key(&asset),
                Error::<T>::UnsupportedAsset
            );
            Self::calculate_pint_equivalent(asset, Self::index_total_asset_balance(asset))
        }

        /// Ensures the given asset id is a liquid asset
        fn ensure_liquid_asset(asset_id: &T::AssetId) -> DispatchResult {
            Assets::<T>::get(asset_id)
                .filter(|availability| matches!(availability, AssetAvailability::Liquid(_)))
                .ok_or(Error::<T>::UnsupportedAsset)?;
            Ok(())
        }

        /// Ensures the given asset is not the native asset
        fn ensure_not_native_asset(asset_id: &T::AssetId) -> DispatchResult {
            ensure!(
                *asset_id != T::SelfAssetId::get(),
                Error::<T>::NativeAssetDisallowed
            );
            Ok(())
        }

        /// Mints the given amount of index token into the user's account and
        /// updates the lock accordingly
        fn do_mint_index_token(user: &T::AccountId, amount: T::Balance) {
            // increase the total issuance
            let issued = T::IndexToken::issue(amount);
            // add minted PINT to user's free balance
            T::IndexToken::resolve_creating(user, issued);

            Self::do_add_index_token_lock(user, amount);
        }

        /// Locks up the given amount of index token according to the
        /// `LockupPeriod` and updates the existing locks
        fn do_add_index_token_lock(user: &T::AccountId, amount: T::Balance) {
            let current_block = frame_system::Pallet::<T>::block_number();
            let mut locks = IndexTokenLocks::<T>::get(user);
            locks.push(IndexTokenLock {
                locked: amount,
                end_block: current_block + T::LockupPeriod::get(),
            });
            Self::do_insert_index_token_locks(user, locks);
        }

        /// inserts the given locks and filters expired locks.
        fn do_insert_index_token_locks(
            user: &T::AccountId,
            locks: Vec<IndexTokenLock<T::BlockNumber, T::Balance>>,
        ) {
            let current_block = frame_system::Pallet::<T>::block_number();
            let mut locked = T::Balance::zero();

            let locks = locks
                .into_iter()
                .filter(|lock| {
                    if current_block >= lock.end_block {
                        // lock period is over
                        false
                    } else {
                        // track locked amount
                        locked = locked.saturating_add(lock.locked);
                        true
                    }
                })
                .collect::<Vec<_>>();

            if locks.is_empty() {
                // remove the lock entirely
                T::IndexToken::remove_lock(T::IndexTokenLockIdentifier::get(), user);
                IndexTokenLocks::<T>::remove(user);
                LockedIndexToken::<T>::remove(user);
            } else {
                // set the lock, if it already exists, this will update it
                T::IndexToken::set_lock(
                    T::IndexTokenLockIdentifier::get(),
                    user,
                    locked,
                    WithdrawReasons::all(),
                );

                IndexTokenLocks::<T>::insert(user, locks);
                LockedIndexToken::<T>::insert(user, locked);
            }
        }

        /// Updates the index token locks for the given user.
        fn do_update_index_token_locks(user: &T::AccountId) {
            let locks = IndexTokenLocks::<T>::get(user);
            if !locks.is_empty() {
                Self::do_insert_index_token_locks(user, IndexTokenLocks::<T>::get(user))
            }
        }
    }

    impl<T: Config> AssetRecorder<T::AccountId, T::AssetId, T::Balance> for Pallet<T> {
        /// Creates an entry in the assets map and contributes the given amount
        /// of asset to the treasury.
        fn add_liquid(
            caller: &T::AccountId,
            asset_id: T::AssetId,
            units: T::Balance,
            nav: T::Balance,
        ) -> DispatchResult {
            if units.is_zero() {
                return Ok(());
            }
            // native asset can't be added
            Self::ensure_not_native_asset(&asset_id)?;
            // transfer the given units of asset from the caller into the treasury account
            T::Currency::transfer(asset_id, caller, &Self::treasury_account(), units)?;
            // mint PINT into caller's balance increasing the total issuance
            T::IndexToken::deposit_creating(caller, nav);
            Ok(())
        }

        fn add_saft(
            caller: &T::AccountId,
            asset_id: T::AssetId,
            units: T::Balance,
            nav: T::Balance,
        ) -> DispatchResult {
            if units.is_zero() {
                return Ok(());
            }
            // native asset can't be added as saft
            Self::ensure_not_native_asset(&asset_id)?;

            // ensure that the given asset id is either SAFT or not yet registered
            Assets::<T>::try_mutate(asset_id, |maybe_available| -> DispatchResult {
                if let Some(exits) = maybe_available.replace(AssetAvailability::Saft) {
                    ensure!(exits.is_saft(), Error::<T>::ExpectedSAFT);
                }
                Ok(())
            })?;

            // mint SAFT into the treasury's account
            T::Currency::deposit(asset_id, &Self::treasury_account(), units)?;
            // mint PINT into caller's balance increasing the total issuance
            T::IndexToken::deposit_creating(caller, nav);

            Ok(())
        }

        fn insert_asset_availability(
            asset_id: T::AssetId,
            availability: AssetAvailability,
        ) -> Option<AssetAvailability> {
            Assets::<T>::mutate(asset_id, |maybe_available| {
                maybe_available.replace(availability)
            })
        }

        fn remove_liquid(
            who: T::AccountId,
            asset_id: T::AssetId,
            units: T::Balance,
            nav: T::Balance,
            recipient: Option<T::AccountId>,
        ) -> DispatchResult {
            if units.is_zero() {
                return Ok(());
            }
            ensure!(Self::is_liquid_asset(&asset_id), Error::<T>::ExpectedLiquid);
            ensure!(
                T::IndexToken::can_slash(&who, nav),
                Error::<T>::InsufficientDeposit
            );

            let recipient = recipient.unwrap_or_else(|| who.clone());

            // Execute the transfer which will take of updating the balance
            T::RemoteAssetManager::transfer_asset(recipient, asset_id, units)?;

            // burn index token accordingly, no index token changes in the meantime
            T::IndexToken::slash(&who, nav);

            Ok(())
        }

        fn remove_saft(
            who: T::AccountId,
            asset_id: T::AssetId,
            units: T::Balance,
            nav: T::Balance,
        ) -> DispatchResult {
            if units.is_zero() {
                return Ok(());
            }
            // native asset can't be processed here
            Self::ensure_not_native_asset(&asset_id)?;

            ensure!(!Self::is_liquid_asset(&asset_id), Error::<T>::ExpectedSAFT);
            ensure!(
                T::IndexToken::can_slash(&who, nav),
                Error::<T>::InsufficientDeposit
            );

            // burn SAFT by withdrawing from the index
            T::Currency::withdraw(asset_id, &Self::treasury_account(), units)?;
            // burn index token accordingly, no index token changes in the meantime
            T::IndexToken::slash(&who, nav);

            Ok(())
        }
    }

    impl<T: Config> MultiAssetRegistry<T::AssetId> for Pallet<T> {
        fn native_asset_location(asset: &T::AssetId) -> Option<MultiLocation> {
            Assets::<T>::get(asset).and_then(|availability| {
                if let AssetAvailability::Liquid(location) = availability {
                    Some(location)
                } else {
                    None
                }
            })
        }

        fn is_liquid_asset(asset: &T::AssetId) -> bool {
            Assets::<T>::get(asset)
                .map(|availability| availability.is_liquid())
                .unwrap_or_default()
        }
    }

    /// Trait for the asset-index pallet extrinsic weights.
    pub trait WeightInfo {
        fn add_asset() -> Weight;
        fn set_metadata() -> Weight;
    }

    /// For backwards compatibility and tests
    impl WeightInfo for () {
        fn add_asset() -> Weight {
            Default::default()
        }

        fn set_metadata() -> Weight {
            Default::default()
        }
    }
}
