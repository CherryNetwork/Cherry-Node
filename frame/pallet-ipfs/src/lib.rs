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
	use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	///  Struct for holding IPFS information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Ipfs<T: Config> {
		pub cid_addr: Vec<u8>,
		pub owners: BTreeMap<AccountOf<T>, OwnershipLayer>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub enum OwnershipLayer {
		Owner,
		Editor,
		Reader,
	}

	impl Default for OwnershipLayer {
		fn default() -> Self {
			OwnershipLayer::Editor
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

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
		/// Account doesn't exists.
		AccNotExist,
		/// Handles checking that the IPFS is owned by the account.
		NotIpfsOwner,
		/// Handles checking that the IPFS is editable by the account.
		NotIpfsEditor,
		/// Handles checking that the IPFS is readable by the account.
		NotIpfsReader,
		/// Ensures that an account is different from the other.
		SameAccount,
		/// Ensures that an accounts onwership layer  is different.
		SameOwnershipLayer,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created(T::AccountId, T::Hash),
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		Transferred(T::AccountId, T::AccountId, T::Hash),
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
		AddOwner(T::AccountId, T::Hash, T::AccountId),
		RemoveOwner(T::AccountId, T::Hash),
		ChangeOwnershipLayer(T::AccountId, T::Hash, T::AccountId),
	}

	// Storage items.

	/// Keeps track of the number of IPFS Assets in existence.
	#[pallet::storage]
	#[pallet::getter(fn ipfs_cnt)]
	pub(super) type IpfsCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Stores a IPFS's unique traits, owner and price.
	#[pallet::storage]
	#[pallet::getter(fn ipfs_asset)]
	pub(super) type IpfsAsset<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Ipfs<T>>;

	/// Keeps track of what accounts own what IPFS.
	#[pallet::storage]
	#[pallet::getter(fn ipfs_asset_owned)]
	pub(super) type IpfsAssetOwned<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<T::Hash, T::MaxIpfsOwned>, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new unique IPFS.
		///
		/// The actual IPFS creation is done in the `mint()` function.
		#[pallet::weight(0)]
		pub fn create_ipfs(origin: OriginFor<T>, ci_address: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let ipfs_id = Self::mint(&sender, Vec::<u8>::new())?;

			log::info!(
				"A IPFS is born with ID: {:?} {:?}.",
				sp_std::str::from_utf8(&ci_address),
				ipfs_id
			);

			Self::deposit_event(Event::Created(sender, ipfs_id));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn remove_owner(
			origin: OriginFor<T>,
			ipfs_id: T::Hash,
			remove_acct: T::AccountId,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			ensure!(signer != remove_acct, <Error<T>>::SameAccount);
			ensure!(
				Self::determine_account_ownership_layer(&ipfs_id, &signer)?
					== OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			let mut ipfs = Self::ipfs_asset(&ipfs_id).ok_or(<Error<T>>::IpfsNotExist)?;
			ipfs.owners.remove(&remove_acct);

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

		#[pallet::weight(0)]
		pub fn add_owner(
			origin: OriginFor<T>,
			ipfs_id: T::Hash,
			add_acct: T::AccountId,
			ownership_layer: OwnershipLayer,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			ensure!(
				Self::determine_account_ownership_layer(&ipfs_id, &signer)?
					== OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			let mut ipfs = Self::ipfs_asset(&ipfs_id).ok_or(<Error<T>>::IpfsNotExist)?;

			ipfs.owners.insert(add_acct.clone(), ownership_layer.clone());

			<IpfsAsset<T>>::insert(&ipfs_id, ipfs);
			<IpfsAssetOwned<T>>::try_mutate(&add_acct, |ipfs_vec| ipfs_vec.try_push(ipfs_id))
				.map_err(|_| <Error<T>>::ExceedMaxIpfsOwned)?;

			Self::deposit_event(Event::AddOwner(signer, ipfs_id, add_acct));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn change_ownership(
			origin: OriginFor<T>,
			ipfs_id: T::Hash,
			acct_to_change: T::AccountId,
			ownership_layer: OwnershipLayer,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				Self::determine_account_ownership_layer(&ipfs_id, &signer)?
					== OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			let mut ipfs = Self::ipfs_asset(&ipfs_id).ok_or(<Error<T>>::IpfsNotExist)?;
			let ownership = Self::determine_account_ownership_layer(&ipfs_id, &acct_to_change)?;

			ensure!(ownership != ownership_layer, <Error<T>>::SameOwnershipLayer);

			ipfs.owners.insert(acct_to_change.clone(), ownership_layer);

			<IpfsAsset<T>>::insert(&ipfs_id, ipfs);
			<IpfsAssetOwned<T>>::try_mutate(&acct_to_change, |ipfs_vec| ipfs_vec.try_push(ipfs_id))
				.map_err(|_| <Error<T>>::ExceedMaxIpfsOwned)?;

			Self::deposit_event(Event::ChangeOwnershipLayer(signer, ipfs_id, acct_to_change));

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn mint(owner: &T::AccountId, cid: Vec<u8>) -> Result<T::Hash, Error<T>> {
			let mut ipfs = Ipfs::<T> {
				cid_addr: cid.clone(),
				owners: BTreeMap::<AccountOf<T>, OwnershipLayer>::new(),
			};

			ipfs.owners.insert(owner.clone(), OwnershipLayer::Owner);

			log::info!("{:?}", sp_std::str::from_utf8(&ipfs.cid_addr));

			let ipfs_id = T::Hashing::hash_of(&ipfs);
			let new_cnt = Self::ipfs_cnt().checked_add(1).ok_or(<Error<T>>::IpfsCntOverflow)?;

			<IpfsAssetOwned<T>>::try_mutate(&owner, |ipfs_vec| ipfs_vec.try_push(ipfs_id))
				.map_err(|_| <Error<T>>::ExceedMaxIpfsOwned)?;

			<IpfsAsset<T>>::insert(ipfs_id, ipfs);
			<IpfsCnt<T>>::put(new_cnt);

			Ok(ipfs_id)
		}

		pub fn determine_account_ownership_layer(
			ipfs_id: &T::Hash,
			acct: &T::AccountId,
		) -> Result<OwnershipLayer, Error<T>> {
			match Self::ipfs_asset(ipfs_id) {
				Some(ipfs) => {
					if let Some(layer) = ipfs.owners.get_key_value(acct) {
						Ok(layer.1.clone())
					} else {
						Err(<Error<T>>::AccNotExist)
					}
				}
				None => Err(<Error<T>>::IpfsNotExist),
			}
		}
	}
}
