use frame_support::{assert_noop, assert_ok, parameter_types};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use super::*;
use crate::mock::*;
use crate::{self as pallet_updater, Config};


#[test]
fn test_genesis_work() {
	new_test_ext().execute_with(|| {
		let members = Updater::members();

		assert_eq!(members.len(), 1usize);
		assert_eq!(vec![1u64], members)
	})
}

#[test]
fn test_add_member_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Updater::add_member(Origin::signed(1), 2));

		let members = Updater::members();

		assert_eq!(members.len(), 2usize);
		assert!(members.contains(&2));
	})
}

#[test] 
fn test_add_member_fail() { 
	new_test_ext().execute_with(|| { 
		Updater::add_member(Origin::signed(1), 2);

		assert_noop!(Updater::add_member(Origin::signed(1), 2),
					Error::<Test>::SameAccount);		

		assert_noop!(Updater::add_member(Origin::signed(3), 4), 
					Error::<Test>::NotMember);	
	}); 
} 

#[test]
fn test_remove_member_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Updater::add_member(Origin::signed(1), 2));

		let members = Updater::members();

		assert_eq!(members.len(), 2usize);
		assert!(members.contains(&2));

		assert_ok!(Updater::remove_member(Origin::signed(2), 1));

		let members = Updater::members();

		assert_eq!(members.len(), 1usize);
		assert!(!members.contains(&1));
	})
}

#[test] 
fn test_remove_member_fail() { 
	new_test_ext().execute_with(|| { 
		assert_noop!(Updater::remove_member(Origin::signed(1), 2), 
				Error::<Test>::AccNotExist);

		assert_noop!(Updater::add_member(Origin::signed(2), 1), 
					Error::<Test>::NotMember);
	}); 
} 

#[test]
fn test_propose_code_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(4);

		let code = vec![0u8, 0u8, 0u8];
		let hash = BlakeTwo256::hash_of(&code);

		assert_ok!(Updater::propose_code(Origin::signed(1), code.clone()));
		
		let proposals = Updater::proposals();
		let proposals_cnt = Updater::proposal_count();
		let codes = Updater::codes();
		let vote_single = Updater::voting(hash).unwrap();
		let vote_assert = Votes { index: 0, threshold: 1, ayes: vec![], nays: vec![], end: 4 };

		assert!(proposals.contains(&hash));
		assert_eq!(proposals_cnt, 1);

		assert_eq!(codes, code);

		assert_eq!(vote_single, vote_assert);
	})
}

#[test] 
fn test_propose_code_fail() { 
	new_test_ext().execute_with(|| { 
		let code = vec![0u8, 0u8, 0u8];

		assert_ok!(Updater::propose_code(Origin::signed(1),		
					code.clone()));

		assert_noop!(
			Updater::propose_code(Origin::signed(1), code.clone()),
			Error::<Test>::DuplicateProposal
		);
		assert_noop!(
			Updater::propose_code(Origin::signed(6), code.clone()),
			Error::<Test>::NotMember
		);
	}); 
} 

#[test]
fn test_vote_code_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Updater::add_member(Origin::signed(1), 2));

		let code = vec![0u8, 0u8, 0u8];
		let hash = BlakeTwo256::hash_of(&code);
		let hash_w = BlakeTwo256::hash_of(&vec![0u8]);

		assert_ok!(Updater::propose_code(Origin::signed(1), code.clone()));

		assert_ok!(Updater::vote_code(Origin::signed(1), hash.clone(), 0, true));

		let voting = Updater::voting(&hash).expect("Error with first voting call");

		assert_eq!(voting.index, 0);

		assert_eq!(voting.ayes.len(), 1);

		assert!(voting.ayes.contains(&1));
		
		assert_ok!(Updater::vote_code(Origin::signed(1), hash.clone(), 0, false));

		let voting = Updater::voting(&hash).expect("Error with second voting call");

		assert_eq!(voting.nays.len(), 1);

		assert!(voting.nays.contains(&1));
		

		assert_ok!(Updater::vote_code(Origin::signed(2), hash.clone(), 0, false));

		let voting = Updater::voting(&hash).expect("Error with third voting call");

		assert_eq!(voting.nays.len(), 2);

		assert!(voting.nays.contains(&2));

		assert_ok!(Updater::vote_code(Origin::signed(2), hash.clone(), 0, true));

		let voting = Updater::voting(&hash).expect("Error with fourth voting call");

		assert_eq!(voting.ayes.len(), 1);

		assert!(voting.ayes.contains(&2));
	})
}

#[test] 
fn test_vote_code_fail() { 
	new_test_ext().execute_with(|| { 
		let code = vec![0u8, 0u8, 0u8];
		let hash = BlakeTwo256::hash_of(&code);

		assert_noop!(
			Updater::vote_code(Origin::signed(1), hash.clone(), 0, true),
			Error::<Test>::ProposalMissing
		);

		assert_ok!(Updater::propose_code(Origin::signed(1), code.clone()));

		assert_noop!(
			Updater::vote_code(Origin::signed(1), hash.clone(), 1, true),
			Error::<Test>::WrongIndex
		);

		assert_noop!(
			Updater::vote_code(Origin::signed(5), hash, 0, true),
			Error::<Test>::NotMember
		);
	}); 
} 


#[test]
fn test_close_vote_work() {
	new_test_ext().execute_with(|| {
		let code = vec![0u8, 0u8, 0u8];
		let hash = BlakeTwo256::hash_of(&code);

		assert_ok!(Updater::propose_code(Origin::signed(1), code.clone()));		
		
		assert_ok!(Updater::close_vote(Origin::signed(1), hash.clone(), 0));
	})
}


fn test_close_vote_fail() {
	new_test_ext().execute_with(|| {
		let code = vec![0u8, 0u8, 0u8];
		let hash = BlakeTwo256::hash_of(&code);
		
		assert_noop!(
			Updater::close_vote(Origin::signed(4), hash.clone(), 0),
			Error::<Test>::NotMember
		);

		assert_ok!(Updater::propose_code(Origin::signed(1), code.clone()));		

		assert_noop!(
			Updater::close_vote(Origin::signed(1), hash.clone(), 1),
			Error::<Test>::WrongIndex
		);	
	})
}