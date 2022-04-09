use super::*;
use frame_support::{assert_noop, assert_ok, parameter_types};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use frame_system::{EventRecord, Phase};
use crate::{self as pallet_updater, Config};
use super::Event as UpdaterEvent;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

fn record(event: Event) -> EventRecord<Event, H256> {
	EventRecord { phase: Phase::Initialization, event, topics: vec![] }
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Call, Config, Storage, Event<T>},
		Updater: pallet_updater::{Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
}

parameter_types! {
	pub const MaxMemberCnt: u32 = 3;
	pub const MaxProposalCnt: u32 = 1;
	pub const UpdaterMotionDuration: u64 = 0;
}

impl pallet_updater::Config for Test {
	type Event = Event;
	type Call = Call;
	type MaxProposals = MaxProposalCnt;
	type MaxMembers = MaxMemberCnt;
	type MotionDuration = UpdaterMotionDuration;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = GenesisConfig {
		// We use default for brevity, but you can configure as desired if needed.
		system: Default::default(),
		updater: pallet_updater::GenesisConfig { phantom: Default::default(), members: vec![1u64] },
	}
	.build_storage()
	.unwrap();
	t.into()
}

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
		Updater::add_member(Origin::signed(1), 2);

		let members = Updater::members();

		assert_eq!(members.len(), 2usize);
		assert!(members.contains(&2));

		assert_noop!(Updater::add_member(Origin::signed(1), 2), Error::<Test>::SameAccount);

		assert_eq!(members.len(), 2usize);

		assert_noop!(Updater::add_member(Origin::signed(3), 4), Error::<Test>::NotMember);

		assert_eq!(members.len(), 2usize);
	})
}

#[test]
fn test_remove_member_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(Updater::remove_member(Origin::signed(1), 2), Error::<Test>::AccNotExist);

		assert_ok!(Updater::add_member(Origin::signed(1), 2));

		let members = Updater::members();

		assert_eq!(members.len(), 2usize);
		assert!(members.contains(&2));

		assert_ok!(Updater::remove_member(Origin::signed(2), 1));

		let members = Updater::members();

		assert_eq!(members.len(), 1usize);
		assert!(!members.contains(&1));

		assert_noop!(Updater::add_member(Origin::signed(1), 2), Error::<Test>::NotMember);

		assert_eq!(members.len(), 1usize);
	})
}

#[test]
fn test_propose_code_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(4);

		let code = vec![0u8, 0u8, 0u8];
		let hash = BlakeTwo256::hash_of(&code);

		assert_ok!(Updater::propose_code(Origin::signed(1), code.clone()));
		assert_noop!(
			Updater::propose_code(Origin::signed(1), code.clone()),
			Error::<Test>::DuplicateProposal
		);
		assert_noop!(
			Updater::propose_code(Origin::signed(6), code.clone()),
			Error::<Test>::NotMember
		);

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
fn test_vote_code_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Updater::add_member(Origin::signed(1), 2));

		let code = vec![0u8, 0u8, 0u8];
		let hash = BlakeTwo256::hash_of(&code);
		let hash_w = BlakeTwo256::hash_of(&vec![0u8]);

		assert_ok!(Updater::propose_code(Origin::signed(1), code.clone()));

		assert_ok!(Updater::vote_code(Origin::signed(1), hash.clone(), 0, true));

		let voting = Updater::voting(&hash).expect("Error with first voting call");

		assert_noop!(
			Updater::vote_code(Origin::signed(1), hash_w, 0, true),
			Error::<Test>::ProposalMissing
		);

		assert_eq!(voting.index, 0);

		assert_eq!(voting.ayes.len(), 1);

		assert!(voting.ayes.contains(&1));

		assert_noop!(
			Updater::vote_code(Origin::signed(1), hash.clone(), 1, true),
			Error::<Test>::WrongIndex
		);

		assert_ok!(Updater::vote_code(Origin::signed(1), hash.clone(), 0, false));

		let voting = Updater::voting(&hash).expect("Error with second voting call");

		assert_eq!(voting.nays.len(), 1);

		assert!(voting.nays.contains(&1));

		assert_noop!(
			Updater::vote_code(Origin::signed(5), hash.clone(), 1, true),
			Error::<Test>::NotMember
		);

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
fn test_close_vote_work() {
	new_test_ext().execute_with(|| {
		let code = vec![0u8, 0u8, 0u8];
		let hash = BlakeTwo256::hash_of(&code);

		assert_ok!(Updater::propose_code(Origin::signed(1), code.clone()));

		assert_noop!(
			Updater::close_vote(Origin::signed(4), hash.clone(), 0),
			Error::<Test>::NotMember
		);
		assert_noop!(
			Updater::close_vote(Origin::signed(1), hash.clone(), 1),
			Error::<Test>::WrongIndex
		);

		assert_ok!(Updater::close_vote(Origin::signed(1), hash.clone(), 0));
	})
}

/*
#[test]
fn test_event_add_member() {
    new_test_ext().execute_with(|| {
        let res = <pallet_updater::Pallet<Test>>::add_member(Origin::signed(1), 2).unwrap();

        assert_eq!(System::events(), vec![record(Event::Updater(UpdaterEvent::AddedUpdater(2)))]);
    });
}*/