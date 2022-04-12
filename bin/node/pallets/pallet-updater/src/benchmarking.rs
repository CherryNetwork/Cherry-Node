//! Benchmarking for pallet-updater.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Updater;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use hex_literal::hex;
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::AccountId32;

benchmarks! {
	add_member {
		let caller: T::AccountId = whitelisted_caller();
		let member_id = account("member", 1, 0);
	}: _(RawOrigin::Signed(caller), member_id)

	remove_member {
		let caller: T::AccountId = whitelisted_caller();
		let member_id = account("member", 1, 0);
	}: _(RawOrigin::Signed(caller), member_id)

	propose_code {
		let caller: T::AccountId = whitelisted_caller();
		let code = vec![0u8];
	}: _(RawOrigin::Signed(caller), code)

	vote_code {
		let caller: T::AccountId = whitelisted_caller();
		let code = vec![0u8];
		let hash = T::Hashing::hash_of(&code);
		let index =  0;
		let approve = true;
	}: _(RawOrigin::Signed(caller), hash, index, approve)

	close_vote {
		let caller: T::AccountId = whitelisted_caller();
		let code = vec![0u8];
		let hash = T::Hashing::hash_of(&code);
		let index =  0;
	}: _(RawOrigin::Signed(caller), hash, index)

}

impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
