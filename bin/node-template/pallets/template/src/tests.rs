use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
/// Import the template pallet.
pub use pallet_iris;

fn register_offchain_ext(ext: &mut sp_io::TestExternalities) {
	let (offchain, _offchain_state) = TestOffchainExt::with_offchain_db(ext.offchain_db());
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
}

fn new_block() -> u64 {
	let number = frame_system::Pallet::<Test>::block_number() + 1;
	let hash = H256::repeat_byte(number as u8);
	LEAF_DATA.with(|r| r.borrow_mut().a = number);

	frame_system::Pallet::<Test>::initialize(
		&number,
		&hash,
		&Default::default(),
		frame_system::InitKind::Full,
	);
	Iris::on_initialize(number)
}

#[test]
fn should_start_empty() {
	let _ = env_logger::try_init();
	new_test_ext().execute_with(|| {
		// given
		assert_eq!(
			crate::DataQueue::<Test>::get(),
			None
		);

		// when
		let weight = new_block();

		// then
		assert_eq!(
			crate::DataQueue::<Test>::get(),
			None
		);
	});
}

#[test]
fn ipfs_add_bytes_works_for_valid_value() {
	let multiaddr_vec = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWMvyvKxYcy9mjbFbXcogFSCvENzQ62ogRxHKZaksFCkAp".to_vec();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".to_vec;
	let name = "test.txt".to_vec;
	let cost = 1;
	new_test_ext().execute_with(|| {
		// invoke ipfs_add_bytes extrinsic
		assert_ok!(Iris::create_storage_asset(
			Origin::signed(1),
			multiaddr_vec,
			cid_vec));
		// assert_eq!(Iris::data_queue(), Some(AddBytes(OpaqueMultiaddr(multiaddr_vec), cid_vec)));
	});
}

