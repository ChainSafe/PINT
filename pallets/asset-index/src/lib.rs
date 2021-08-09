// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # AssetIndex Pallet
//!
//! Tracks all the assets in the PINT index, composed of multiple assets
//!
//! The value of the assets is determined depending on their class.
//! The value of liquid assets is calculated by multiplying their current unit price by the amount
//! held in the index. Whereas the value of an asset secured by SAFTs is measured by the total value
//! of all SAFTs.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod types;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit, clippy::large_enum_variant, clippy::type_complexity)]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResultWithPostInfo,
		pallet_prelude::*,
		require_transactional,
		sp_runtime::{
			traits::{AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, CheckedDiv, CheckedSub, Saturating, Zero},
			ArithmeticError, FixedPointNumber,
		},
		sp_std::{convert::TryInto, prelude::*, result::Result},
		traits::{Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency, WithdrawReasons},
		transactional, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{MultiCurrency, MultiReservableCurrency};
	use polkadot_parachain::primitives::Id as ParaId;
	use xcm::v0::{Junction, MultiLocation};

	use pallet_price_feed::{AssetPricePair, Price, PriceFeed};

	use crate::types::{
		AssetMetadata, AssetRedemption, AssetVolume, AssetWithdrawal, AssetsDistribution, AssetsVolume, IndexTokenLock,
		PendingRedemption, RedemptionState,
	};
	use primitives::{
		fee::{BaseFee, FeeRate},
		traits::{AssetRecorder, MultiAssetRegistry, NavProvider, RemoteAssetManager, SaftRegistry, UnbondingOutcome},
		AssetAvailability, Ratio,
	};
	use sp_core::U256;

	type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

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
		type Currency: MultiReservableCurrency<Self::AccountId, CurrencyId = Self::AssetId, Balance = Self::Balance>;

		/// The types that provides the necessary asset price pairs
		type PriceFeed: PriceFeed<Self::AssetId>;

		/// The type registry that stores all NAV for non liquid assets
		type SaftRegistry: SaftRegistry<Self::AssetId, Self::Balance>;

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

	/// (AssetId) -> AssetAvailability
	#[pallet::storage]
	#[pallet::getter(fn assets)]
	pub type Assets<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, AssetAvailability, OptionQuery>;

	///  (AccountId) -> Vec<PendingRedemption>
	#[pallet::storage]
	#[pallet::getter(fn pending_withrawals)]
	pub type PendingWithdrawals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Vec<PendingRedemption<T::AssetId, T::Balance, BlockNumberFor<T>>>,
		OptionQuery,
	>;

	/// Tracks the locks of the minted index token that are locked up until
	/// their `LockupPeriod` is over  (AccountId) -> Vec<IndexTokenLockInfo>
	#[pallet::storage]
	#[pallet::getter(fn index_token_locks)]
	pub type IndexTokenLocks<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Vec<IndexTokenLock<T::BlockNumber, T::Balance>>, ValueQuery>;

	/// Tracks the amount of the currently locked index token per user.
	/// This is equal to the sum(IndexTokenLocks[AccountId])
	///  (AccountId) -> Balance
	#[pallet::storage]
	#[pallet::getter(fn locked_index_tokens)]
	pub type LockedIndexToken<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance, ValueQuery>;

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
		/// All the liquid assets together with their parachain id known at
		/// genesis
		pub liquid_assets: Vec<(T::AssetId, ParaId)>,
		/// ALl safts to register at genesis
		pub saft_assets: Vec<T::AssetId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { liquid_assets: Default::default(), saft_assets: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (asset, id) in self.liquid_assets.iter().cloned() {
				let availability = AssetAvailability::Liquid((Junction::Parent, Junction::Parachain(id.into())).into());
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
		AssetRemoved(T::AssetId, T::Balance, AccountIdFor<T>, Option<AccountIdFor<T>>, T::Balance),
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
		WithdrawalCompleted(AccountIdFor<T>, Vec<AssetWithdrawal<T::AssetId, T::Balance>>),
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

			// register asset if not yet known
			if is_new_asset {
				Assets::<T>::insert(asset_id, availability.clone());
				Self::deposit_event(Event::AssetRegistered(asset_id, availability));
			}

			Self::deposit_event(Event::AssetAdded(asset_id, units, caller, amount));
			Ok(().into())
		}

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
		#[pallet::weight(10_000)] // TODO: Set weights
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

			Self::deposit_event(Event::AssetRemoved(asset_id, units, caller, recipient, value));
			Ok(().into())
		}

		/// Registers a new asset in the index together with its availability
		///
		/// Only callable by the admin origin and for assets that are not yet
		/// registered.
		#[pallet::weight(T::WeightInfo::register_asset())]
		pub fn register_asset(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			availability: AssetAvailability,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			Assets::<T>::try_mutate(asset_id, |maybe_available| -> DispatchResult {
				// allow new assets only
				ensure!(maybe_available.replace(availability.clone()).is_none(), Error::<T>::AssetAlreadyExists);
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
		/// - `name`: The user friendly name of this asset. Limited in length by `StringLimit`.
		/// - `symbol`: The exchange symbol for this asset. Limited in length by `StringLimit`.
		/// - `decimals`: The number of decimals this asset uses to represent one unit.
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

			let bounded_name: BoundedVec<u8, T::StringLimit> =
				name.clone().try_into().map_err(|_| <Error<T>>::BadMetadata)?;
			let bounded_symbol: BoundedVec<u8, T::StringLimit> =
				symbol.clone().try_into().map_err(|_| <Error<T>>::BadMetadata)?;

			Metadata::<T>::try_mutate_exists(id, |metadata| {
				*metadata = Some(AssetMetadata { name: bounded_name, symbol: bounded_symbol, decimals });

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
		#[pallet::weight(T::WeightInfo::deposit())]
		pub fn deposit(origin: OriginFor<T>, asset_id: T::AssetId, units: T::Balance) -> DispatchResultWithPostInfo {
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
		#[transactional]
		pub fn withdraw(origin: OriginFor<T>, amount: T::Balance) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;
			ensure!(amount >= T::MinimumRedemption::get(), Error::<T>::MinimumRedemption);

			// update the locks of prior deposits
			Self::do_update_index_token_locks(&caller);

			let free_balance = T::IndexToken::free_balance(&caller);
			T::IndexToken::ensure_can_withdraw(
				&caller,
				amount,
				WithdrawReasons::all(),
				free_balance.saturating_sub(amount),
			)?;

			// amount = fee + redeem
			let fee = amount.fee(T::BaseWithdrawalFee::get()).ok_or(Error::<T>::AssetUnitsOverflow)?;
			let redeem = amount.checked_sub(&fee).ok_or(Error::<T>::InsufficientDeposit)?.into();

			// calculate the distribution of all liquid assets
			let distribution = Self::get_liquid_asset_distribution()?;

			// calculate the payout for each asset based on the redeem amount
			let asset_redemption = Self::get_asset_redemption(distribution, redeem)?;

			// update the index balance by burning all of the redeemed tokens and the fee
			// SAFETY: this is guaranteed to be lower than `amount`
			let effectively_withdrawn = fee + asset_redemption.redeemed_pint;

			// withdraw from caller balance
			T::IndexToken::withdraw(
				&caller,
				effectively_withdrawn,
				WithdrawReasons::all(),
				ExistenceRequirement::AllowDeath,
			)?;

			// issue new tokens to compensate the fee and put it into the treasury
			let fee = T::IndexToken::issue(fee);
			T::IndexToken::resolve_creating(&Self::treasury_account(), fee);

			let mut assets = Vec::with_capacity(asset_redemption.asset_amounts.len());

			// start the redemption process for each withdrawal
			for (asset, units) in asset_redemption.asset_amounts {
				// start the unbonding routine
				let state = match T::RemoteAssetManager::unbond(asset, units) {
					UnbondingOutcome::NotSupported | UnbondingOutcome::SufficientReserve => {
						// nothing to unbond, the funds are assumed to be available after the
						// redemption period is over
						RedemptionState::Unbonding
					}
					UnbondingOutcome::Outcome(outcome) => {
						// the outcome of the dispatched xcm call
						if outcome.ensure_complete().is_ok() {
							// the XCM call was dispatched successfully, however, this is  *NOT*
							// synonymous with a successful completion of the unbonding process.
							// instead, this state implies that XCM is now being processed on a
							// different parachain
							RedemptionState::Unbonding
						} else {
							// failed to send the unbond xcm
							RedemptionState::Initiated
						}
					}
				};

				// reserve the funds in the treasury's account until the redemption period is
				// over after which they can be transferred to the user account
				// NOTE: this should always succeed due to the way the distribution is
				// calculated
				T::Currency::reserve(asset, &Self::treasury_account(), units)?;

				// T::Currency::transfer(asset, &Self::treasury_account(), &caller, units)?;
				assets.push(AssetWithdrawal { asset, state, units });
			}

			// after this block an asset withdrawal is allowed to advance to the transfer
			// state
			let end_block = frame_system::Pallet::<T>::block_number().saturating_add(T::WithdrawalPeriod::get());
			// lock the assets for the withdrawal period starting at current block
			PendingWithdrawals::<T>::mutate(&caller, |maybe_redemption| {
				let redemption = maybe_redemption.get_or_insert_with(|| Vec::with_capacity(1));
				redemption.push(PendingRedemption { end_block, assets })
			});

			Self::deposit_event(Event::WithdrawalInitiated(caller, effectively_withdrawn));
			Ok(().into())
		}

		/// Attempts to complete all currently pending redemption processes
		/// started by `withdraw`.
		///
		/// This checks every pending withdrawal within `PendingWithdrawal` and
		/// tries to close it. Completing a withdrawal will succeed if
		/// following conditions are met:
		///   - the `LockupPeriod` has passed since the withdrawal was initiated
		///   - the unbonding process on other parachains was successful
		///   - the treasury can cover the asset transfer to the caller's account, from which the
		///     caller then can initiate an `Xcm::Withdraw` to remove the assets from the PINT
		///     parachain entirely.
		///
		/// *NOTE*: All individual withdrawals that resulted from "Withdraw"
		/// will be completed separately, however, the entire record of pending
		/// withdrawals will not be fully closed until the last withdrawal is
		/// completed. This means that a single `AssetWithdrawal` will be closed
		/// as soon as the aforementioned conditions are met, regardless of
		/// whether the other `AssetWithdrawal` of the same `PendingWithdrawal`
		/// entry can also be closed successfully.
		#[pallet::weight(10_000)] // TODO: Set weights
		pub fn complete_withdraw(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;

			let current_block = frame_system::Pallet::<T>::block_number();

			PendingWithdrawals::<T>::try_mutate_exists(&caller, |maybe_pending| -> DispatchResult {
				let pending = maybe_pending.take().ok_or(<Error<T>>::NoPendingWithdrawals)?;

				// try to redeem each redemption, but only close it if all assets could be
				// redeemed
				let still_pending: Vec<_> = pending
					.into_iter()
					.filter_map(|mut redemption| {
						// only try to close if the lockup period is over
						if redemption.end_block >= current_block {
							if Self::do_complete_redemption(&caller, &mut redemption.assets) {
								// all redemptions completed, remove from storage
								Self::deposit_event(Event::WithdrawalCompleted(caller.clone(), redemption.assets));
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
			})?;
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

		/// The free balance of the given account for the given asset.
		pub fn free_asset_balance(asset: T::AssetId, account: &T::AccountId) -> T::Balance {
			T::Currency::free_balance(asset, account)
		}

		/// The combined balance of the given account for the given asset.
		pub fn total_asset_balance(asset: T::AssetId, account: &T::AccountId) -> T::Balance {
			T::Currency::total_balance(asset, account)
		}

		/// The combined balance of the treasury account for the given asset.
		pub fn index_total_asset_balance(asset: T::AssetId) -> T::Balance {
			T::Currency::total_balance(asset, &Self::treasury_account())
		}

		/// The free balance of the treasury account for the given asset.
		pub fn index_free_asset_balance(asset: T::AssetId) -> T::Balance {
			T::Currency::free_balance(asset, &Self::treasury_account())
		}

		/// Calculates the total NAV of the Index token: `sum(NAV_asset) / total
		/// pint`
		pub fn total_nav() -> Result<T::Balance, DispatchError> {
			Self::calculate_nav(Assets::<T>::iter().map(|(k, _)| k))
		}

		/// Calculates the NAV of all liquid assets the Index token:
		/// `sum(NAV_liquid) / total pint`
		pub fn liquid_nav() -> Result<T::Balance, DispatchError> {
			Self::calculate_nav(Self::liquid_assets())
		}

		/// Calculates the NAV of all SAFT the Index token: `sum(NAV_saft) /
		/// total pint`
		pub fn saft_nav() -> Result<T::Balance, DispatchError> {
			Self::calculate_nav(Self::saft_assets())
		}

		/// Calculates the total NAV of all holdings
		fn calculate_nav(iter: impl Iterator<Item = T::AssetId>) -> Result<T::Balance, DispatchError> {
			let total_issuance = T::IndexToken::total_issuance();
			if total_issuance.is_zero() {
				return Ok(T::Balance::zero());
			}
			let nav = iter.into_iter().try_fold(T::Balance::zero(), |nav, asset| -> Result<_, DispatchError> {
				nav.checked_add(&Self::calculate_pint_equivalent(asset, Self::index_free_asset_balance(asset))?)
					.ok_or_else(|| Error::<T>::NAVOverflow.into())
			})?;

			Ok(nav.checked_div(&total_issuance).ok_or(Error::<T>::NAVOverflow)?)
		}

		/// Calculates the volume of the given units with the provided price
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
		fn calculate_pint_equivalent(asset: T::AssetId, units: T::Balance) -> Result<T::Balance, DispatchError> {
			Self::calculate_volume(units, &T::PriceFeed::get_price(asset)?)
		}

		/// Calculates the NAV of a single asset
		pub fn asset_nav(asset: T::AssetId) -> Result<T::Balance, DispatchError> {
			ensure!(Assets::<T>::contains_key(&asset), Error::<T>::UnsupportedAsset);
			Self::calculate_pint_equivalent(asset, Self::index_free_asset_balance(asset))
		}

		/// Iterates over all liquid assets
		pub fn liquid_assets() -> impl Iterator<Item = T::AssetId> {
			Assets::<T>::iter().filter(|(_, availability)| availability.is_liquid()).map(|(id, _)| id)
		}

		/// Iterates over all SAFT assets
		pub fn saft_assets() -> impl Iterator<Item = T::AssetId> {
			Assets::<T>::iter().filter(|(_, holding)| holding.is_saft()).map(|(k, _)| k)
		}

		/// Returns a Vec of the current prices of all active liquid assets
		/// together with the derived volume of these assets residing in the
		/// index
		pub fn get_liquid_asset_volumes() -> Result<AssetsVolume<T::AssetId, T::Balance>, DispatchError> {
			// accumulated pint volume that the assets represent
			let mut total_volume = T::Balance::zero();

			let volumes = Self::liquid_assets()
				.map(|asset| -> Result<_, DispatchError> {
					let volume = Self::get_liquid_asset_volume(asset)?;
					total_volume = total_volume.checked_add(&volume.pint_volume).ok_or(Error::<T>::NAVOverflow)?;
					Ok(volume)
				})
				.collect::<Result<_, _>>()?;

			Ok(AssetsVolume { volumes, total_volume })
		}

		/// Returns the current price and the volume of the given asset
		///
		/// This current volume of the index is equal to the free balance of the
		/// treasury account.
		pub fn get_liquid_asset_volume(
			asset: T::AssetId,
		) -> Result<AssetVolume<T::AssetId, T::Balance>, DispatchError> {
			let price = T::PriceFeed::get_price(asset)?;
			let pint_volume = Self::calculate_volume(Self::index_free_asset_balance(asset), &price)?;
			Ok(AssetVolume::new(price, pint_volume))
		}

		/// Returns the current distribution of all liquid assets
		///
		/// For each plant the equivalent volume is determined and its share in
		/// the total amount of pint, all these assets represent
		pub fn get_liquid_asset_distribution() -> Result<AssetsDistribution<T::AssetId, T::Balance>, DispatchError> {
			let AssetsVolume { volumes, total_volume } = Self::get_liquid_asset_volumes()?;

			// calculate the share of each asset in the total_pint
			let asset_shares = volumes
				.into_iter()
				.map(|asset| -> Result<_, DispatchError> {
					let ratio = Ratio::checked_from_rational(asset.pint_volume.into(), total_volume.into())
						.ok_or(Error::<T>::NAVOverflow)?;
					Ok((asset, ratio))
				})
				.collect::<Result<_, _>>()?;

			Ok(AssetsDistribution { total_pint: total_volume, asset_shares })
		}

		/// Calculates the pure asset redemption for the given amount of the
		/// index token to be redeemed based on the given distribution
		///
		/// *NOTE*:
		///   - This does not account for fees
		///   - This is a noop for `redeem == 0`
		pub fn get_asset_redemption(
			distribution: AssetsDistribution<T::AssetId, T::Balance>,
			redeem: u128,
		) -> Result<AssetRedemption<T::AssetId, T::Balance>, DispatchError> {
			if redeem.is_zero() {
				return Ok(Default::default());
			}
			// track the pint that effectively gets redeemed
			let mut redeemed_pint = 0u128;

			// calculate the the distribution of the assets
			let asset_amounts = distribution
				.asset_shares
				.into_iter()
				.map(|(asset, ratio)| -> Result<_, DispatchError> {
					let amount: T::Balance = ratio
						.checked_mul_int(redeem)
						.and_then(|pint_units| {
							redeemed_pint += pint_units;
							asset.price.reciprocal_volume(pint_units)
						})
						.ok_or(Error::<T>::AssetVolumeOverflow)
						.and_then(|units| units.try_into().map_err(|_| Error::<T>::AssetUnitsOverflow))?;

					Ok((asset.price.quote, amount))
				})
				.collect::<Result<_, _>>()?;

			Ok(AssetRedemption {
				asset_amounts,
				redeemed_pint: redeemed_pint.try_into().map_err(|_| Error::<T>::AssetUnitsOverflow)?,
			})
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
			ensure!(!Self::is_native_asset(*asset_id), Error::<T>::NativeAssetDisallowed);
			Ok(())
		}

		/// Whether the asset is in fact the native asset
		fn is_native_asset(asset_id: T::AssetId) -> bool {
			asset_id == T::SelfAssetId::get()
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
			locks.push(IndexTokenLock { locked: amount, end_block: current_block + T::LockupPeriod::get() });
			Self::do_insert_index_token_locks(user, locks);
		}

		/// inserts the given locks and filters expired locks.
		fn do_insert_index_token_locks(user: &T::AccountId, locks: Vec<IndexTokenLock<T::BlockNumber, T::Balance>>) {
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
				T::IndexToken::set_lock(T::IndexTokenLockIdentifier::get(), user, locked, WithdrawReasons::all());

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

		/// Tries to complete every single `AssetWithdrawal` by advancing their
		/// states towards the `Withdrawn` state.
		///
		/// Returns `true` if all entries are completed (the have been
		/// transferred to the caller's account)
		#[require_transactional]
		fn do_complete_redemption(
			caller: &T::AccountId,
			assets: &mut Vec<AssetWithdrawal<T::AssetId, T::Balance>>,
		) -> bool {
			// whether all assets reached state `Withdrawn`
			let mut all_withdrawn = true;

			for asset in assets {
				match asset.state {
					RedemptionState::Initiated => {
						// unbonding processes failed
						// TODO retry or handle this separately?
						all_withdrawn = false;
					}
					RedemptionState::Unbonding => {
						// funds are unbonded and can be transferred to the caller's account

						// `unreserve` only moves up to `units` from the reserved balance to free.
						// if this returns `>0` then the treasury's reserved balance is empty, in
						// which case we simply proceed with attempting to transfer
						T::Currency::unreserve(asset.asset, &Self::treasury_account(), asset.units);

						if T::Currency::transfer(asset.asset, &Self::treasury_account(), caller, asset.units).is_ok() {
							asset.state = RedemptionState::Withdrawn;
						} else {
							asset.state = RedemptionState::Transferring;
							all_withdrawn = false;
						}
					}
					RedemptionState::Transferring => {
						// try to transfer again
						if T::Currency::transfer(asset.asset, &Self::treasury_account(), caller, asset.units).is_ok() {
							asset.state = RedemptionState::Withdrawn;
						} else {
							all_withdrawn = false;
						}
					}
					RedemptionState::Withdrawn => {}
				}
			}
			all_withdrawn
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

		fn add_saft(caller: &T::AccountId, asset_id: T::AssetId, units: T::Balance, nav: T::Balance) -> DispatchResult {
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
			Assets::<T>::mutate(asset_id, |maybe_available| maybe_available.replace(availability))
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
			ensure!(T::IndexToken::can_slash(&who, nav), Error::<T>::InsufficientDeposit);

			let recipient = recipient.unwrap_or_else(|| who.clone());

			// Execute the transfer which will take of updating the balance
			T::RemoteAssetManager::transfer_asset(recipient, asset_id, units)?;

			// burn index token accordingly, no index token changes in the meantime
			T::IndexToken::slash(&who, nav);

			Ok(())
		}

		fn remove_saft(who: T::AccountId, asset_id: T::AssetId, units: T::Balance, nav: T::Balance) -> DispatchResult {
			if units.is_zero() {
				return Ok(());
			}
			// native asset can't be processed here
			Self::ensure_not_native_asset(&asset_id)?;

			ensure!(!Self::is_liquid_asset(&asset_id), Error::<T>::ExpectedSAFT);
			ensure!(T::IndexToken::can_slash(&who, nav), Error::<T>::InsufficientDeposit);

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
			Assets::<T>::get(asset).map(|availability| availability.is_liquid()).unwrap_or_default()
		}
	}

	impl<T: Config> SaftRegistry<T::AssetId, T::Balance> for Pallet<T> {
		fn net_saft_value(asset: T::AssetId) -> T::Balance {
			T::SaftRegistry::net_saft_value(asset)
		}
	}

	impl<T: Config> NavProvider<T::AssetId, T::Balance> for Pallet<T> {
		fn index_token_equivalent(asset: T::AssetId, units: T::Balance) -> Result<T::Balance, DispatchError> {
			// Price_asset/NAV*units
			if Self::is_native_asset(asset) {
				return Ok(units);
			}
			let price = Self::relative_asset_price(asset)?;
			price
				.reciprocal_volume(units.into())
				.and_then(|n| TryInto::<T::Balance>::try_into(n).ok())
				.ok_or_else(|| ArithmeticError::Overflow.into())
		}

		fn asset_equivalent(index_tokens: T::Balance, asset: T::AssetId) -> Result<T::Balance, DispatchError> {
			if Self::is_native_asset(asset) {
				return Ok(index_tokens);
			}
			// NAV/Price_asset*units
			let price = Self::relative_asset_price(asset)?;
			price
				.volume(index_tokens.into())
				.and_then(|n| TryInto::<T::Balance>::try_into(n).ok())
				.ok_or_else(|| ArithmeticError::Overflow.into())
		}

		fn relative_asset_price(asset: T::AssetId) -> Result<AssetPricePair<T::AssetId>, DispatchError> {
			let base_price = Self::nav()?;
			if Self::is_native_asset(asset) {
				return Ok(AssetPricePair::new(asset, asset, base_price));
			}

			let quote_price = if Self::is_liquid_asset(&asset) {
				T::PriceFeed::get_price(asset)?.price
			} else {
				let val = Self::net_saft_value(asset);
				Price::checked_from_rational(val.into(), Self::asset_balance(asset).into())
					.ok_or(ArithmeticError::Overflow)?
			};
			let price = base_price.checked_div(&quote_price).ok_or(ArithmeticError::Overflow)?;
			Ok(AssetPricePair::new(T::SelfAssetId::get(), asset, price))
		}

		fn calculate_net_asset_value(asset: T::AssetId, units: T::Balance) -> Result<T::Balance, DispatchError> {
			// the asset net worth depends on whether the asset is liquid or SAFT
			if Self::is_liquid_asset(&asset) {
				Self::calculate_net_liquid_value(asset, units)
			} else {
				Self::calculate_net_saft_value(asset, units)
			}
		}

		fn calculate_net_liquid_value(asset: T::AssetId, units: T::Balance) -> Result<T::Balance, DispatchError> {
			// TODO change price API
			let price = T::PriceFeed::get_price(asset)?.price;
			price
				.checked_mul_int(units.into())
				.and_then(|n| TryInto::<T::Balance>::try_into(n).ok())
				.ok_or_else(|| ArithmeticError::Overflow.into())
		}

		fn calculate_net_saft_value(asset: T::AssetId, units: T::Balance) -> Result<T::Balance, DispatchError> {
			let val = Self::net_saft_value(asset);
			let price = Price::checked_from_rational(val.into(), Self::asset_balance(asset).into())
				.ok_or(ArithmeticError::Overflow)?;
			price
				.checked_mul_int(units.into())
				.and_then(|n| TryInto::<T::Balance>::try_into(n).ok())
				.ok_or_else(|| ArithmeticError::Overflow.into())
		}

		fn total_net_liquid_value() -> Result<U256, DispatchError> {
			Self::liquid_assets().into_iter().try_fold(U256::zero(), |worth, asset| -> Result<_, DispatchError> {
				worth
					.checked_add(U256::from(Self::net_liquid_value(asset)?.into()))
					.ok_or_else(|| Error::<T>::NAVOverflow.into())
			})
		}

		fn total_net_saft_value() -> Result<U256, DispatchError> {
			Self::saft_assets().into_iter().try_fold(U256::zero(), |worth, asset| -> Result<_, DispatchError> {
				worth
					.checked_add(U256::from(Self::net_saft_value(asset).into()))
					.ok_or_else(|| Error::<T>::NAVOverflow.into())
			})
		}

		fn total_net_asset_value() -> Result<U256, DispatchError> {
			Assets::<T>::iter().try_fold(U256::zero(), |worth, (asset, availability)| -> Result<_, DispatchError> {
				if availability.is_liquid() {
					worth.checked_add(U256::from(Self::net_liquid_value(asset)?.into()))
				} else {
					worth.checked_add(U256::from(Self::net_saft_value(asset).into()))
				}
				.ok_or_else(|| Error::<T>::NAVOverflow.into())
			})
		}

		fn net_asset_value(asset: T::AssetId) -> Result<T::Balance, DispatchError> {
			if Self::is_liquid_asset(&asset) {
				Self::calculate_net_liquid_value(asset, Self::asset_balance(asset))
			} else {
				Ok(Self::net_saft_value(asset))
			}
		}

		fn nav() -> Result<Ratio, DispatchError> {
			let total_issuance = T::IndexToken::total_issuance();
			if total_issuance.is_zero() {
				return Ok(Ratio::zero());
			}
			Assets::<T>::iter().try_fold(Ratio::zero(), |nav, (asset, availability)| -> Result<_, DispatchError> {
				let value =
					if availability.is_liquid() { Self::net_liquid_value(asset)? } else { Self::net_saft_value(asset) };
				let proportion = Ratio::checked_from_rational(value.into(), total_issuance.into())
					.ok_or(ArithmeticError::Overflow)?;
				Ok(nav.checked_add(&proportion).ok_or(ArithmeticError::Overflow)?)
			})
		}

		fn liquid_nav() -> Result<Ratio, DispatchError> {
			let total_issuance = T::IndexToken::total_issuance();
			if total_issuance.is_zero() {
				return Ok(Ratio::zero());
			}
			Self::liquid_assets().try_fold(Ratio::zero(), |nav, asset| -> Result<_, DispatchError> {
				let value = Self::net_liquid_value(asset)?;
				let proportion = Ratio::checked_from_rational(value.into(), total_issuance.into())
					.ok_or(ArithmeticError::Overflow)?;
				Ok(nav.checked_add(&proportion).ok_or(ArithmeticError::Overflow)?)
			})
		}

		fn saft_nav() -> Result<Ratio, DispatchError> {
			let total_issuance = T::IndexToken::total_issuance();
			if total_issuance.is_zero() {
				return Ok(Ratio::zero());
			}
			Self::saft_assets().try_fold(Ratio::zero(), |nav, asset| -> Result<_, DispatchError> {
				let value = Self::net_saft_value(asset);
				let proportion = Ratio::checked_from_rational(value.into(), total_issuance.into())
					.ok_or(ArithmeticError::Overflow)?;
				Ok(nav.checked_add(&proportion).ok_or(ArithmeticError::Overflow)?)
			})
		}

		fn index_token_issuance() -> T::Balance {
			T::IndexToken::total_issuance()
		}

		fn asset_balance(asset: T::AssetId) -> T::Balance {
			// the free balance constitutes the funds held by the index
			T::Currency::free_balance(asset, &Self::treasury_account())
		}
	}

	/// Trait for the asset-index pallet extrinsic weights.
	pub trait WeightInfo {
		fn add_asset() -> Weight;
		fn register_asset() -> Weight;
		fn deposit() -> Weight;
		fn set_metadata() -> Weight;
	}

	/// For backwards compatibility and tests
	impl WeightInfo for () {
		fn add_asset() -> Weight {
			Default::default()
		}

		fn register_asset() -> Weight {
			Default::default()
		}

		fn deposit() -> Weight {
			Default::default()
		}

		fn set_metadata() -> Weight {
			Default::default()
		}
	}
}
