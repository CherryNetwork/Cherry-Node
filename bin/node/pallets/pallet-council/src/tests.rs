use super::*;
use crate::{self as pallet_council, Config};
use frame_support::{assert_noop, assert_ok, parameter_types};
use mock::*;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

#[test]
fn motions_basic_environment_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(pallet_council::Pallet::<Test>::members(), vec![1, 2, 3]);
		assert_eq!(*pallet_council::Pallet::<Test>::proposals(), Vec::<H256>::new());
	});
}

#[test]
fn close_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(set_gov_token_id(<Test as pallet::Config>::Origin::root()));
		let proposal = make_proposal(42);
		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(pallet_council::Pallet::<Test>::propose(
			<Test as pallet::Config>::Origin::signed(1),
			3,
			Box::new(proposal.clone()),
			proposal_len
		));

		assert_noop!(
			pallet_council::Pallet::<Test>::close(
				<Test as pallet::Config>::Origin::signed(4),
				hash,
				0,
				proposal_weight,
				proposal_len
			),
			Error::<Test>::TooEarly
		);

		assert_ok!(pallet_council::Pallet::<Test>::vote(
			<Test as pallet::Config>::Origin::signed(1),
			hash,
			0,
			true
		));
		assert_ok!(pallet_council::Pallet::<Test>::vote(
			<Test as pallet::Config>::Origin::signed(2),
			hash,
			0,
			true
		));

		System::set_block_number(4);
		assert_ok!(pallet_council::Pallet::<Test>::close(
			<Test as pallet::Config>::Origin::signed(4),
			hash,
			0,
			proposal_weight,
			proposal_len
		));

		assert_eq!(
			System::events(),
			vec![
				record(mock::Event::Council(pallet_council::Event::<Test>::Proposed(
					1, 0, hash, 3
				))),
				record(mock::Event::Council(pallet_council::Event::<Test>::Voted(
					1, hash, true, 100, 0
				))),
				record(mock::Event::Council(pallet_council::Event::<Test>::Voted(
					2, hash, true, 200, 0
				))),
				record(mock::Event::Council(pallet_council::Event::<Test>::Closed(hash, 200, 0))),
				record(mock::Event::Council(pallet_council::Event::<Test>::Approved(hash))),
				record(mock::Event::Council(pallet_council::Event::<Test>::Executed(
					hash,
					Err(DispatchError::BadOrigin)
				))),
			]
		);
	});
}

#[test]
fn proposal_weight_limit_works_on_approve() {
	new_test_ext().execute_with(|| {
		let proposal = mock::Call::Council(crate::Call::set_members {
			new_members: vec![1, 2, 3],
			prime: None,
			old_count: MaxMembers::get(),
		});
		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().weight;
		let hash = BlakeTwo256::hash_of(&proposal);
		assert_ok!(pallet_council::Pallet::<Test>::propose(
			<Test as pallet::Config>::Origin::signed(1),
			3,
			Box::new(proposal.clone()),
			proposal_len
		));
		assert_ok!(pallet_council::Pallet::<Test>::vote(<Test as pallet::Config>::Origin::signed(1), hash, 0, true));
		System::set_block_number(4);
		assert_noop!(
			pallet_council::Pallet::<Test>::close(<Test as pallet::Config>::Origin::signed(4), hash, 0, proposal_weight - 100, proposal_len),
			Error::<Test>::WrongProposalWeight
		);
		assert_ok!(pallet_council::Pallet::<Test>::close(<Test as pallet::Config>::Origin::signed(4), hash, 0, proposal_weight, proposal_len));
	})
}

#[test]
fn proposal_weight_limit_ignored_on_disapprove() {
	new_test_ext().execute_with(|| {
		let proposal = mock::Call::Council(crate::Call::set_members {
			new_members: vec![1, 2, 3],
			prime: None,
			old_count: MaxMembers::get(),
		});
		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(pallet_council::Pallet::<Test>::propose(
			<Test as pallet::Config>::Origin::signed(1),
			3,
			Box::new(proposal.clone()),
			proposal_len
		));
		// No votes, this proposal wont pass
		System::set_block_number(4);
		assert_ok!(pallet_council::Pallet::<Test>::close(
			<Test as pallet::Config>::Origin::signed(4),
			hash,
			0,
			proposal_weight - 100,
			proposal_len
		));
	})
}

// #[test]
// fn removal_of_old_voters_votes_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash = BlakeTwo256::hash_of(&proposal);
// 		let end = 4;
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			3,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, true));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 0, true));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 0, threshold: 3, ayes: vec![1, 2], nays: vec![], end })
// 		);
// 		Collective::change_members_sorted(&[4], &[1], &[2, 3, 4]);
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 0, threshold: 3, ayes: vec![2], nays: vec![], end })
// 		);

// 		let proposal = make_proposal(69);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash = BlakeTwo256::hash_of(&proposal);
// 		assert_ok!(Collective::propose(
// 			Origin::signed(2),
// 			2,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 1, true));
// 		assert_ok!(Collective::vote(Origin::signed(3), hash, 1, false));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 1, threshold: 2, ayes: vec![2], nays: vec![3], end })
// 		);
// 		Collective::change_members_sorted(&[], &[3], &[2, 4]);
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 1, threshold: 2, ayes: vec![2], nays: vec![], end })
// 		);
// 	});
// }

// #[test]
// fn removal_of_old_voters_votes_works_with_set_members() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash = BlakeTwo256::hash_of(&proposal);
// 		let end = 4;
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			3,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, true));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 0, true));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 0, threshold: 3, ayes: vec![1, 2], nays: vec![], end })
// 		);
// 		assert_ok!(Collective::set_members(Origin::root(), vec![2, 3, 4], None, MaxMembers::get()));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 0, threshold: 3, ayes: vec![2], nays: vec![], end })
// 		);

// 		let proposal = make_proposal(69);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash = BlakeTwo256::hash_of(&proposal);
// 		assert_ok!(Collective::propose(
// 			Origin::signed(2),
// 			2,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 1, true));
// 		assert_ok!(Collective::vote(Origin::signed(3), hash, 1, false));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 1, threshold: 2, ayes: vec![2], nays: vec![3], end })
// 		);
// 		assert_ok!(Collective::set_members(Origin::root(), vec![2, 4], None, MaxMembers::get()));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 1, threshold: 2, ayes: vec![2], nays: vec![], end })
// 		);
// 	});
// }

// #[test]
// fn propose_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash = proposal.blake2_256().into();
// 		let end = 4;
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			3,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_eq!(*Collective::proposals(), vec![hash]);
// 		assert_eq!(Collective::proposal_of(&hash), Some(proposal));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 0, threshold: 3, ayes: vec![], nays: vec![], end })
// 		);

// 		assert_eq!(
// 			System::events(),
// 			vec![record(Event::Collective(CollectiveEvent::Proposed(1, 0, hash, 3)))]
// 		);
// 	});
// }

// #[test]
// fn limit_active_proposals() {
// 	new_test_ext().execute_with(|| {
// 		for i in 0..MaxProposals::get() {
// 			let proposal = make_proposal(i as u64);
// 			let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 			assert_ok!(Collective::propose(
// 				Origin::signed(1),
// 				3,
// 				Box::new(proposal.clone()),
// 				proposal_len
// 			));
// 		}
// 		let proposal = make_proposal(MaxProposals::get() as u64 + 1);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		assert_noop!(
// 			Collective::propose(Origin::signed(1), 3, Box::new(proposal.clone()), proposal_len),
// 			Error::<Test, Instance1>::TooManyProposals
// 		);
// 	})
// }

// #[test]
// fn correct_validate_and_get_proposal() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = Call::Collective(crate::Call::set_members {
// 			new_members: vec![1, 2, 3],
// 			prime: None,
// 			old_count: MaxMembers::get(),
// 		});
// 		let length = proposal.encode().len() as u32;
// 		assert_ok!(Collective::propose(Origin::signed(1), 3, Box::new(proposal.clone()), length));

// 		let hash = BlakeTwo256::hash_of(&proposal);
// 		let weight = proposal.get_dispatch_info().weight;
// 		assert_noop!(
// 			Collective::validate_and_get_proposal(
// 				&BlakeTwo256::hash_of(&vec![3; 4]),
// 				length,
// 				weight
// 			),
// 			Error::<Test, Instance1>::ProposalMissing
// 		);
// 		assert_noop!(
// 			Collective::validate_and_get_proposal(&hash, length - 2, weight),
// 			Error::<Test, Instance1>::WrongProposalLength
// 		);
// 		assert_noop!(
// 			Collective::validate_and_get_proposal(&hash, length, weight - 10),
// 			Error::<Test, Instance1>::WrongProposalWeight
// 		);
// 		let res = Collective::validate_and_get_proposal(&hash, length, weight);
// 		assert_ok!(res.clone());
// 		let (retrieved_proposal, len) = res.unwrap();
// 		assert_eq!(length as usize, len);
// 		assert_eq!(proposal, retrieved_proposal);
// 	})
// }

// #[test]
// fn motions_ignoring_non_collective_proposals_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		assert_noop!(
// 			Collective::propose(Origin::signed(42), 3, Box::new(proposal.clone()), proposal_len),
// 			Error::<Test, Instance1>::NotMember
// 		);
// 	});
// }

// #[test]
// fn motions_ignoring_non_collective_votes_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash: H256 = proposal.blake2_256().into();
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			3,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_noop!(
// 			Collective::vote(Origin::signed(42), hash, 0, true),
// 			Error::<Test, Instance1>::NotMember,
// 		);
// 	});
// }

// #[test]
// fn motions_ignoring_bad_index_collective_vote_works() {
// 	new_test_ext().execute_with(|| {
// 		System::set_block_number(3);
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash: H256 = proposal.blake2_256().into();
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			3,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_noop!(
// 			Collective::vote(Origin::signed(2), hash, 1, true),
// 			Error::<Test, Instance1>::WrongIndex,
// 		);
// 	});
// }

// #[test]
// fn motions_vote_after_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash: H256 = proposal.blake2_256().into();
// 		let end = 4;
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			2,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		// Initially there a no votes when the motion is proposed.
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 0, threshold: 2, ayes: vec![], nays: vec![], end })
// 		);
// 		// Cast first aye vote.
// 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, true));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 0, threshold: 2, ayes: vec![1], nays: vec![], end })
// 		);
// 		// Try to cast a duplicate aye vote.
// 		assert_noop!(
// 			Collective::vote(Origin::signed(1), hash, 0, true),
// 			Error::<Test, Instance1>::DuplicateVote,
// 		);
// 		// Cast a nay vote.
// 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, false));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 0, threshold: 2, ayes: vec![], nays: vec![1], end })
// 		);
// 		// Try to cast a duplicate nay vote.
// 		assert_noop!(
// 			Collective::vote(Origin::signed(1), hash, 0, false),
// 			Error::<Test, Instance1>::DuplicateVote,
// 		);

// 		assert_eq!(
// 			System::events(),
// 			vec![
// 				record(Event::Collective(CollectiveEvent::Proposed(1, 0, hash, 2))),
// 				record(Event::Collective(CollectiveEvent::Voted(1, hash, true, 1, 0))),
// 				record(Event::Collective(CollectiveEvent::Voted(1, hash, false, 0, 1))),
// 			]
// 		);
// 	});
// }

// #[test]
// fn motions_all_first_vote_free_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash: H256 = proposal.blake2_256().into();
// 		let end = 4;
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			2,
// 			Box::new(proposal.clone()),
// 			proposal_len,
// 		));
// 		assert_eq!(
// 			Collective::voting(&hash),
// 			Some(Votes { index: 0, threshold: 2, ayes: vec![], nays: vec![], end })
// 		);

// 		// For the motion, acc 2's first vote, expecting Ok with Pays::No.
// 		let vote_rval: DispatchResultWithPostInfo =
// 			Collective::vote(Origin::signed(2), hash, 0, true);
// 		assert_eq!(vote_rval.unwrap().pays_fee, Pays::No);

// 		// Duplicate vote, expecting error with Pays::Yes.
// 		let vote_rval: DispatchResultWithPostInfo =
// 			Collective::vote(Origin::signed(2), hash, 0, true);
// 		assert_eq!(vote_rval.unwrap_err().post_info.pays_fee, Pays::Yes);

// 		// Modifying vote, expecting ok with Pays::Yes.
// 		let vote_rval: DispatchResultWithPostInfo =
// 			Collective::vote(Origin::signed(2), hash, 0, false);
// 		assert_eq!(vote_rval.unwrap().pays_fee, Pays::Yes);

// 		// For the motion, acc 3's first vote, expecting Ok with Pays::No.
// 		let vote_rval: DispatchResultWithPostInfo =
// 			Collective::vote(Origin::signed(3), hash, 0, true);
// 		assert_eq!(vote_rval.unwrap().pays_fee, Pays::No);

// 		// acc 3 modify the vote, expecting Ok with Pays::Yes.
// 		let vote_rval: DispatchResultWithPostInfo =
// 			Collective::vote(Origin::signed(3), hash, 0, false);
// 		assert_eq!(vote_rval.unwrap().pays_fee, Pays::Yes);

// 		// Test close() Extrincis | Check DispatchResultWithPostInfo with Pay Info

// 		let proposal_weight = proposal.get_dispatch_info().weight;
// 		let close_rval: DispatchResultWithPostInfo =
// 			Collective::close(Origin::signed(2), hash, 0, proposal_weight, proposal_len);
// 		assert_eq!(close_rval.unwrap().pays_fee, Pays::No);

// 		// trying to close the proposal, which is already closed.
// 		// Expecting error "ProposalMissing" with Pays::Yes
// 		let close_rval: DispatchResultWithPostInfo =
// 			Collective::close(Origin::signed(2), hash, 0, proposal_weight, proposal_len);
// 		assert_eq!(close_rval.unwrap_err().post_info.pays_fee, Pays::Yes);
// 	});
// }

// #[test]
// fn motions_reproposing_disapproved_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let proposal_weight = proposal.get_dispatch_info().weight;
// 		let hash: H256 = proposal.blake2_256().into();
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			3,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 0, false));
// 		assert_ok!(Collective::close(Origin::signed(2), hash, 0, proposal_weight, proposal_len));
// 		assert_eq!(*Collective::proposals(), vec![]);
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			2,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_eq!(*Collective::proposals(), vec![hash]);
// 	});
// }

// // Breaking after commit 8e9fcd17cddb77e0eba78ba50a3045eb8a484225 -- we no longer use
// // pallet_democracy - @charmitro @zycon91 #[test]
// // fn motions_approval_with_enough_votes_and_lower_voting_threshold_works() {
// // 	new_test_ext().execute_with(|| {
// // 		let proposal = Call::Democracy(mock_democracy::Call::external_propose_majority {});
// // 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// // 		let proposal_weight = proposal.get_dispatch_info().weight;
// // 		let hash: H256 = proposal.blake2_256().into();
// // 		// The voting threshold is 2, but the required votes for `ExternalMajorityOrigin` is 3.
// // 		// The proposal will be executed regardless of the voting threshold
// // 		// as long as we have enough yes votes.
// // 		//
// // 		// Failed to execute with only 2 yes votes.
// // 		assert_ok!(Collective::propose(
// // 			Origin::signed(1),
// // 			2,
// // 			Box::new(proposal.clone()),
// // 			proposal_len
// // 		));
// // 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, true));
// // 		assert_ok!(Collective::vote(Origin::signed(2), hash, 0, true));
// // 		assert_ok!(Collective::close(Origin::signed(2), hash, 0, proposal_weight, proposal_len));
// // 		assert_eq!(
// // 			System::events(),
// // 			vec![
// // 				record(Event::Collective(CollectiveEvent::Proposed(1, 0, hash, 2))),
// // 				record(Event::Collective(CollectiveEvent::Voted(1, hash, true, 1, 0))),
// // 				record(Event::Collective(CollectiveEvent::Voted(2, hash, true, 2, 0))),
// // 				record(Event::Collective(CollectiveEvent::Closed(hash, 2, 0))),
// // 				record(Event::Collective(CollectiveEvent::Approved(hash))),
// // 				record(Event::Collective(CollectiveEvent::Executed(
// // 					hash,
// // 					Err(DispatchError::BadOrigin)
// // 				))),
// // 			]
// // 		);

// // 		System::reset_events();

// // 		// Executed with 3 yes votes.
// // 		assert_ok!(Collective::propose(
// // 			Origin::signed(1),
// // 			2,
// // 			Box::new(proposal.clone()),
// // 			proposal_len
// // 		));
// // 		assert_ok!(Collective::vote(Origin::signed(1), hash, 1, true));
// // 		assert_ok!(Collective::vote(Origin::signed(2), hash, 1, true));
// // 		assert_ok!(Collective::vote(Origin::signed(3), hash, 1, true));
// // 		assert_ok!(Collective::close(Origin::signed(2), hash, 1, proposal_weight, proposal_len));
// // 		assert_eq!(
// // 			System::events(),
// // 			vec![
// // 				record(Event::Collective(CollectiveEvent::Proposed(1, 1, hash, 2))),
// // 				record(Event::Collective(CollectiveEvent::Voted(1, hash, true, 1, 0))),
// // 				record(Event::Collective(CollectiveEvent::Voted(2, hash, true, 2, 0))),
// // 				record(Event::Collective(CollectiveEvent::Voted(3, hash, true, 3, 0))),
// // 				record(Event::Collective(CollectiveEvent::Closed(hash, 3, 0))),
// // 				record(Event::Collective(CollectiveEvent::Approved(hash))),
// // 				record(Event::Democracy(mock_democracy::pallet::Event::<Test>::ExternalProposed)),
// // 				record(Event::Collective(CollectiveEvent::Executed(hash, Ok(())))),
// // 			]
// // 		);
// // 	});
// // }

// #[test]
// fn motions_disapproval_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let proposal_weight = proposal.get_dispatch_info().weight;
// 		let hash: H256 = proposal.blake2_256().into();
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			3,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, true));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 0, false));
// 		assert_ok!(Collective::close(Origin::signed(2), hash, 0, proposal_weight, proposal_len));

// 		assert_eq!(
// 			System::events(),
// 			vec![
// 				record(Event::Collective(CollectiveEvent::Proposed(1, 0, hash, 3))),
// 				record(Event::Collective(CollectiveEvent::Voted(1, hash, true, 1, 0))),
// 				record(Event::Collective(CollectiveEvent::Voted(2, hash, false, 1, 1))),
// 				record(Event::Collective(CollectiveEvent::Closed(hash, 1, 1))),
// 				record(Event::Collective(CollectiveEvent::Disapproved(hash))),
// 			]
// 		);
// 	});
// }

// #[test]
// fn motions_approval_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let proposal_weight = proposal.get_dispatch_info().weight;
// 		let hash: H256 = proposal.blake2_256().into();
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			2,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, true));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 0, true));
// 		assert_ok!(Collective::close(Origin::signed(2), hash, 0, proposal_weight, proposal_len));

// 		assert_eq!(
// 			System::events(),
// 			vec![
// 				record(Event::Collective(CollectiveEvent::Proposed(1, 0, hash, 2))),
// 				record(Event::Collective(CollectiveEvent::Voted(1, hash, true, 1, 0))),
// 				record(Event::Collective(CollectiveEvent::Voted(2, hash, true, 2, 0))),
// 				record(Event::Collective(CollectiveEvent::Closed(hash, 2, 0))),
// 				record(Event::Collective(CollectiveEvent::Approved(hash))),
// 				record(Event::Collective(CollectiveEvent::Executed(
// 					hash,
// 					Err(DispatchError::BadOrigin)
// 				))),
// 			]
// 		);
// 	});
// }

// #[test]
// fn motion_with_no_votes_closes_with_disapproval() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let proposal_weight = proposal.get_dispatch_info().weight;
// 		let hash: H256 = proposal.blake2_256().into();
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			3,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		assert_eq!(
// 			System::events()[0],
// 			record(Event::Collective(CollectiveEvent::Proposed(1, 0, hash, 3)))
// 		);

// 		// Closing the motion too early is not possible because it has neither
// 		// an approving or disapproving simple majority due to the lack of votes.
// 		assert_noop!(
// 			Collective::close(Origin::signed(2), hash, 0, proposal_weight, proposal_len),
// 			Error::<Test, Instance1>::TooEarly
// 		);

// 		// Once the motion duration passes,
// 		let closing_block = System::block_number() + MotionDuration::get();
// 		System::set_block_number(closing_block);
// 		// we can successfully close the motion.
// 		assert_ok!(Collective::close(Origin::signed(2), hash, 0, proposal_weight, proposal_len));

// 		// Events show that the close ended in a disapproval.
// 		assert_eq!(
// 			System::events()[1],
// 			record(Event::Collective(CollectiveEvent::Closed(hash, 0, 3)))
// 		);
// 		assert_eq!(
// 			System::events()[2],
// 			record(Event::Collective(CollectiveEvent::Disapproved(hash)))
// 		);
// 	})
// }

// #[test]
// fn close_disapprove_does_not_care_about_weight_or_len() {
// 	// This test confirms that if you close a proposal that would be disapproved,
// 	// we do not care about the proposal length or proposal weight since it will
// 	// not be read from storage or executed.
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash: H256 = proposal.blake2_256().into();
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			2,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		// First we make the proposal succeed
// 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, true));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 0, true));
// 		// It will not close with bad weight/len information
// 		assert_noop!(
// 			Collective::close(Origin::signed(2), hash, 0, 0, 0),
// 			Error::<Test, Instance1>::WrongProposalLength,
// 		);
// 		assert_noop!(
// 			Collective::close(Origin::signed(2), hash, 0, 0, proposal_len),
// 			Error::<Test, Instance1>::WrongProposalWeight,
// 		);
// 		// Now we make the proposal fail
// 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, false));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 0, false));
// 		// It can close even if the weight/len information is bad
// 		assert_ok!(Collective::close(Origin::signed(2), hash, 0, 0, 0));
// 	})
// }

// #[test]
// fn disapprove_proposal_works() {
// 	new_test_ext().execute_with(|| {
// 		let proposal = make_proposal(42);
// 		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
// 		let hash: H256 = proposal.blake2_256().into();
// 		assert_ok!(Collective::propose(
// 			Origin::signed(1),
// 			2,
// 			Box::new(proposal.clone()),
// 			proposal_len
// 		));
// 		// Proposal would normally succeed
// 		assert_ok!(Collective::vote(Origin::signed(1), hash, 0, true));
// 		assert_ok!(Collective::vote(Origin::signed(2), hash, 0, true));
// 		// But Root can disapprove and remove it anyway
// 		assert_ok!(Collective::disapprove_proposal(Origin::root(), hash));
// 		assert_eq!(
// 			System::events(),
// 			vec![
// 				record(Event::Collective(CollectiveEvent::Proposed(1, 0, hash, 2))),
// 				record(Event::Collective(CollectiveEvent::Voted(1, hash, true, 1, 0))),
// 				record(Event::Collective(CollectiveEvent::Voted(2, hash, true, 2, 0))),
// 				record(Event::Collective(CollectiveEvent::Disapproved(hash))),
// 			]
// 		);
// 	})
// }

// #[test]
// #[should_panic(expected = "Members cannot contain duplicate accounts.")]
// fn genesis_build_panics_with_duplicate_members() {
// 	pallet_collective::GenesisConfig::<Test> {
// 		members: vec![1, 2, 3, 1],
// 		phantom: Default::default(),
// 	}
// 	.build_storage()
// 	.unwrap();
// }

// #[test]
// fn migration_v4() {
// 	new_test_ext().execute_with(|| {
// 		use frame_support::traits::PalletInfoAccess;

// 		let old_pallet = "OldCollective";
// 		let new_pallet = <Collective as PalletInfoAccess>::name();
// 		frame_support::storage::migration::move_pallet(
// 			new_pallet.as_bytes(),
// 			old_pallet.as_bytes(),
// 		);
// 		StorageVersion::new(0).put::<Collective>();

// 		crate::migrations::v4::pre_migrate::<Collective, _>(old_pallet);
// 		crate::migrations::v4::migrate::<Test, Collective, _>(old_pallet);
// 		crate::migrations::v4::post_migrate::<Collective, _>(old_pallet);

// 		let old_pallet = "OldCollectiveMajority";
// 		let new_pallet = <CollectiveMajority as PalletInfoAccess>::name();
// 		frame_support::storage::migration::move_pallet(
// 			new_pallet.as_bytes(),
// 			old_pallet.as_bytes(),
// 		);
// 		StorageVersion::new(0).put::<CollectiveMajority>();

// 		crate::migrations::v4::pre_migrate::<CollectiveMajority, _>(old_pallet);
// 		crate::migrations::v4::migrate::<Test, CollectiveMajority, _>(old_pallet);
// 		crate::migrations::v4::post_migrate::<CollectiveMajority, _>(old_pallet);

// 		let old_pallet = "OldDefaultCollective";
// 		let new_pallet = <DefaultCollective as PalletInfoAccess>::name();
// 		frame_support::storage::migration::move_pallet(
// 			new_pallet.as_bytes(),
// 			old_pallet.as_bytes(),
// 		);
// 		StorageVersion::new(0).put::<DefaultCollective>();

// 		crate::migrations::v4::pre_migrate::<DefaultCollective, _>(old_pallet);
// 		crate::migrations::v4::migrate::<Test, DefaultCollective, _>(old_pallet);
// 		crate::migrations::v4::post_migrate::<DefaultCollective, _>(old_pallet);
// 	});
// }