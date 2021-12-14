#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		ensure,
		pallet_prelude::*,
		sp_runtime::traits::Hash,
		traits::{Currency, Randomness},
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_std::vec::Vec;

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Struct for holding IPFS information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Ipfs<T: Config> {
		pub cid_addr: Vec<u8>,
		pub owners: Vec<AccountOf<T>>,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types it depends on.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the IPFS pallet.
		type Currency: Currency<Self::AccountId>;

		/// The maximum amount of IPFS Assets a single account can own.
		#[pallet::constant]
		type MaxIpfsOwned: Get<u32>;

		/// The type of Randomness we want to specify for this pallet.
		type IpfsRandomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	// Errors.
	#[pallet::error]
	pub enum Error<T> {
		/// Handles arithmetic overflow when incrementing the IPFS counter.
		IpfsCntOverflow,
		/// An account cannot own more IPFS Assets than `MaxIPFSCount`.
		ExceedMaxIpfsOwned,
		/// Buyer cannot be the owner.
		BuyerIsIpfsOwner,
		/// Cannot transfer a IPFS to its owner.
		TransferToSelf,
		/// Handles checking whether the IPFS exists.
		IpfsNotExist,
		/// Handles checking that the IPFS is owned by the account transferring, buying or setting a price for it.
		NotIpfsOwner,
		/// Ensures that an account is different from the other.
		SameAccount,
	}

	// Events.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created(T::AccountId, T::Hash),
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		Transferred(T::AccountId, T::AccountId, T::Hash),
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
		AddOwner(T::AccountId, T::Hash, T::AccountId),
		RemoveOwner(T::AccountId, T::Hash),
	}

	// Storage items.

	#[pallet::storage]
	#[pallet::getter(fn ipfs_cnt)]
	/// Keeps track of the number of IPFS Assets in existence.
	pub(super) type IpfsCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ipfs_asset)]
	/// Stores a IPFS's unique traits, owner and price.
	pub(super) type IpfsAsset<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Ipfs<T>>;

	#[pallet::storage]
	#[pallet::getter(fn ipfs_asset_owned)]
	/// Keeps track of what accounts own what IPFS.
	pub(super) type IpfsAssetOwned<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<T::Hash, T::MaxIpfsOwned>, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new unique IPFS.
		///
		/// The actual IPFS creation is done in the `mint()` function.
		#[pallet::weight(100)]
		pub fn create_ipfs(origin: OriginFor<T>, ci_address: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let ipfs_id = Self::mint(&sender, Vec::<u8>::new())?;

			log::info!("A IPFS is born with ID: {:?} {:?}.", sp_std::str::from_utf8(&ci_address), ipfs_id);

			Self::deposit_event(Event::Created(sender, ipfs_id));

			Ok(())
		}

		#[pallet::weight(10)]
		pub fn remove_owner(
			origin: OriginFor<T>,
			ipfs_id: T::Hash,
			remove_acct: T::AccountId,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			ensure!(signer != remove_acct, <Error<T>>::SameAccount);
			ensure!(Self::is_ipfs_owner(&ipfs_id, &signer)?, <Error<T>>::NotIpfsOwner);

			let mut ipfs = Self::ipfs_asset(&ipfs_id).ok_or(<Error<T>>::IpfsNotExist)?;

			// Find user and index it from the owners list
			let index = ipfs.owners.iter().position(|user| *user == remove_acct.clone()).unwrap();
			ipfs.owners.swap_remove(index);

			<IpfsAsset<T>>::insert(&ipfs_id, ipfs);
			<IpfsAssetOwned<T>>::try_mutate(&remove_acct, |ipfs_vec| {
				if let Some(index) = ipfs_vec.iter().position(|i| *i == ipfs_id) {
					ipfs_vec.swap_remove(index);
					log::info!("peos\n\n");
					Ok(true)
				} else {
					Ok(false)
				}
			})
				.map_err(|_: bool| <Error<T>>::ExceedMaxIpfsOwned)?;

			Self::deposit_event(Event::RemoveOwner(remove_acct, ipfs_id));

			Ok(())
		}

		#[pallet::weight(10)]
		pub fn add_owner(
			origin: OriginFor<T>,
			ipfs_id: T::Hash,
			add_acct: T::AccountId,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(Self::is_ipfs_owner(&ipfs_id, &signer)?, <Error<T>>::NotIpfsOwner);

			let mut ipfs = Self::ipfs_asset(&ipfs_id).ok_or(<Error<T>>::IpfsNotExist)?;

			ipfs.owners.push(add_acct.clone());

			<IpfsAsset<T>>::insert(&ipfs_id, ipfs);
			<IpfsAssetOwned<T>>::try_mutate(&add_acct, |ipfs_vec| ipfs_vec.try_push(ipfs_id))
				.map_err(|_| <Error<T>>::ExceedMaxIpfsOwned)?;

			Self::deposit_event(Event::AddOwner(signer, ipfs_id, add_acct));

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn mint(owner: &T::AccountId, cid: Vec<u8>) -> Result<T::Hash, Error<T>> {
			let mut ipfs = Ipfs::<T> { cid_addr: cid, owners: Vec::<AccountOf<T>>::new() };

			ipfs.owners.push(owner.clone());

			log::info!("{:?}", sp_std::str::from_utf8(&ipfs.cid_addr));

			let ipfs_id = T::Hashing::hash_of(&ipfs);

			let new_cnt = Self::ipfs_cnt().checked_add(1).ok_or(<Error<T>>::IpfsCntOverflow)?;

			<IpfsAssetOwned<T>>::try_mutate(&owner, |ipfs_vec| ipfs_vec.try_push(ipfs_id))
				.map_err(|_| <Error<T>>::ExceedMaxIpfsOwned)?;

			<IpfsAsset<T>>::insert(ipfs_id, ipfs);
			<IpfsCnt<T>>::put(new_cnt);

			Ok(ipfs_id)
		}

		pub fn push_owner(ipfs_id: &T::Hash, acct: &T::AccountId) -> Result<T::Hash, Error<T>> {
			match Self::ipfs_asset(ipfs_id) {
				Some(mut ipfs) => {
					ipfs.owners.push(acct.clone());
					Ok(*ipfs_id)
				}
				None => Err(<Error<T>>::IpfsNotExist),
			}
		}

		// Helper to check correct IPFS owner
		pub fn is_ipfs_owner(ipfs_id: &T::Hash, acct: &T::AccountId) -> Result<bool, Error<T>> {
			match Self::ipfs_asset(ipfs_id) {
				Some(ipfs) => {
					if ipfs.owners.iter().any(|i| *i == *acct) {
						Ok(true)
					} else {
						Ok(false)
					}
				}
				None => Err(<Error<T>>::IpfsNotExist),
			}
		}
	}
}
