use super::*;

#[allow(unused)]
use crate::Pallet as Ipfs;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, vec::Vec, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	create_ipfs_asset {
		let caller: T::AccountId = whitelisted_caller();
		let addr = Vec::new();
	}: _(RawOrigin::Signed(caller), addr.clone(), addr.clone())
}

impl_benchmark_test_suite!(Ipfs, crate::mock::new_test_ext(), crate::mock::Test);
