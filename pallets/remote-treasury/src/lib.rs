// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Remote Treasury Pallet
//!
//! The Remote Treasury pallet provides functionality for handling DOT on the relay chain via XCMP.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::{
            traits::{AccountIdConversion, AtLeast32BitUnsigned, Convert, Zero},
            ModuleId,
        },
        traits::Get,
        transactional,
    };
    use frame_system::pallet_prelude::*;
    use xcm::v0::{ExecuteXcm, Junction, MultiAsset, MultiLocation, NetworkId, Order, Xcm};
    use xcm_executor::traits::LocationConversion;

    type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin that is allowed to manage the treasury and dispatch cross-chain calls from the
        /// Treasury's account
        type AdminOrigin: EnsureOrigin<Self::Origin>;

        /// The balance type for cross chain transfers
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Into<u128>;

        /// Asset Id that is used to identify different kinds of assets.
        type AssetId: Parameter + Member + Clone;

        /// Convert a `T::AssetId` to its relative `MultiLocation` identifier.
        type AssetIdConvert: Convert<Self::AssetId, Option<MultiLocation>>;

        /// Convert an `AccountId` to `AccountId32` for cross chain messages
        type AccountId32Convert: Convert<AccountIdFor<Self>, [u8; 32]>;

        /// Used to convert accounts to locations
        type AccountIdConverter: LocationConversion<AccountIdFor<Self>>;

        /// ModuleId must be an unique 8 character string.
        /// It is used to generate the account ID which holds the balance of the treasury.
        #[pallet::constant]
        type ModuleId: Get<ModuleId>;

        /// Self chain location.
        #[pallet::constant]
        type SelfLocation: Get<MultiLocation>;

        /// Identifier for the relay chain's asset type
        #[pallet::constant]
        type RelayChainAssetId: Get<Self::AssetId>;

        /// The network id of relay chain. Typically `NetworkId::Polkadot`.
        #[pallet::constant]
        type RelayChainNetworkId: Get<NetworkId>;

        /// Executor for cross chain messages.
        type XcmHandler: ExecuteXcm;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Admin successfully transferred relay chain assets from the treasury's account on the relay chain into the recipient's account on the relay chain.
        /// parameters. \[recipient, amount\]
        TransferredRelayChainAsset(AccountIdFor<T>, AccountIdFor<T>, T::Balance),
        /// Admin successfully transferred some asset units.
        /// parameters. \[sender, asset_id, amount, dest\]
        Transferred(AccountIdFor<T>, T::AssetId, T::Balance, MultiLocation),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Thrown when conversion from accountId to MultiLocation failed
        BadLocation,
        /// Can't transfer to the provided location.
        InvalidDestination,
        /// Thrown when the destination of a requested cross-chain transfer is the location of
        /// the local chain itself
        NoCrossChainTransfer,
        /// Failed to convert the provided currency into a location
        NotCrossChainTransferableAsset,
        /// Execution of a cross-chain failed
        FailedXcmExecution,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::extra_constants]
    impl<T: Config> Pallet<T> {
        /// Returns the accountID for the treasury balance
        /// Transferring balance to this account funds the treasury
        pub fn account_id() -> AccountIdFor<T> {
            T::ModuleId::get().into_account()
        }

        /// Returns the location of the treasury account on the relay chain
        pub fn treasury_location() -> MultiLocation {
            (
                Junction::Parent,
                Junction::AccountId32 {
                    network: T::RelayChainNetworkId::get(),
                    id: T::AccountId32Convert::convert(Self::account_id()),
                },
            )
                .into()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer units of the asset type to the provided destination.
        ///
        /// Dependent on the the asset's native location wrt the destination, the appropriate
        /// cross chain message routine is executed:
        ///
        /// 1) Is the native location of the provided asset the local system, then requested amount
        /// of the native asset is transferred from the treasury account to the destination's
        /// account within this system, then this transfer is mirrored on the
        /// destination's system via a cross chain message.
        ///
        /// 2) If the native location of the provided asset matches the destination's location,
        /// then the requested amount is withdrawn from the treasury locally as well as on
        /// corresponding holding on the destination's system and deposited into the destination's
        /// account within the destination system.
        ///
        /// 3) If the native location of the provided asset neither matches with the local system or
        /// with the provided destination, then the requested amount is withdrawn from the treasury
        /// locally as and send to the asset's native location. There the amount is removed from
        /// the treasury's holding and put into the holding of the destination's account. Finally
        /// another xcm from the asset's native location to the provided destination is issued
        /// and the provided amount is put into the destination's account in the destination's
        /// system.
        ///
        /// Only callable by the AdminOrigin.
        #[transactional]
        #[pallet::weight(1000)]
        pub fn transfer(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            amount: T::Balance,
            dest: MultiLocation,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin.clone())?;
            let who = ensure_signed(origin)?;

            if amount.is_zero() {
                // nothing to transfer
                return Ok(().into());
            }

            let id: MultiLocation = T::AssetIdConvert::convert(asset_id.clone())
                .ok_or(Error::<T>::NotCrossChainTransferableAsset)?;
            let asset = MultiAsset::ConcreteFungible {
                id,
                amount: amount.into(),
            };

            Self::do_transfer_multiasset(Self::account_id(), asset, dest.clone())?;
            Self::deposit_event(Event::Transferred(who, asset_id, amount, dest));
            Ok(().into())
        }

        /// Transfer units of the relay chain asset from the treasury asset's location to another destination.
        /// Only callable by the AdminOrigin.
        #[transactional]
        #[pallet::weight(10)] // TODO: Set weights
        pub fn transfer_relay_chain_asset(
            origin: OriginFor<T>,
            amount: T::Balance,
            recipient: AccountIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin.clone())?;
            let who = ensure_signed(origin)?;

            let mut relay_chain_location: MultiLocation =
                T::AssetIdConvert::convert(T::RelayChainAssetId::get())
                    .ok_or(Error::<T>::NotCrossChainTransferableAsset)?;
            let asset = MultiAsset::ConcreteFungible {
                id: relay_chain_location.clone(),
                amount: amount.into(),
            };

            // the recipient's account on the relay chain
            relay_chain_location
                .push(Junction::AccountId32 {
                    network: T::RelayChainNetworkId::get(),
                    id: T::AccountId32Convert::convert(recipient.clone()),
                })
                .map_err(|_| Error::<T>::NotCrossChainTransferableAsset)?;

            Self::do_transfer_multiasset(Self::account_id(), asset, relay_chain_location)?;
            Self::deposit_event(Event::TransferredRelayChainAsset(who, recipient, amount));
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Executes a cross chain message to transfer the `MultiAsset` to its correct location.
        fn do_transfer_multiasset(
            who: AccountIdFor<T>,
            asset: MultiAsset,
            dest: MultiLocation,
        ) -> DispatchResultWithPostInfo {
            let xcm_origin = T::AccountIdConverter::try_into_location(who)
                .map_err(|_| Error::<T>::BadLocation)?;

            let (dest, recipient) = Self::split_multi_location(&dest);

            let dest = dest.ok_or_else(|| Error::<T>::InvalidDestination)?;
            let self_location = T::SelfLocation::get();
            ensure!(dest != self_location, Error::<T>::NoCrossChainTransfer);

            let recipient = recipient.ok_or_else(|| Error::<T>::InvalidDestination)?;

            // the native location of the asset type
            let reserve =
                Self::asset_reserve(&asset).ok_or_else(|| Error::<T>::InvalidDestination)?;

            let xcm = if reserve == self_location {
                Self::transfer_reserve_asset_locally(asset, dest, recipient)
            } else if reserve == dest {
                Self::transfer_to_reserve(asset, dest, recipient)
            } else {
                Self::transfer_to_non_reserve(asset, reserve, dest, recipient)
            };

            T::XcmHandler::execute_xcm(xcm_origin, xcm)
                .map_err(|_| Error::<T>::FailedXcmExecution)?;

            Ok(().into())
        }

        /// A cross chain message that will
        /// - withdraw the `asset` from the issuer's holding (locally)
        /// - deposit the `asset` into `dest`'s holding (locally)
        /// - send another Xcm to `dest`
        /// - remove `asset` from sender's holding (on `dest`)
        /// - deposit `asset` into `recipient` (on `dest`)
        fn transfer_reserve_asset_locally(
            asset: MultiAsset,
            dest: MultiLocation,
            recipient: MultiLocation,
        ) -> Xcm {
            Xcm::WithdrawAsset {
                assets: vec![asset],
                effects: vec![Order::DepositReserveAsset {
                    assets: vec![MultiAsset::All],
                    dest,
                    effects: vec![Order::DepositAsset {
                        assets: vec![MultiAsset::All],
                        dest: recipient,
                    }],
                }],
            }
        }

        /// A cross chain message that will
        /// - withdraw the `asset` from the issuer's holding (locally)
        /// - send another Xcm to `reserve`
        /// - withdraw `asset` from the holding (on `reserve`)
        /// - deposit `asset` into `recipient` (on `reserve`)
        fn transfer_to_reserve(
            asset: MultiAsset,
            reserve: MultiLocation,
            recipient: MultiLocation,
        ) -> Xcm {
            Xcm::WithdrawAsset {
                assets: vec![asset],
                effects: vec![Order::InitiateReserveWithdraw {
                    assets: vec![MultiAsset::All],
                    reserve,
                    effects: vec![Order::DepositAsset {
                        assets: vec![MultiAsset::All],
                        dest: recipient,
                    }],
                }],
            }
        }

        /// A cross chain message that will
        /// - withdraw the `asset` from the issuer's holding (locally)
        /// - send another Xcm to `reserve`
        /// - withdraw `asset` from the holding (on `reserve`)
        /// - deposit `asset` into `dest` (on `reserve`)
        /// - send another Xcm to `dest`
        /// - deposit `asset` into `recipient` (in `dest`)
        ///
        /// If the `reserve` is the relay chain and `dest` includes the hop via the relay chain
        /// `dest` is reanchored from the relay chain's point of view.
        fn transfer_to_non_reserve(
            asset: MultiAsset,
            reserve: MultiLocation,
            dest: MultiLocation,
            recipient: MultiLocation,
        ) -> Xcm {
            let mut reanchored_dest = dest.clone();
            if reserve == Junction::Parent.into() {
                if let MultiLocation::X2(Junction::Parent, Junction::Parachain { id }) = dest {
                    reanchored_dest = Junction::Parachain { id }.into();
                }
            }

            Xcm::WithdrawAsset {
                assets: vec![asset],
                effects: vec![Order::InitiateReserveWithdraw {
                    assets: vec![MultiAsset::All],
                    reserve,
                    effects: vec![Order::DepositReserveAsset {
                        assets: vec![MultiAsset::All],
                        dest: reanchored_dest,
                        effects: vec![Order::DepositAsset {
                            assets: vec![MultiAsset::All],
                            dest: recipient,
                        }],
                    }],
                }],
            }
        }

        /// Returns the chain location part of the asset.
        fn asset_reserve(asset: &MultiAsset) -> Option<MultiLocation> {
            if let MultiAsset::ConcreteFungible { id, .. } = asset {
                Self::split_multi_location(id).0
            } else {
                None
            }
        }

        /// Splits the `location` into the chain location part and the recipient location.
        fn split_multi_location(
            location: &MultiLocation,
        ) -> (Option<MultiLocation>, Option<MultiLocation>) {
            let chain_location = match (location.first(), location.at(1)) {
                (Some(Junction::Parent), Some(Junction::Parachain { id })) => {
                    Some((Junction::Parent, Junction::Parachain { id: *id }).into())
                }
                (Some(Junction::Parent), _) => Some(Junction::Parent.into()),
                (Some(Junction::Parachain { id }), _) => {
                    Some(Junction::Parachain { id: *id }.into())
                }
                _ => None,
            };

            let (path, last_junction) = location.clone().split_last();
            // make sure the path until the final junction consists of chain junction
            let target_location = last_junction
                .into_iter()
                .filter(|_| {
                    path.iter().all(|junction| {
                        matches!(junction, Junction::Parent | Junction::Parachain { id: _ })
                    })
                })
                .map(Into::into)
                .next();

            (chain_location, target_location)
        }
    }
}
