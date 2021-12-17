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
// 	Iris::on_initialize(number)
// }

#[test]
fn ipfs_initial_state() {
	new_test_ext().execute_with(|| {
		assert_eq!(0, 1);
	});
}
