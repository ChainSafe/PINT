// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! # XCM Support Pallet
//!
//! Provides support for xcm operations within PINT

#![cfg_attr(not(feature = "std"), no_std)]

pub use crate::traits::{Error, XcmAssetHandler};
use frame_support::{
    sp_runtime::{sp_std::marker::PhantomData, traits::Convert},
    sp_std::prelude::*,
    traits::Get,
};
use xcm::opaque::v0::{MultiLocation, Outcome};
use xcm::v0::{ExecuteXcm, Junction, MultiAsset, NetworkId, Order, Xcm};
use xcm_executor::traits::Convert as XcmConvert;

pub mod config;
pub mod traits;
pub use config::Config;

/// The type responsible for executing cross chain transfers
pub struct XcmAssetExecutor<Config>(PhantomData<Config>);

impl<Config: config::Config> XcmAssetExecutor<Config> {
    /// Executes a cross chain message to transfer the `MultiAsset` to its correct location.
    fn do_transfer_multiasset(
        who: Config::AccountId,
        asset: MultiAsset,
        dest: MultiLocation,
    ) -> frame_support::sp_std::result::Result<Outcome, Error> {
        let id = Config::AccountId32Convert::convert(who);

        let origin: MultiLocation = Junction::AccountId32 {
            id,
            network: NetworkId::Any,
        }
        .into();

        let (dest, recipient) = Self::split_multi_location(&dest);

        let dest = dest.ok_or(Error::InvalidDestination)?;
        let self_location = Config::SelfLocation::get();
        frame_support::ensure!(dest != self_location, Error::NoCrossChainTransfer);

        let recipient = recipient.ok_or(Error::InvalidDestination)?;

        // the native location of the asset type
        let reserve = Self::asset_reserve(&asset).ok_or(Error::InvalidDestination)?;

        let xcm = if reserve == self_location {
            Self::transfer_reserve_asset_locally(asset, dest, recipient)
        } else if reserve == dest {
            Self::transfer_to_reserve(asset, dest, recipient)
        } else {
            Self::transfer_to_non_reserve(asset, reserve, dest, recipient)
        };

        let outcome = Config::XcmExecutor::execute_xcm(origin, xcm, Config::WeightLimit::get());

        Ok(outcome)
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
    ) -> Xcm<Config::Call> {
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
    ) -> Xcm<Config::Call> {
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
    ) -> Xcm<Config::Call> {
        let mut reanchored_dest = dest.clone();
        if reserve == Junction::Parent.into() {
            if let MultiLocation::X2(Junction::Parent, Junction::Parachain(id)) = dest {
                reanchored_dest = Junction::Parachain(id).into();
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
            (Some(Junction::Parent), Some(Junction::Parachain(id))) => {
                Some((Junction::Parent, Junction::Parachain(*id)).into())
            }
            (Some(Junction::Parent), _) => Some(Junction::Parent.into()),
            (Some(Junction::Parachain(id)), _) => Some(Junction::Parachain(*id).into()),
            _ => None,
        };

        let (path, last_junction) = location.clone().split_last();
        // make sure the path until the final junction consists of chain junction
        let target_location = last_junction
            .into_iter()
            .filter(|_| {
                path.iter()
                    .all(|junction| matches!(junction, Junction::Parent | Junction::Parachain(_)))
            })
            .map(Into::into)
            .next();

        (chain_location, target_location)
    }
}

impl<Config: config::Config> XcmAssetHandler<Config::AccountId, Config::Amount, Config::AssetId>
    for XcmAssetExecutor<Config>
{
    fn execute_xcm_transfer(
        who: Config::AccountId,
        asset_id: Config::AssetId,
        amount: Config::Amount,
    ) -> frame_support::sp_std::result::Result<Outcome, Error> {
        let dest: MultiLocation = Config::AssetIdConvert::convert(asset_id)
            .map_err(|_| Error::NotCrossChainTransferableAsset)?;

        let asset = MultiAsset::ConcreteFungible {
            id: dest.clone(),
            amount: Config::AmountU128Convert::convert(amount),
        };

        Self::do_transfer_multiasset(who, asset, dest)
    }
}
