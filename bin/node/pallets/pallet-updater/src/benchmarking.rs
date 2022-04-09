//! Benchmarking for pallet-updater.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Updater;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::AccountId32 as AccountId;

benchmarks! {
    iter_propose {
        let origin = whitelisted_caller();
        let code = vec![0u8];
        
        for _ in 1000 {
            Updater::propose_code(origin, code);
        }

    }
}


impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
