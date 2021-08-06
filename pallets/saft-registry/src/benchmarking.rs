// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::assert_ok;
use frame_support::{
	dispatch::UnfilteredDispatchable,
	traits::{EnsureOrigin, Get},
};
use frame_system::RawOrigin;

use crate::Pallet as SaftRegistry;

const SEED: u32 = 0;
fn whitelisted_account<T: Config>(name: &'static str, counter: u32) -> T::AccountId {
	let acc = account(name, counter, SEED);
	whitelist_acc::<T>(&acc);
	acc
}

fn whitelist_acc<T: Config>(acc: &T::AccountId) {
	frame_benchmarking::benchmarking::add_to_whitelist(frame_system::Account::<T>::hashed_key_for(acc).into());
}

benchmarks! {
	add_saft {
		let asset: T::AssetId = 0u32.into();
		let origin = T::AdminOrigin::successful_origin();
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

	// TODO:
	//
	// require `remote_asset` of pallet asset-index
	//
	// https://github.com/ChainSafe/PINT/pull/73
	//
	// remove_saft {
	//     assert_ok!(SaftRegistry::<T>::add_saft(<Origin<T>>::Root.into(), 0.into(), 100_u32.into(), 20_u32.into()));
	// }: _(
	//     // <Origin<T>>::Signed(whitelisted_caller()),
	//     <Origin<T>>::Root,
	//     0.into(),
	//     0
	// ) verify {
	// }

	report_nav {
		let asset: T::AssetId = 0u32.into();
		let origin = T::AdminOrigin::successful_origin();
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
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::assert_ok;

	#[test]
	fn add_saft() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_add_saft::<Test>());
		});
	}

	#[test]
	fn report_nav() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_report_nav::<Test>());
		});
	}
}
