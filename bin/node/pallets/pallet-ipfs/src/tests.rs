use super::*;
use frame_support::assert_ok;
use mock::*;
use sp_core::offchain::{testing, IpfsResponse, OffchainDbExt, OffchainWorkerExt, TransactionPoolExt};
use sp_core::Pair;
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use std::sync::Arc;

#[test]
fn cherry_initial_state() {
	new_test_ext().execute_with(|| {
		let data_queue = crate::DataQueue::<Test>::get();
		let len = data_queue.len();

		assert_eq!(len, 0);
	});
}

#[test]
fn cherry_ipfs_add_bytes_works_for_valid_value() {
	let (p, _) = sp_core::sr25519::Pair::generate();
	let multiaddr_vec =
		"/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWMvyvKxYcy9mjbFbXcogFSCvENzQ62ogRxHKZaksFCkAp"
			.as_bytes()
			.to_vec();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let size = 1024;
	let reserve_price = 100;

	let expected_data_command = crate::DataCommand::AddBytes(
		OpaqueMultiaddr(multiaddr_vec.clone()),
		cid_vec.clone(),
		size.clone(),
		reserve_price.clone(),
		p.clone().public(),
		true,
	);

	new_test_ext_funded(p.clone()).execute_with(|| {
		assert_ok!(mock::Ipfs::create_ipfs_asset(
			Origin::signed(p.clone().public()),
			multiaddr_vec.clone(),
			cid_vec.clone(),
			size.clone(),
			reserve_price.clone(),
		));

		let mut data_queue = crate::DataQueue::<Test>::get();
		let len = data_queue.len();
		assert_eq!(len, 1);
		let actual_data_command = data_queue.pop();
		assert_eq!(actual_data_command, Some(expected_data_command));
	});
}

#[test]
fn cherry_can_add_bytes_to_ipfs() {
	let (p, _) = sp_core::sr25519::Pair::generate();
	let (offchain, state) = testing::TestOffchainExt::new();
	let (pool, _) = testing::TestTransactionPoolExt::new();
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(
		&keystore,
		crate::KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.unwrap();

	let mut t = new_test_ext_funded(p.clone());
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let multiaddr_vec =
		"/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWJdTrFkpPFMi4112oJUKeNvU4Haskg2rqNRCjdgc1yyVH"
			.as_bytes()
			.to_vec();
	let cid_vec = "QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9".as_bytes().to_vec();
	let size = 1024;
	let reserve_price = 100;
	let bytes = "sjdasdadasdjasdlasd".as_bytes().to_vec();

	// mock IPFS calls
	// These have to represent exactly the same ipfs_requests that
	// are done in your actual code and wit the same order.
	// - @charmitro
	{
		let mut state = state.write();
		// connect to external node
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::Success),
			..Default::default()
		});
		// fetch data
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			id: sp_core::offchain::IpfsRequestId(0),
			response: Some(IpfsResponse::CatBytes(bytes.clone())),
			..Default::default()
		});
		// disconnect from the external node
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::Success),
			..Default::default()
		});
		// add bytes to your local node
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::AddBytes(cid_vec.clone())),
			..Default::default()
		});
		// insert pin
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::Success),
			..Default::default()
		});
		// disconnect
		state.expect_ipfs_request(testing::IpfsPendingRequest {
			response: Some(IpfsResponse::Success),
			..Default::default()
		});
}

	t.execute_with(|| {
		assert_ok!(mock::Ipfs::create_ipfs_asset(
			Origin::signed(p.clone().public()),
			multiaddr_vec.clone(),
			cid_vec.clone(),
			size.clone(),
			reserve_price.clone(),
		));

		assert_ok!(mock::Ipfs::submit_ipfs_add_results(
			Origin::signed(p.clone().public()),
			p.clone().public(),
			cid_vec.clone(),
			size.clone(),
			reserve_price.clone(),
		));

		assert_ok!(mock::Ipfs::handle_data_requests());
	});
}
