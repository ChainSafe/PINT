// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

use crate as pallet;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use pallet::PriceFeed as _;
use primitives::Price;
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

const ASSET_X_ID: AssetId = 2;
const ASSET_Y_ID: AssetId = 3;

#[test]
fn feed_creation_and_mapping_should_work() {
	new_test_ext().execute_with(|| {
		// insert two feeds
		assert_ok!(FeedBuilder::new().description(b"PINT".to_vec()).build_and_store());
		assert_ok!(FeedBuilder::new().description(b"X".to_vec()).build_and_store());

		// PINT asset id is not tracked yet
		assert_noop!(PriceFeed::get_price(PINTAssetId::get()), Error::<Test>::AssetPriceFeedNotFound);

		// map feed 0 to PINT
		assert_ok!(PriceFeed::map_asset_price_feed(Origin::signed(ADMIN_ACCOUNT_ID), PINTAssetId::get(), 0));

		// map feed 1 to assetId 2
		assert_ok!(PriceFeed::map_asset_price_feed(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_X_ID, 1));

		assert_ok!(PriceFeed::unmap_asset_price_feed(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_X_ID));
	});
}

#[test]
fn non_admin_cannot_map_feeds() {
	new_test_ext().execute_with(|| {
		assert_ok!(FeedBuilder::new().description(b"PINT".to_vec()).build_and_store());

		assert_noop!(PriceFeed::map_asset_price_feed(Origin::signed(1), PINTAssetId::get(), 0), BadOrigin);
	})
}

#[test]
fn cannot_get_price_pair_for_feed_without_valid_round() {
	new_test_ext().execute_with(|| {
		// insert two feeds
		assert_ok!(FeedBuilder::new().description(b"PINT".to_vec()).build_and_store());
		assert_ok!(FeedBuilder::new().description(b"X".to_vec()).build_and_store());

		assert_ok!(PriceFeed::map_asset_price_feed(Origin::signed(ADMIN_ACCOUNT_ID), PINTAssetId::get(), 0));
		assert_ok!(PriceFeed::map_asset_price_feed(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_X_ID, 1));
		assert_noop!(PriceFeed::get_price(ASSET_X_ID), Error::<Test>::InvalidFeedValue);
	})
}

#[test]
fn price_pair_should_be_available() {
	new_test_ext().execute_with(|| {
		// insert two feeds
		assert_ok!(FeedBuilder::new().description(b"X".to_vec()).min_submissions(1).decimals(0).build_and_store());
		assert_ok!(FeedBuilder::new().description(b"Y".to_vec()).min_submissions(1).decimals(0).build_and_store());

		assert_ok!(PriceFeed::map_asset_price_feed(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_X_ID, 0));
		assert_ok!(PriceFeed::map_asset_price_feed(Origin::signed(ADMIN_ACCOUNT_ID), ASSET_Y_ID, 1));

		// insert round feed 1
		let feed_id = 0;
		let round_id = 1;
		let oracle = 2;
		let base_submission = 1_000;
		assert_ok!(ChainlinkFeed::submit(Origin::signed(oracle), feed_id, round_id, base_submission));

		// insert round feed 2
		let feed_id = 1;
		let round_id = 1;
		let oracle = 2;
		let quote_submission = 200;
		assert_ok!(ChainlinkFeed::submit(Origin::signed(oracle), feed_id, round_id, quote_submission));

		let base_price = PriceFeed::get_price(ASSET_X_ID).expect("price pair should be available");
		assert_eq!(base_price, Price::saturating_from_integer(base_submission as u128));

		let quote_price = PriceFeed::get_price(ASSET_Y_ID).expect("price pair should be available");
		assert_eq!(quote_price, Price::saturating_from_integer(quote_submission as u128));

		let pair = PriceFeed::get_relative_price_pair(ASSET_X_ID, ASSET_Y_ID).expect("relative price available");

		assert_eq!(pair.price, Price::saturating_from_rational(base_submission, quote_submission));
	})
}
