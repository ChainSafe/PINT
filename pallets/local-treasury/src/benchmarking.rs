// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_support::{sp_runtime::traits::AccountIdConversion, traits::Currency, PalletId};
use frame_system::Origin;

benchmarks! {
	withdraw {
		let local_treasury: <T as frame_system::Config>::AccountId = PalletId(*b"Treasury").into_account_truncating();
		T::Currency::deposit_creating(&local_treasury, 10_000_000_u32.into());
		let admin: <T as frame_system::Config>::AccountId = account("admin", 0, 0);
	}: _(
		<Origin<T>>::Root,
		5_000_000_u32.into(),
		admin.clone()
	) verify {
		assert_eq!(
			T::Currency::free_balance(&admin),
			5_000_000_u32.into(),
		);
	}
}
