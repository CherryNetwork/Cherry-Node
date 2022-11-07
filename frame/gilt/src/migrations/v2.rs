use crate::*;
use frame_support::{ 
    traits::{
        Get, GetStorageVersion, StorageVersion, PalletInfoAccess, 
    },
    weights::Weight, BoundedVec,
};
use sp_runtime::traits::Zero;
use sp_std::vec;

pub fn migrate<T: frame_system::Config + crate::Config, P: GetStorageVersion + PalletInfoAccess>() -> Weight {
    let on_chain_storage_version = <P as GetStorageVersion>::on_chain_storage_version();
	log::info!(
		target: "runtime::gilt",
		"Running migration to v2 for gilt with storage version {:?}",
		on_chain_storage_version,
	);

    if on_chain_storage_version < 4 {
        let unbounded = vec![(0, BalanceOf::<T>::zero()); T::QueueCount::get() as usize];
        let bounded: BoundedVec<_, _> = unbounded
            .try_into()
            .expect("QueueTotals should support up to QueueCount items. qed");
        QueueTotals::<T>::put(bounded);
		log_migration("migration");

		StorageVersion::new(2).put::<P>();
		<T as frame_system::Config>::BlockWeights::get().max_block
	} else {
		log::warn!(
			target: "runtime::gilt",
			"Attempted to apply migration to v2 but failed because storage version is {:?}",
			on_chain_storage_version,
		);
		0
	}
}

fn log_migration(stage: &str) {
	log::info!(
		target: "runtime::gilt",
		"{}",
		stage,
	);
}