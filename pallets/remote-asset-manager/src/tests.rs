// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate::mock::*;

use frame_support::{assert_noop, assert_ok};
use pallet_asset_index::types::AssetAvailability;
use xcm::v0::{
    Junction::{self, *},
    MultiAsset::*,
    MultiLocation::*,
    NetworkId,
};
use xcm_simulator::TestExt;
use frame_support::traits::fungibles::Inspect;

fn print_events<T: frame_system::Config>(context: &str) {
    println!("------ {:?} events ------", context);
    frame_system::Pallet::<T>::events().iter().for_each(|r| {
        println!("{:?}", r.event);
    });
}

fn register_relay() {
    assert_ok!(pallet_asset_index::Pallet::<para::Runtime>::register_asset(
        para::Origin::signed(ADMIN_ACCOUNT.clone()),
        RELAY_CHAIN_ASSET,
        AssetAvailability::Liquid(Parent.into())
    ));
}

#[test]
fn para_account_funded_on_relay() {
    MockNet::reset();

    Relay::execute_with(|| {
        let para_balance_on_relay =
            pallet_balances::Pallet::<relay::Runtime>::free_balance(&para_relay_account());
        assert_eq!(para_balance_on_relay, INITIAL_BALANCE);
    });
}

#[test]
fn can_deposit_from_relay() {
    MockNet::reset();
    let relay_deposit_amount = 1000;
    Para::execute_with(|| register_relay());

    Relay::execute_with(|| {
        // transfer from relay to parachain
        assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
            relay::Origin::signed(ALICE),
            X1(Parachain(PARA_ID)),
            X1(Junction::AccountId32 {
                network: NetworkId::Any,
                id: ALICE.into(),
            }),
            vec![ConcreteFungible {
                id: Null,
                amount: relay_deposit_amount
            }],
            relay_deposit_amount as u64,
        ));
    });
    Para::execute_with(|| {
        // ensure deposit arrived
        assert_eq!(
            orml_tokens::Pallet::<para::Runtime>::total_issuance(
                RELAY_CHAIN_ASSET
            ),
            relay_deposit_amount
        );
        assert_eq!(
            orml_tokens::Pallet::<para::Runtime>::balance(
                RELAY_CHAIN_ASSET, &ALICE
            ),
            relay_deposit_amount
        );
    });
}
#[test]
fn can_transact_register_proxy() {
    MockNet::reset();

    Para::execute_with(|| {
        register_relay()

        // para::RemoteAssetManager::transfer_asset(para_relay_account(), 5,100);
        // let x = para::RemoteAssetManager::send_add_proxy(
        //     para::Origin::signed(ADMIN_ACCOUNT),
        //     RELAY_CHAIN_ASSET,
        //     ParaProxyType(RelayProxyType::Staking as u8),
        //     None,
        // );
        // dbg!(x);
        //
        // print_events::<para::Runtime>("Para");
    });
    //
    // Relay::execute_with(|| {
    //     let para_balance_on_relay =
    //         pallet_balances::Pallet::<relay::Runtime>::free_balance(&para_relay_account());
    //     assert_eq!(para_balance_on_relay, INITIAL_BALANCE);
    // });
}
