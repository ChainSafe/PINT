// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::benchmarks;
use frame_support::{assert_ok, dispatch::UnfilteredDispatchable, sp_runtime::traits::Zero, traits::EnsureOrigin};
use primitives::traits::AssetRecorderBenchmarks;
use xcm::v1::{Junction, MultiLocation};

use crate::Pallet as SaftRegistry;

use super::*;

const MAX_SAFT_RECORDS: u32 = 100;

benchmarks! {
	add_saft {
		let asset: T::AssetId = T::try_convert(2u8).unwrap();
		let origin = T::AdminOrigin::successful_origin();

		assert_ok!(T::AssetRecorderBenchmarks::add_asset(
			T::try_convert(3u8).unwrap(),
			100u32.into(),
			MultiLocation::default(),
			1000u32.into()
		));

		let call = Call::<T>::add_saft(
				asset,
				100u32.into(),
				20u32.into()
		);
	}: { call.dispatch_bypass_filter(origin)? }
	 verify {
		let id = SaftRegistry::<T>::saft_counter(asset) - 1;
		assert_eq!(
			SaftRegistry::<T>::active_safts(asset, id),
			Some(SAFTRecord::new(100_u32.into(), 20_u32.into()))
		);
	}

	remove_saft {
		let asset: T::AssetId = T::try_convert(2u8).unwrap();
		let origin = T::AdminOrigin::successful_origin();
		let nav = 100u32.into();
		let units = 20u32.into();

		assert_ok!(T::AssetRecorderBenchmarks::add_asset(
			T::try_convert(3u8).unwrap(),
			100u32.into(),
			MultiLocation::default(),
			1000u32.into()
		));

		assert_ok!(T::AssetRecorderBenchmarks::deposit_saft_equivalent(nav));
		assert_ok!(SaftRegistry::<T>::add_saft(origin.clone(), asset, nav, units));

		let call = Call::<T>::remove_saft(
				asset,
				0u32
		);
	}:  { call.dispatch_bypass_filter(origin)? }
		verify {
			assert!(
				SaftRegistry::<T>::active_safts(asset, 0).is_none()
			)
	}

	report_nav {
		let asset: T::AssetId = T::try_convert(2u8).unwrap();
		let origin = T::AdminOrigin::successful_origin();

		assert_ok!(T::AssetRecorderBenchmarks::add_asset(
			T::try_convert(3u8).unwrap(),
			100u32.into(),
			MultiLocation::default(),
			1000u32.into()
		));

		assert_ok!(SaftRegistry::<T>::add_saft(
			origin.clone(),
			asset,
			100_u32.into(),
			20_u32.into(),
		));

		let call = Call::<T>::report_nav(
					asset,
		0,
		1000_u32.into()
		);
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_eq!(
			SaftRegistry::<T>::active_safts(asset, 0u32),
			Some(SAFTRecord::new(1000_u32.into(), 20_u32.into()))
		);
	}

	convert_to_liquid {
		let o in 1 .. MAX_SAFT_RECORDS;

		let nav = 1337u32;
		let units = 1234u32;
		let asset:T::AssetId = T::try_convert(5u8).unwrap();
		let origin = T::AdminOrigin::successful_origin();

		assert_ok!(T::AssetRecorderBenchmarks::add_asset(
			T::try_convert(3u8).unwrap(),
			100u32.into(),
			MultiLocation::default(),
			1000u32.into()
		));

		assert_ok!(SaftRegistry::<T>::add_saft(
			origin.clone(),
			asset,
			nav.into(),
			units.into(),
		));

		assert_ok!(<SAFTCounter<T>>::try_mutate(asset, |counter: &mut u32| -> Result<(), ()> {
			*counter = o;
			Ok(())
		}));

		let call = Call::<T>::convert_to_liquid(
			asset,
			(Junction::Parachain(100)).into()
		);
	}: { call.dispatch_bypass_filter(origin)? } verify {
		assert_eq!(
			SaftRegistry::<T>::saft_counter(asset),
			0
		);
		assert!(
			SaftRegistry::<T>::saft_nav(asset).is_zero()
		);
	}
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;

	use crate::mock::{new_test_ext, Test};

	use super::*;

	#[test]
	fn add_saft() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_add_saft());
		});
	}

	#[test]
	fn remove_saft() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_remove_saft());
		});
	}

	#[test]
	fn report_nav() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_report_nav());
		});
	}

	#[test]
	fn convert_to_liquid() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_convert_to_liquid());
		});
	}
}
