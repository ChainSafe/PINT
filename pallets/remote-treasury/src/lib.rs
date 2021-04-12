// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # Remote Treasury Pallet
//!
//! The Remote Treasury pallet provides functionality for handling DOT on the relay chain via XCMP.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
// this is requires as the #[pallet::event] proc macro generates code that violates this lint
#[allow(clippy::unused_unit)]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::{
            traits::{AccountIdConversion, AtLeast32BitUnsigned, Convert},
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

        // /// The Call type required for other chains.
        type RemoteCall: Parameter;

        /// Convert an `AccountId` to `AccountId32` for cross chain messages
        type AccountId32Convert: Convert<AccountIdFor<Self>, [u8; 32]>;

        /// Used to convert accounts to locations
        type AccountIdConverter: LocationConversion<AccountIdFor<Self>>;

        /// ModuleId must be an unique 8 character string.
        /// It is used to generate the account ID which holds the balance of the treasury.
        #[pallet::constant]
        type ModuleId: Get<ModuleId>;

        // /// Descriptor of where the treasury asset exist: `(Parent, AccountId32)`
        // #[pallet::constant]
        // type Location: Get<MultiLocation>;

        /// The network id of relay chain. Typically `NetworkId::Polkadot`.
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
        /// Admin successfully transferred some funds from the DOT treasury on the relay chain into the recipient's account on the relay chain.
        /// parameters. \[recipient, amount\]
        TransferredDOT(AccountIdFor<T>, T::Balance),
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
        /// Execution of a cross-chain failed
        FailedXcmExecution,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::extra_constants]
    impl<T: Config> Pallet<T> {
        /// Returns the accountID for the treasury balance
        /// Transferring balance to this account funds the treasury
        pub fn account_id() -> T::AccountId {
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
        /// Transfer balance from the treasury asset's location to another destination.
        /// Only callable by the AdminOrigin.
        #[transactional]
        #[pallet::weight(10_000)] // TODO: Set weights
        pub fn transfer_dot(
            origin: OriginFor<T>,
            amount: T::Balance,
            recipient: AccountIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;

            let asset = MultiAsset::ConcreteFungible {
                id: Junction::Parent.into(),
                amount: amount.into(),
            };

            // the recipient's account on the relay chain
            let dest = (
                Junction::Parent,
                Junction::AccountId32 {
                    network: T::RelayChainNetworkId::get(),
                    id: T::AccountId32Convert::convert(recipient.clone()),
                },
            )
                .into();

            let xcm_origin = T::AccountIdConverter::try_into_location(Self::account_id())
                .map_err(|_| Error::<T>::BadLocation)?;

            Self::do_transfer_on_relay_chain(xcm_origin, asset, dest)?;

            Self::deposit_event(Event::TransferredDOT(recipient, amount));

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Transfer the `MultiAsset` via the `XcmHandler` without depositing event.
        ///
        /// Executes a cross-chain message to withdraw DOT from the treasury's holding on
        /// the relay chain and deposits it into destination's account on the relay chain.
        fn do_transfer_on_relay_chain(
            origin: MultiLocation,
            asset: MultiAsset,
            dest: MultiLocation,
        ) -> DispatchResultWithPostInfo {
            let (dest, recipient) =
                Self::split_multi_location(dest).ok_or_else(|| Error::<T>::InvalidDestination)?;

            let xcm = Xcm::WithdrawAsset {
                assets: vec![asset],
                effects: vec![Order::InitiateReserveWithdraw {
                    assets: vec![MultiAsset::All],
                    reserve: dest,
                    effects: vec![Order::DepositAsset {
                        assets: vec![MultiAsset::All],
                        dest: recipient,
                    }],
                }],
            };

            T::XcmHandler::execute_xcm(origin, xcm).map_err(|_| Error::<T>::FailedXcmExecution)?;

            Ok(().into())
        }

        /// Splits the `location` into the chain location part and the recipient location.
        fn split_multi_location(location: MultiLocation) -> Option<(MultiLocation, MultiLocation)> {
            let chain_location = match (location.first(), location.at(1)) {
                (Some(Junction::Parent), Some(Junction::Parachain { id })) => {
                    Some((Junction::Parent, Junction::Parachain { id: *id }).into())
                }
                (Some(Junction::Parent), _) => Some(Junction::Parent.into()),
                (Some(Junction::Parachain { id }), _) => {
                    Some(Junction::Parachain { id: *id }.into())
                }
                _ => None,
            }?;

            let (path, location) = location.split_last();

            // make sure the path until `location` only consists of chains
            path.iter()
                .all(|junction| {
                    matches!(junction, Junction::Parent | Junction::Parachain { id: _ })
                })
                .then(|| location.map(|location| (chain_location, location.into())))?
        }
    }
}
