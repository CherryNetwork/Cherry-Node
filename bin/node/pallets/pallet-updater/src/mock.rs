#![cfg(test)]

use frame_support::parameter_types;
use frame_system as system;
use frame_system::{EventRecord, Phase};
use sp_core::H256;
use sp_io;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

use crate::{self as pallet_updater, Config};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub fn record(event: Event) -> EventRecord<Event, H256> {
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
