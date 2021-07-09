// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate::mock::*;

use xcm_simulator::TestExt;

fn print_events<T: frame_system::Config>(context: &str) {
    println!("------ {:?} events ------", context);
    frame_system::Pallet::<T>::events().iter().for_each(|r| {
        println!("{:?}", r.event);
    });
}

fn register_relay() {
    // para::AssetIndex;
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
fn can_transact_register_proxy() {
    MockNet::reset();

    Para::execute_with(|| {
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
