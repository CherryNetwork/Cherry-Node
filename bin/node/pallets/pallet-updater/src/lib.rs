#![cfg_attr(not(feature = "std"), no_std)]

/*
	Memberships pallet that members are capable of doing RuntimeUpgrades.
	Notes: Runtime Upgrades are only succesful if the extrinsic is submitted with
	`unchecked_weight`,  that means:
	```rust
		//                REQUIRED
		#[pallet::weight((*_weight, call.get_dispatch_info().class))]
		pub fn <call_name>_unchecked_weight(
			origin: OriginFor<T>,
			call: Box<<T as Config>::Call>,
			_weight: Weight, // REQUIRED
		) -> DispatchResultWithPostInfo {
			. . .
		}
	```

	`unchecked_weight` is needed because `set_code` will always exhaust block limits,
	that way we bypass the limit with not actually telling the node what the limit is.
*/
use scale_info::TypeInfo;
use sp_runtime::{traits::Hash, RuntimeDebug};

use frame_support::codec::{Decode, Encode};

pub use pallet::*;
use sp_std::vec::Vec;

/// Simple index type for proposal counting.
pub type ProposalIndex = u32;

/// A number of members.
pub type MemberCount = u32;

/// Info for keeping track of a motion being voted on.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Votes<AccountId, BlockNumber> {
	/// Proposal's unique index.
	index: ProposalIndex,
	/// Number of approval votes that are needed to pass the proposal.
	threshold: MemberCount,
	/// Current set of voters that approved it.
	ayes: Vec<AccountId>,
	/// Current set of voters that rejected it.
	nays: Vec<AccountId>,
	/// The hard end time of this vote.
	end: BlockNumber,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::{DispatchResult, GetDispatchInfo, UnfilteredDispatchable},
		pallet_prelude::*,
		traits::InitializeMembers,
		Parameter,
	};
	use frame_system::pallet_prelude::{OriginFor, *};
	use sp_std::boxed::Box;
	use sp_std::vec::Vec;
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		type Call: Parameter + UnfilteredDispatchable<Origin = Self::Origin> + GetDispatchInfo;

		#[pallet::constant]
		/// Maximum number of members.
		type MaxMembers: Get<u32>;
		/// Maximum number of proposals allowed to be active at the same time.
		type MaxProposals: Get<ProposalIndex>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub phantom: PhantomData<I>,
		pub members: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self { phantom: Default::default(), members: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			use sp_std::collections::btree_set::BTreeSet;
			let members_set: BTreeSet<_> = self.members.iter().collect();

			assert_eq!(members_set.len(), self.members.len(), "Members must be unique");

			Pallet::<T, I>::initialize_members(&self.members)
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[pallet::storage]
	#[pallet::getter(fn updater)]
	pub type Members<T: Config<I>, I: 'static = ()> =
		StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type Proposals<T: Config<I>, I: 'static = ()> = StorageValue<_, Vec<T::Hash>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn proposal_count)]
	pub type ProposalCount<T: Config<I>, I: 'static = ()> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn codes)]
	pub type Codes<T: Config<I>, I: 'static = ()> = StorageValue<_, Vec<u8>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn voting)]
	pub type Voting<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Identity, T::Hash, Votes<T::AccountId, T::BlockNumber>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// User added.
		AddedUpdater(T::AccountId),
		/// User removed.
		RemovedUpdater(T::AccountId),
		/// A motion (given hash) has been proposed (by given account).
		ProposedCode(T::AccountId, ProposalIndex, T::Hash),
		/// A motion (given hash) has been voted on by given account.
		/// [account, proposal_hash, voted, yes, no]
		VotedCode(T::AccountId, T::Hash, bool, MemberCount, MemberCount),
		/// A motion was approved.
		Approved(T::Hash, MemberCount, MemberCount),
		/// A motion was disapproved.
		Disapproved(T::Hash, MemberCount, MemberCount),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Ensures that an account is different from the other.
		SameAccount,
		/// User with the `AccountId` is not a member.
		AccNotExist,
		/// Signer is not a member.
		NotMember,
		/// Duplicate proposal not allowed.
		DuplicateProposal,
		/// Proposal must exist.
		ProposalMissing,
		/// Mismatched index.
		WrongIndex,
		/// Duplicate vote ignored.
		DuplicateVote,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Add a new member to storage.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn add_member(origin: OriginFor<T>, add_acct: T::AccountId) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			let mut updaters = Self::updater();
			ensure!(updaters.contains(&signer), Error::<T, I>::NotMember);
			ensure!(!updaters.contains(&add_acct), Error::<T, I>::SameAccount);

			updaters.push(add_acct.clone());
			<Members<T, I>>::put(updaters);

			Self::deposit_event(Event::AddedUpdater(add_acct));

			Ok(())
		}

		/// Remove a member from storage.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn remove_member(origin: OriginFor<T>, remove_acct: T::AccountId) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(signer != remove_acct, <Error<T, I>>::SameAccount);

			let updaters = Self::updater();
			ensure!(updaters.contains(&remove_acct), <Error<T, I>>::AccNotExist);

			<Members<T, I>>::mutate(|v| v.retain(|x| x != &remove_acct));

			Self::deposit_event(Event::RemovedUpdater(remove_acct));

			Ok(())
		}

		/// Adds a proposal to be voted on.
		///
		/// Requires the sender to be a member.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn propose_code(origin: OriginFor<T>, code: Vec<u8>) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let updaters = Self::updater();
			ensure!(updaters.contains(&signer), Error::<T, I>::NotMember);

			let proposal_hash = T::Hashing::hash_of(&code);
			let mut proposals = Self::proposals();
			ensure!(!proposals.contains(&proposal_hash), <Error<T, I>>::DuplicateProposal);

			proposals.push(proposal_hash);
			<Proposals<T, I>>::put(proposals);
			<ProposalCount<T, I>>::mutate(|i| *i += 1);
			<Codes<T, I>>::put(code);

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn vote_code(
			origin: OriginFor<T>,
			proposal: T::Hash,
			#[pallet::compact] index: ProposalIndex,
			approve: bool,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let updaters = Self::updater();
			ensure!(updaters.contains(&signer), Error::<T, I>::NotMember);

			let mut voting = Self::voting(&proposal).ok_or(Error::<T, I>::ProposalMissing)?;
			ensure!(voting.index == index, Error::<T, I>::WrongIndex);

			let position_yes = voting.ayes.iter().position(|a| a == &signer);
			let position_no = voting.nays.iter().position(|a| a == &signer);

			if approve {
				if position_yes.is_none() {
					voting.ayes.push(signer.clone());
				} else {
					return Err(Error::<T, I>::DuplicateVote.into());
				}
				if let Some(pos) = position_no {
					voting.nays.swap_remove(pos);
				}
			} else {
				if position_no.is_none() {
					voting.nays.push(signer.clone());
				} else {
					return Err(Error::<T, I>::DuplicateVote.into());
				}
				if let Some(pos) = position_yes {
					voting.ayes.swap_remove(pos);
				}
			}

			let yes_votes = voting.ayes.len() as MemberCount;
			let no_votes = voting.nays.len() as MemberCount;
			Self::deposit_event(Event::VotedCode(signer, proposal, approve, yes_votes, no_votes));

			Voting::<T, I>::insert(&proposal, voting);

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn close_vote(
			origin: OriginFor<T>,
			proposal_hash: T::Hash,
			#[pallet::compact] index: ProposalIndex,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let updaters = Self::updater();
			ensure!(updaters.contains(&signer), Error::<T, I>::NotMember);

			let voting = Self::voting(&proposal_hash).ok_or(Error::<T, I>::ProposalMissing)?;
			ensure!(voting.index == index, Error::<T, I>::WrongIndex);

			let yes_votes = voting.ayes.len() as MemberCount;
			let no_votes = voting.nays.len() as MemberCount;
			let seats = updaters.len() as MemberCount;
			let approved = yes_votes >= voting.threshold;
			let disapproved = seats.saturating_sub(no_votes) < voting.threshold;

			if approved {
				Self::deposit_event(Event::Approved(proposal_hash, yes_votes, no_votes));
			} else if disapproved {
				Self::deposit_event(Event::Disapproved(proposal_hash, yes_votes, no_votes));
			}

			// can probably move those into another function. A 'remove_proposal' function @zycon91
			Codes::<T, I>::kill();
			Voting::<T, I>::remove(proposal_hash);
			Proposals::<T, I>::kill();
			ProposalCount::<T, I>::kill();

			Ok(())
		}

		#[pallet::weight((*_weight, call.get_dispatch_info().class))]
		pub fn updater_unchecked_weight(
			origin: OriginFor<T>,
			call: Box<<T as Config<I>>::Call>,
			_weight: Weight,
		) -> DispatchResultWithPostInfo {
			log::info!("\n\n\n test call \n\n\n");

			Ok(Pays::Yes.into())
		}
	}

	impl<T: Config<I>, I: 'static> InitializeMembers<T::AccountId> for Pallet<T, I> {
		fn initialize_members(members: &[T::AccountId]) {
			if !members.is_empty() {
				assert!(<Members<T, I>>::get().is_empty(), "Members already initialized");
				<Members<T, I>>::put(members);
			}
		}
	}
}