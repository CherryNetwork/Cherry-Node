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
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Call: From<Call<Self>>;

		#[pallet::constant]
		type MaxUpdatersCnt: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn updater)]
	pub type Members<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn updater_cnt)]
	pub type MemberCnt<T> = StorageValue<_, u32, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		AddedUpdaters(T::AccountId),
		RemovedUpdaters(T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Ensures that an account is different from the other.
		SameAccount,
		/// User with the `AccountId` is not a member.
		AccNotExist,
		/// Signer is not a member.
		NotMember,
	}

	// Callables
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// Add a new member to storage
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn add_member(origin: OriginFor<T>, add_acct: T::AccountId) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			let mut updaters = Self::updater();
			ensure!(updaters.contains(&signer), Error::<T>::NotMember);
			ensure!(!updaters.contains(&add_acct), Error::<T>::SameAccount);

			updaters.push(add_acct.clone());
			<Members<T>>::put(updaters);

			Self::deposit_event(Event::AddedUpdaters(add_acct));

			Ok(())
		}

		// Remove a new member from storage
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn remove_member(origin: OriginFor<T>, remove_acct: T::AccountId) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(signer != remove_acct, <Error<T>>::SameAccount);

			let updaters = Self::updater();
			ensure!(updaters.contains(&remove_acct), <Error<T>>::AccNotExist);

			// https://docs.substrate.io/rustdocs/latest/sp_std/vec/struct.Vec.html#method.retain
			<Members<T>>::mutate(|v| v.retain(|x| *x != remove_acct));

			Self::deposit_event(Event::RemovedUpdaters(remove_acct));

			Ok(())
		}
	}
}
