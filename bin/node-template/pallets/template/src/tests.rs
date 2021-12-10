use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

#[test]
fn iris_initial_state() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			// Iris::DataQueue::<Test>::get(),
			// <pallet::Pallet<mock::Test> as Trait>
			crate::DataQueue::<Test>::get(),
			None
		);
	});
}

// #[test]
// fn iris_ipfs_add_bytes_works_for_valid_value() {
// 	let multiaddr_vec = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWMvyvKxYcy9mjbFbXcogFSCvENzQ62ogRxHKZaksFCkAp".to_vec();
// 	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".to_vec;
// 	let name = "test.txt".to_vec;
// 	let cost = 1;
// 	new_test_ext().execute_with(|| {
// 		// invoke ipfs_add_bytes extrinsic
// 		assert_ok!(Iris::create_storage_asset(
// 			Origin::signed(1),
// 			multiaddr_vec,
// 			cid_vec
// 		));
// 		// assert_eq!(Iris::data_queue(), Some(AddBytes(OpaqueMultiaddr(multiaddr_vec), cid_vec)));
// 	});
// }

