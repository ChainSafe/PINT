// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::mock::*;

use codec::Encode;
use frame_support::assert_ok;
use xcm::v0::{
    Junction::{self, Parachain, Parent},
    MultiAsset::*,
    MultiLocation::*,
    NetworkId, OriginKind,
    Xcm::*,
};
use xcm_simulator::TestExt;

fn print_events<T: frame_system::Config>(context: &str) {
    println!("------ {:?} events ------", context);
    frame_system::Pallet::<T>::events().iter().for_each(|r| {
        println!("{:?}", r.event);
    });
}

#[test]
fn can_bond() {
    MockNet::reset();

    // Relay::execute_with(|| {
    //     assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
    // 			relay::Origin::signed(ALICE),
    // 			X1(Parachain(1)),
    // 			X1(Junction::AccountId32 {
    // 				network: NetworkId::Any,
    // 				id: ALICE.into(),
    // 			}),
    // 			vec![ConcreteFungible { id: Null, amount: 123 }],
    // 			123,
    // 		));
    // });
    //
    // ParaA::execute_with(|| {
    //     // free execution, full amount received
    //     assert_eq!(
    //         pallet_balances::Pallet::<para::Runtime>::free_balance(&ALICE),
    //         INITIAL_BALANCE + 123
    //     );
    //
    //     print_events::<para::Runtime>("ParaA");
    // });
}
