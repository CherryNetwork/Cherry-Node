use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use sp_core::Pair;

// fn new_block() -> u64 {
// 	let number = frame_system::Pallet::<Test>::block_number() + 1;
// 	let hash = H256::repeat_byte(number as u8);
// 	LEAF_DATA.with(|r| r.borrow_mut().a = number);

// 	frame_system::Pallet::<Test>::initialize(
// 		&number,
// 		&hash,
// 		&Default::default(),
// 		frame_system::InitKind::Full,
// 	);
// 	Cherry::on_initialize(number)
// }

#[test]
fn cherry_initial_state() {
	new_test_ext().execute_with(|| {
		// Given: The node is initialized at block 0
		// When: I query runtime storagey
		let data_queue = crate::DataQueue::<Test>::get();
		let len = data_queue.len();
		// Then: Runtime storage is empty
		assert_eq!(len, 0);
	});
}

#[test]
fn cherry_ipfs_add_bytes_works_for_valid_value() {
	// Given: I am a valid node with a positive balance
	let (p, _) = sp_core::sr25519::Pair::generate();
	let multiaddr_vec = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWMvyvKxYcy9mjbFbXcogFSCvENzQ62ogRxHKZaksFCkAp".as_bytes().to_vec();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let name: Vec<u8> = "test.txt".as_bytes().to_vec();
	let cost = 1;
	let id = 1;
	let balance = 1;

	// 
	let expected_data_command = crate::DataCommand::AddBytes(
		OpaqueMultiaddr(multiaddr_vec.clone()),
		cid_vec.clone(),
		p.clone().public(),
		name.clone(),
		id.clone(),
		balance.clone(),
	);

	new_test_ext_funded(p.clone()).execute_with(|| {
		// WHEN: I invoke the create_storage_assets extrinsic
		assert_ok!(Cherry::create_storage_asset(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			multiaddr_vec.clone(),
			cid_vec.clone(),
			name.clone(),
			id.clone(),
			balance.clone(),
		));

		// THEN: There is a single DataCommand::AddBytes in the DataQueue
		let mut data_queue = crate::DataQueue::<Test>::get();
		let len = data_queue.len();
		assert_eq!(len, 1);
		let actual_data_command = data_queue.pop();
		assert_eq!(actual_data_command, Some(expected_data_command));
	});
}

#[test]
fn cherry_request_data_works_for_valid_values() {
	// GIVEN: I am a valid  Cherrynode with a positive balance
	let (p, _) = sp_core::sr25519::Pair::generate();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();

	let expected_data_command = crate::DataCommand::CatBytes(
		p.clone().public(),
		cid_vec.clone(),
		p.clone().public(),
	);
	new_test_ext_funded(p.clone()).execute_with(|| {
		// WHEN: I invoke the request_data extrinsic
		assert_ok!(Cherry::request_data(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
		));

		// THEN: There should be a single DataCommand::CatBytes in the DataQueue
		let mut data_queue = crate::DataQueue::<Test>::get();
		let len = data_queue.len();
		assert_eq!(len, 1);
		let actual_data_command = data_queue.pop();
		assert_eq!(actual_data_command, Some(expected_data_command));
	});
}

#[test]
fn cherry_submit_ipfs_add_results_works_for_valid_values() {
	// GIVEN: I am a valid  Cherrynode with a positive valance
	let (p, _) = sp_core::sr25519::Pair::generate();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let id = 1;
	let balance = 1;

	new_test_ext_funded(p.clone()).execute_with(|| {
		// WHEN: I invoke the submit_ipfs_add_results extrinsic
		assert_ok!(Cherry::submit_ipfs_add_results(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
			id.clone(),
			balance.clone(),
		));

		// THEN: a new asset class is created
		// AND: A new entry is added to the AssetClassOwnership StorageDoubleMap
		let admin_asset_class_id = crate::AssetClassOwnership::<Test>::get(p.clone().public(), cid_vec.clone());
		assert_eq!(admin_asset_class_id, id.clone());
	});
}

#[test]
fn cherry_mint_tickets_works_for_valid_values() {
	// GIVEN: I am a valid  Cherrynode with a positive valance
	let (p, _) = sp_core::sr25519::Pair::generate();
	let (p, _) = sp_core::sr25519::Pair::generate();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let balance = 1;
	let id = 1;

	new_test_ext_funded(p.clone()).execute_with(|| {
		// AND: I create an owned asset class
		assert_ok!(Cherry::submit_ipfs_add_results(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
			id.clone(),
			balance.clone(),
		));
		// WHEN: I invoke the mint_tickets extrinsic
		assert_ok!(Cherry::mint_tickets(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
			balance.clone(),
		));
		// THEN: new assets are created and awarded to the benficiary
		// AND: A new entry is added to the AssetAccess StorageDoubleMap
		let asset_class_owner = crate::AssetAccess::<Test>::get(p.clone().public(), cid_vec.clone());
		assert_eq!(asset_class_owner, p.clone().public())
	});
}

#[test]
fn cherry_submit_rpc_ready_works_for_valid_values() {
	let (p, _) = sp_core::sr25519::Pair::generate();
	new_test_ext_funded(p.clone()).execute_with(|| {
		assert_ok!(Cherry::submit_rpc_ready(
			Origin::signed(p.clone().public()),
			p.clone().public(),
		));
	});
}

// #[test]
// fn cherry_purchase_tickets_works_for_valid_values() {
// 	let (p, _) = sp_core::sr25519::Pair::generate();
// 	new_test_ext_funded(p.clone()).execute_with(|| {
// 		assert_ok!(Cherry::purchase_ticket(
// 			Origin::signed(p.clone().public()),
// 			p.clone().public(),
// 		));
// 	});
// }
