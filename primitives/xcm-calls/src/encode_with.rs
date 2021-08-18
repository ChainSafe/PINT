// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

//! Additional types for the remote asset manager pallet
use codec::{Encode, EncodeAsRef, HasCompact, Output};
use frame_support::{sp_runtime::MultiAddress, sp_std::marker::PhantomData};

/// A helper to encode an item using the provided context
pub trait EncodeWith<Input, Context> {
	/// Same as `Encode::encode_to` but with additional context
	fn encode_to_with<T: Output + ?Sized>(input: &Input, ctx: &Context, dest: &mut T);
}

/// Encodes the type as it is
pub struct PassthroughEncoder<I, T>(PhantomData<(I, T)>);

impl<I: Encode, Context> EncodeWith<I, Context> for PassthroughEncoder<I, Context> {
	fn encode_to_with<T: Output + ?Sized>(input: &I, _: &Context, dest: &mut T) {
		input.encode_to(dest)
	}
}

/// Encodes the type as it is but compact
pub struct PassthroughCompactEncoder<I, T>(PhantomData<(I, T)>);

impl<I: HasCompact, Context> EncodeWith<I, Context> for PassthroughCompactEncoder<I, Context> {
	fn encode_to_with<T: Output + ?Sized>(input: &I, _: &Context, dest: &mut T) {
		<<I as HasCompact>::Type as EncodeAsRef<'_, I>>::RefType::from(input).encode_to(dest)
	}
}

/// Encodes an `AccountId` as `Multiaddress` regardless of the asset id
pub struct MultiAddressLookupSourceEncoder<AccountId, AccountIndex, Context>(
	PhantomData<(AccountId, AccountIndex, Context)>,
);

impl<AccountId, AccountIndex, Context> EncodeWith<AccountId, Context>
	for MultiAddressLookupSourceEncoder<AccountId, AccountIndex, Context>
where
	AccountId: Encode + Clone,
	AccountIndex: HasCompact,
{
	fn encode_to_with<T: Output + ?Sized>(account: &AccountId, _: &Context, dest: &mut T) {
		MultiAddress::<AccountId, AccountIndex>::from(account.clone()).encode_to(dest)
	}
}
