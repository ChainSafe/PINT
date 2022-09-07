// // Copyright 2021 ChainSafe Systems
// // SPDX-License-Identifier: LGPL-3.0-only
//
use crate as pallet;
use crate::{mock::*, Error, mock};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use sp_runtime::FixedPointNumber;
use pallet::PriceFeed as _;
use primitives::Price;

const ASSET_X_ID: AssetId = 2;
const ASSET_Y_ID: AssetId = 3;
const ASSET_Z_ID: AssetId = 4;
const FLOAT_Y_VALUE: f64 = 4.0;
const FLOAT_Z_VALUE: f64 = 2.0;
const ASSET_X_VALUE: Price = Price::from_u32(1);

#[test]
fn get_price_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(OrmlOracle::feed_values(Origin::signed(1), vec![(ASSET_X_ID, ASSET_X_VALUE)]));
		let value = PriceFeed::get_price(ASSET_X_ID).unwrap();
		assert_eq!(value, ASSET_X_VALUE);
	})
}

#[test]
fn price_pair_should_be_available() {
	new_test_ext().execute_with(|| {
		// insert two feeds
		assert_ok!(OrmlOracle::feed_values(
			Origin::signed(2),
			vec![(ASSET_Y_ID, Price::from_float(FLOAT_Y_VALUE)),
				(ASSET_Z_ID, Price::from_float(FLOAT_Z_VALUE))])
		);
		let pair = PriceFeed::get_relative_price_pair(ASSET_Y_ID, ASSET_Z_ID).expect("relative price available");
		assert_eq!(pair.price, Price::from_float(FLOAT_Y_VALUE/FLOAT_Z_VALUE));
	})
}
