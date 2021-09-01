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
	use sp_core::U256;
	use xcm::v0::MultiLocation;

	#[cfg(feature = "runtime-benchmarks")]
	use pallet_price_feed::PriceFeedBenchmarks;
	use pallet_price_feed::{AssetPricePair, Price, PriceFeed};
	use primitives::{
		fee::{BaseFee, FeeRate},
		traits::{AssetRecorder, MultiAssetRegistry, NavProvider, RemoteAssetManager, SaftRegistry},
		AssetAvailability, AssetProportion, AssetProportions, Ratio,
	};

	use crate::types::{AssetMetadata, AssetRedemption, AssetWithdrawal, IndexTokenLock, PendingRedemption};
	use primitives::traits::MaybeTryFrom;

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
		type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize + MaybeTryFrom<u8>;

		/// The native asset id
		#[pallet::constant]
		type SelfAssetId: Get<Self::AssetId>;

		/// Currency type for deposit/withdraw assets to/from the user's
		/// sovereign account
		type Currency: MultiReservableCurrency<Self::AccountId, CurrencyId = Self::AssetId, Balance = Self::Balance>;

		/// The types that provides the necessary asset price pairs
		type PriceFeed: PriceFeed<Self::AssetId>;

		#[cfg(feature = "runtime-benchmarks")]
		/// The type that provides benchmark features of pallet_price_feed
		type PriceFeedBenchmarks: PriceFeedBenchmarks<Self::AccountId, Self::AssetId>;

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

	/// Metadata of an asset ( for reversed usage now ).
	#[pallet::storage]
	#[pallet::getter(fn asset_metadata)]
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
		pub liquid_assets: Vec<(T::AssetId, polkadot_parachain::primitives::Id)>,
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
			use xcm::v0::Junction;
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
		/// Completed a single asset withdrawal of the PendingRedemption
		/// \[Account, AssetId, AssetUnits\]
		Withdrawn(AccountIdFor<T>, T::AssetId, T::Balance),
		/// Completed an entire pending asset withdrawal
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
		/// Thrown if adding saft when total nav is empty
		EmptyNav,
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
		/// This gets thrown if the total supply of index tokens is 0 so no NAV can be calculated to
		/// determine the Asset/Index Token rate.
		InsufficientIndexTokens,
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
		///
		/// The Governance committee decides the tokens that comprise the index,
		/// as well as the allocation of each and their value.
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
		#[pallet::weight(T::WeightInfo::remove_asset())]
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

			// the amount of index token the given units of the liquid assets are worth
			let index_tokens = Self::index_token_equivalent(asset_id, units)?;

			// transfer the caller's fund into the treasury account
			Self::remove_liquid(caller.clone(), asset_id, units, index_tokens, recipient.clone())?;

			Self::deposit_event(Event::AssetRemoved(asset_id, units, caller, recipient, index_tokens));
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
		#[pallet::weight(T::WeightInfo::set_metadata())]
		pub fn set_metadata(
			origin: OriginFor<T>,
			id: T::AssetId,
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
		pub fn deposit(origin: OriginFor<T>, asset_id: T::AssetId, units: T::Balance) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			if units.is_zero() {
				return Ok(());
			}
			// native asset can't be deposited here
			Self::ensure_not_native_asset(&asset_id)?;
			// only liquid assets can be deposited
			Self::ensure_liquid_asset(&asset_id)?;

			// can't calculate an exchange rate if the total supply of index tokens is 0
			if Self::index_token_issuance().is_zero() {
				return Err(Error::<T>::InsufficientIndexTokens.into());
			}

			// the amount of index token the given units of the liquid assets are worth
			let index_tokens = Self::index_token_equivalent(asset_id, units)?;

			if index_tokens.is_zero() {
				return Err(Error::<T>::InsufficientDeposit.into());
			}

			// transfer from the caller's sovereign account into the treasury's account
			T::Currency::transfer(asset_id, &caller, &Self::treasury_account(), units)?;

			// mint index token in caller's account
			Self::do_mint_index_token(&caller, index_tokens);

			// tell the remote asset manager that assets are available to bond
			T::RemoteAssetManager::deposit(asset_id, units);

			Self::deposit_event(Event::Deposited(asset_id, units, caller, index_tokens));
			Ok(())
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
		#[pallet::weight(T::WeightInfo::withdraw())]
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

			// calculate the payout for each asset based on the redeem amount
			let AssetRedemption { asset_amounts, redeemed_index_tokens } = Self::liquid_asset_redemptions(redeem)?;

			// update the index balance by burning all of the redeemed tokens and the fee
			// SAFETY: this is guaranteed to be lower than `amount`
			let effectively_withdrawn = fee + redeemed_index_tokens;

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

			let mut assets = Vec::with_capacity(asset_amounts.len());

			// start the redemption process for each withdrawal
			for (asset, units) in asset_amounts {
				// announce the unbonding routine
				T::RemoteAssetManager::announce_withdrawal(asset, units);
				// reserve the funds in the treasury's account until the redemption period is
				// over after which they can be transferred to the user account
				// NOTE: this should always succeed due to the way the asset distribution is
				// calculated
				T::Currency::reserve(asset, &Self::treasury_account(), units)?;
				assets.push(AssetWithdrawal { asset, units, reserved: units, withdrawn: false });
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
		/// started by the `withdraw` extrinsic.
		///
		/// This checks every pending withdrawal within `PendingWithdrawal` and
		/// tries to close it. Completing a withdrawal will succeed if
		/// following conditions are met:
		///   - the `LockupPeriod` has passed since the withdrawal was initiated
		///   - the treasury can cover the asset transfer to the caller's account, from which the
		///     caller then can initiate an `Xcm::Withdraw` to remove the assets from the PINT
		///     parachain entirely, if xcm transfers are supported.
		///
		/// *NOTE*: All individual withdrawals that resulted from "Withdraw"
		/// will be completed separately, however, the entire record of pending
		/// withdrawals will not be fully closed until the last withdrawal is
		/// completed. This means that a single `AssetWithdrawal` will be closed
		/// as soon as the aforementioned conditions are met, regardless of
		/// whether the other `AssetWithdrawal`s in the same `PendingWithdrawal` set
		/// can also be closed successfully.
		#[pallet::weight(T::WeightInfo::complete_withdraw())]
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
						if redemption.end_block >= current_block &&
							Self::do_complete_redemption(&caller, &mut redemption.assets)
						{
							// all individual redemptions withdrawn, can remove them from storage
							Self::deposit_event(Event::WithdrawalCompleted(caller.clone(), redemption.assets));
							return None;
						}
						Some(redemption)
					})
					.collect();

				if !still_pending.is_empty() {
					// still have redemptions pending
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
		#[pallet::weight(T::WeightInfo::unlock())]
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

		/// Iterates over all liquid assets
		pub fn liquid_assets() -> impl Iterator<Item = T::AssetId> {
			Assets::<T>::iter().filter(|(_, availability)| availability.is_liquid()).map(|(id, _)| id)
		}

		/// Iterates over all SAFT assets
		pub fn saft_assets() -> impl Iterator<Item = T::AssetId> {
			Assets::<T>::iter().filter(|(_, holding)| holding.is_saft()).map(|(k, _)| k)
		}

		fn calculate_nav_proportion(asset: T::AssetId, nav: Ratio) -> Result<Ratio, DispatchError> {
			// the proportion is `value(asset) / value(index)` and since `nav = value(index)/supply`, this is
			// `value(asset)/supply / nav`
			let asset_value = Self::net_asset_value(asset)?;
			let share = Ratio::checked_from_rational(asset_value.into(), Self::index_token_issuance().into())
				.ok_or(ArithmeticError::Overflow)?;
			share.checked_div(&nav).ok_or_else(|| ArithmeticError::Overflow.into())
		}

		/// Returns the relative price pair NAV/Asset to calculate the asset equivalent value:
		/// `num(asset) = num(index_tokens) * NAV/Asset`.
		///
		/// *NOTE*: assumes the `quote` is a liquid asset.
		fn liquid_nav_price_pair(nav: Price, quote: T::AssetId) -> Result<AssetPricePair<T::AssetId>, DispatchError> {
			let quote_price = T::PriceFeed::get_price(quote)?;
			let price = nav.checked_div(&quote_price).ok_or(ArithmeticError::Overflow)?;
			Ok(AssetPricePair::new(T::SelfAssetId::get(), quote, price))
		}

		/// Calculates the pure asset redemption for the given amount of the
		/// index token to be redeemed for all the liquid tokens in the index
		///
		/// *NOTE*:
		///   - This does not account for fees
		///   - This is a noop for `redeem == 0`
		pub fn liquid_asset_redemptions(
			redeem: u128,
		) -> Result<AssetRedemption<T::AssetId, T::Balance>, DispatchError> {
			if redeem.is_zero() {
				return Ok(Default::default());
			}
			// track the index tokens that effectively are redeemed
			let mut redeemed_index_tokens = 0u128;

			// calculate the proportions of all liquid assets in the index' liquid value
			let AssetProportions { nav, proportions } = Self::liquid_asset_proportions()?;

			// the total NAV is sum(liquid_nav + saft_nav) and represents the real value of a 1unit of index
			// token
			let nav = Self::saft_nav()?.checked_add(&nav).ok_or(ArithmeticError::Overflow)?;

			// calculate the redeemed amounts
			let asset_amounts = proportions
				.into_iter()
				.map(|proportion| -> Result<_, DispatchError> {
					let index_tokens = proportion.of(redeem).ok_or(ArithmeticError::Overflow)?;
					redeemed_index_tokens =
						redeemed_index_tokens.checked_add(index_tokens).ok_or(ArithmeticError::Overflow)?;

					// the amount of index tokens to redeem in proportion of the liquid asset
					let index_tokens: T::Balance = index_tokens.try_into().map_err(|_| ArithmeticError::Overflow)?;

					// determine the asset amount based on the relative price pair NAV/Asset
					let nav_asset_price = Self::liquid_nav_price_pair(nav, proportion.asset)?;
					let asset_units: T::Balance = nav_asset_price
						.volume(index_tokens.into())
						.and_then(|n| TryInto::<T::Balance>::try_into(n).ok())
						.ok_or(ArithmeticError::Overflow)?;

					Ok((proportion.asset, asset_units))
				})
				.collect::<Result<_, _>>()?;

			Ok(AssetRedemption {
				asset_amounts,
				redeemed_index_tokens: redeemed_index_tokens.try_into().map_err(|_| Error::<T>::AssetUnitsOverflow)?,
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
		/// states towards the `Withdrawn` state. In which all assets were transferred to the
		/// caller's holding accounts.
		///
		/// Returns `true` if all entries are completed (the have been
		/// transferred to the caller's account)
		fn do_complete_redemption(
			caller: &T::AccountId,
			assets: &mut Vec<AssetWithdrawal<T::AssetId, T::Balance>>,
		) -> bool {
			// whether all assets reached state `Withdrawn`
			let mut all_withdrawn = true;

			for asset in assets {
				if !asset.withdrawn {
					// `unreserve` the previously reserved assets from the treasury.
					asset.reserved = T::Currency::unreserve(asset.asset, &Self::treasury_account(), asset.reserved);
					if T::Currency::transfer(asset.asset, &Self::treasury_account(), caller, asset.units).is_ok() {
						Self::deposit_event(Event::Withdrawn(caller.clone(), asset.asset, asset.units));
						asset.withdrawn = true;
						continue;
					}
				}
				all_withdrawn = false
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
			// mint asset into the treasury account
			T::Currency::deposit(asset_id, &Self::treasury_account(), units)?;
			// mint PINT into caller's balance increasing the total issuance
			T::IndexToken::deposit_creating(caller, nav);
			Ok(())
		}

		fn add_saft(
			caller: &T::AccountId,
			asset_id: T::AssetId,
			units: T::Balance,
			saft_nav: T::Balance,
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

			// determine the index token equivalent value of the given saft_nav, or how many index token the
			// given `saft_nav` is worth we get this via `saft_nav / NAV` or `NAV^-1 * saft_nav`
			let index_token: T::Balance = Self::nav()?
				.reciprocal()
				.and_then(|n| n.checked_mul_int(saft_nav.into()).and_then(|n| TryInto::<T::Balance>::try_into(n).ok()))
				.ok_or(ArithmeticError::Overflow)?;

			// mint the given units of the SAFT asset into the treasury's account
			T::Currency::deposit(asset_id, &Self::treasury_account(), units)?;

			// mint PINT into caller's balance increasing the total issuance
			T::IndexToken::deposit_creating(caller, index_token);
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
				T::PriceFeed::get_price(asset)?
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
			let price = T::PriceFeed::get_price(asset)?;
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
			Assets::<T>::iter().try_fold(U256::zero(), |value, (asset, availability)| -> Result<_, DispatchError> {
				if availability.is_liquid() {
					value.checked_add(U256::from(Self::net_liquid_value(asset)?.into()))
				} else {
					value.checked_add(U256::from(Self::net_saft_value(asset).into()))
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

		fn asset_proportion(asset: T::AssetId) -> Result<Ratio, DispatchError> {
			// the proportion is `value(asset) / value(index)` and since `nav = value(index)/supply`, this is
			// `value(asset)/supply / nav`
			let nav = Self::nav()?;
			Self::calculate_nav_proportion(asset, nav)
		}

		fn liquid_asset_proportion(asset: T::AssetId) -> Result<Ratio, DispatchError> {
			let nav = Self::liquid_nav()?;
			Self::calculate_nav_proportion(asset, nav)
		}

		fn saft_asset_proportion(asset: T::AssetId) -> Result<Ratio, DispatchError> {
			let nav = Self::saft_nav()?;
			Self::calculate_nav_proportion(asset, nav)
		}

		fn asset_proportions() -> Result<AssetProportions<T::AssetId>, DispatchError> {
			let nav = Self::nav()?;
			let proportions = Assets::<T>::iter()
				.map(|(id, _)| id)
				.map(|id| Self::calculate_nav_proportion(id, nav).map(|ratio| AssetProportion::new(id, ratio)))
				.collect::<Result<_, _>>()?;
			Ok(AssetProportions { nav, proportions })
		}

		fn liquid_asset_proportions() -> Result<AssetProportions<T::AssetId>, DispatchError> {
			let nav = Self::liquid_nav()?;
			let proportions = Self::liquid_assets()
				.map(|id| Self::calculate_nav_proportion(id, nav).map(|ratio| AssetProportion::new(id, ratio)))
				.collect::<Result<_, _>>()?;
			Ok(AssetProportions { nav, proportions })
		}

		fn saft_asset_proportions() -> Result<AssetProportions<T::AssetId>, DispatchError> {
			let nav = Self::saft_nav()?;
			let proportions = Self::saft_assets()
				.map(|id| Self::calculate_nav_proportion(id, nav).map(|ratio| AssetProportion::new(id, ratio)))
				.collect::<Result<_, _>>()?;
			Ok(AssetProportions { nav, proportions })
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
		fn complete_withdraw() -> Weight;
		fn register_asset() -> Weight;
		fn remove_asset() -> Weight;
		fn deposit() -> Weight;
		fn unlock() -> Weight;
		fn withdraw() -> Weight;
		fn set_metadata() -> Weight;
	}

	/// For backwards compatibility and tests
	impl WeightInfo for () {
		fn add_asset() -> Weight {
			Default::default()
		}

		fn complete_withdraw() -> Weight {
			Default::default()
		}

		fn register_asset() -> Weight {
			Default::default()
		}

		fn remove_asset() -> Weight {
			Default::default()
		}

		fn deposit() -> Weight {
			Default::default()
		}

		fn set_metadata() -> Weight {
			Default::default()
		}

		fn unlock() -> Weight {
			Default::default()
		}

		fn withdraw() -> Weight {
			Default::default()
		}
	}
}
