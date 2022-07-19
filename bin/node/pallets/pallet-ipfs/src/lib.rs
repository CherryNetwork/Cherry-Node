#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod functions;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use codec::{Decode, Encode};
use frame_support::{
	traits::{EstimateNextSessionRotation, Get, ValidatorSet, ValidatorSetWithIdentification},
	RuntimeDebug, LOG_TARGET,
};
use frame_system::{
	self,
	offchain::{SendSignedTransaction, Signer},
};
use scale_info::TypeInfo;
use sp_core::{
	crypto::KeyTypeId,
	offchain::{Duration, OpaqueMultiaddr, StorageKind, Timestamp},
	Bytes,
};
use sp_io::offchain::timestamp;
use sp_runtime::traits::{Convert, Zero};
use sp_staking::offence::{Offence, OffenceError, ReportOffence};
use sp_std::{convert::TryInto, vec::Vec};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"chfs");

pub mod crypto {
	use crate::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};

	app_crypto!(sr25519, KEY_TYPE);

	pub struct AuthorityId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for AuthorityId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for AuthorityId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

pub use pallet::*;
pub use weights::WeightInfo;

#[derive(Encode, Decode, RuntimeDebug, PartialEq, TypeInfo)]
pub enum DataCommand<AccountId> {
	AddBytes(OpaqueMultiaddr, Vec<u8>, u64, u32, AccountId, bool),
	AddBytesRaw(OpaqueMultiaddr, Vec<u8>, AccountId, bool),
	CatBytes(OpaqueMultiaddr, Vec<u8>, AccountId),
	InsertPin(OpaqueMultiaddr, Vec<u8>, AccountId, bool),
	RemovePin(OpaqueMultiaddr, Vec<u8>, AccountId, bool),
	RemoveBlock(OpaqueMultiaddr, Vec<u8>, AccountId),
}

pub type EraIndex = u32;
pub type RewardPoint = u32;

/// Reward points for storage providers of some specific assest id during an era.
#[derive(PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct EraRewardPoints<AccountId> {
	/// the total number of points
	total: RewardPoint,
	/// the reward points for individual validators, sum(i.rewardPoint in individual) = total
	individual: sp_std::collections::btree_map::BTreeMap<AccountId, RewardPoint>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::DispatchResult,
		ensure,
		pallet_prelude::{InvalidTransaction, ValueQuery, *},
		traits::Currency,
		unsigned::{TransactionSource, TransactionValidity, ValidateUnsigned},
	};
	use frame_system::{
		offchain::{AppCrypto, CreateSignedTransaction},
		pallet_prelude::*,
	};
	use scale_info::TypeInfo;
	use sp_core::offchain::OpaqueMultiaddr;
	use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

	pub type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	///  Struct for holding IPFS information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Ipfs<T: Config> {
		pub cid: Vec<u8>,
		pub size: u64,
		pub gateway_url: Vec<u8>,
		pub owners: BTreeMap<AccountOf<T>, OwnershipLayer>,
		pub created_at: T::BlockNumber,
		pub deleting_at: T::BlockNumber,
		pub pinned: bool,
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
			OwnershipLayer::Owner
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn ipfs_nodes)]
	pub(super) type IPFSNodes<T: Config> =
		StorageMap<_, Twox64Concat, Vec<u8>, Vec<OpaqueMultiaddr>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn current_session)]
	pub(super) type CurrentEra<T: Config> = StorageValue<_, EraIndex>;

	#[pallet::storage]
	#[pallet::getter(fn active_session)]
	pub(super) type ActiveEra<T: Config> = StorageValue<_, EraIndex>;

	#[pallet::storage]
	#[pallet::getter(fn era_reward_points)]
	pub(super) type ErasRewardPoints<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		Vec<u8>,
		EraRewardPoints<T::AccountId>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub(super) type Validators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn approved_validators)]
	pub(super) type ApprovedValidators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn offline_validators)]
	pub type OfflineValidators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_session_rewards)]
	pub type SessionParticipation<T: Config> =
		StorageMap<_, Blake2_128Concat, EraIndex, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn unproductive_sessions)]
	pub type UnproductiveSessions<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn data_queue)]
	pub(super) type DataQueue<T: Config> =
		StorageValue<_, Vec<DataCommand<T::AccountId>>, ValueQuery>;

	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<Call<Self>> + frame_system::Config + pallet_session::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the IPFS pallet.
		type Currency: Currency<Self::AccountId>;

		/// The maximum amount of IPFS Assets a single account can own.
		#[pallet::constant]
		type MaxIpfsOwned: Get<u32>;

		#[pallet::constant]
		type MaxDeadSessions: Get<u32>;

		/// Default time that an IPFS asset will be stored online.
		#[pallet::constant]
		type DefaultAssetLifetime: Get<Self::BlockNumber>;

		type Call: From<Call<Self>>;

		type AddRemoveOrigin: EnsureOrigin<Self::Origin>;

		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// if let Call::submit_rpc_ready { .. } = call {
			// 	Self::validate_transaction_parameters()
			// } else if let Call::submit_ipfs_identity { .. } = call {
			// 	Self::validate_transaction_parameters()
			// } else {
			InvalidTransaction::Call.into()
			// }
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		Duplicate,
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
		/// Ensures that an IPFS is not already owned by the account.
		IpfsAlreadyOwned,
		/// Ensures that an IPFS is not already pinned.
		IpfsAlreadyPinned,
		/// Ensures that an IPFS is pinned.
		IpfsNotPinned,
		CantCreateRequest,
		RequestTimeout,
		RequestFailed,
		FeeOutOfBounds,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A request to add bytes was queued
		ValidatorAdditionInitiated(T::AccountId),
		QueuedDataToAdd(T::AccountId),
		QueuedDataToCat(T::AccountId),
		PublishedIdentity(T::AccountId),
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		Transferred(T::AccountId, T::AccountId, T::Hash),
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
		AddOwner(T::AccountId, Vec<u8>, T::AccountId),
		RemoveOwner(T::AccountId, Vec<u8>),
		ChangeOwnershipLayer(T::AccountId, Vec<u8>, T::AccountId),
		CreatedIpfsAsset(T::AccountId, Vec<u8>),
		WriteIpfsAsset(T::AccountId, T::Hash),
		ReadIpfsAsset(T::AccountId, T::Hash),
		AddPin(T::AccountId, Vec<u8>),
		DeleteIpfsAsset(T::AccountId, Vec<u8>),
		UnpinIpfsAsset(T::AccountId, Vec<u8>),
	}

	// Storage items.

	/// Keeps track of the number of IPFS Assets in existence.
	#[pallet::storage]
	#[pallet::getter(fn ipfs_cnt)]
	pub(super) type IpfsCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Stores a IPFS's unique traits, owner and price.
	#[pallet::storage]
	#[pallet::getter(fn ipfs_asset)]
	pub(super) type IpfsAsset<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, Ipfs<T>>;

	/// Keeps track of what accounts own what IPFS.
	#[pallet::storage]
	#[pallet::getter(fn ipfs_asset_owned)]
	pub(super) type IpfsAssetOwned<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<Vec<u8>, T::MaxIpfsOwned>, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_no: BlockNumberFor<T>) -> Weight {
			if block_no % 2u32.into() == 1u32.into() {
				<DataQueue<T>>::kill(); // Research this - @charmitro
			}
			0
		}

		fn offchain_worker(block_no: BlockNumberFor<T>) {
			// handle data requests each block
			if let Err(e) = Self::handle_data_requests() {
				log::error!("IPFS: Encountered an error while processing data requests: {:?}", e);
			}

			if block_no % 5u32.into() == 0u32.into() {
				if let Err(e) = Self::print_metadata() {
					log::error!("IPFS: Encountered an error while obtaining metadata: {:?}", e);
				}

				if let Err(e) = Self::ipfs_nodes_housekeeping() {
					log::error!("IPFS: Encountered an error while obtaining metadata: {:?}", e);
				}
			}

			if let Err(e) = Self::ipfs_garbage_collector(block_no) {
				log::error!("IPFS::GARBAGE_COLLECTOR::ERROR: {:?}", e);
			}
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_validators: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_validators: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_validators(&self.initial_validators);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100)]
		pub fn add_validator(origin: OriginFor<T>, validator_id: T::AccountId) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;
			Self::do_add_validator(validator_id.clone())?;
			Self::approve_validator(validator_id)?;

			Ok(())
		}

		/// Create a new unique IPFS.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_ipfs_asset())]
		pub fn create_ipfs_asset(
			origin: OriginFor<T>,
			addr: Vec<u8>,
			cid: Vec<u8>,
			size: u64,
			price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				!<IpfsAssetOwned<T>>::get(&sender).contains(&cid),
				<Error<T>>::IpfsAlreadyOwned
			);

			if let Some(value) = price {
				if let Some(price_converted) = TryInto::<u32>::try_into(value).ok() {
					let extra_lifetime = 100 * (price_converted / 1000);
					let multiaddr = OpaqueMultiaddr(addr.clone());
					<DataQueue<T>>::mutate(|queue| {
						queue.push(DataCommand::AddBytes(
							multiaddr,
							cid,
							size,
							extra_lifetime,
							sender.clone(),
							true,
						))
					});
				}
			} else {
				let multiaddr = OpaqueMultiaddr(addr.clone());
				<DataQueue<T>>::mutate(|queue| {
					queue.push(DataCommand::AddBytes(multiaddr, cid, size, 0, sender.clone(), true))
				});
			}

			Self::deposit_event(Event::QueuedDataToAdd(sender.clone()));

			Ok(())
		}

		/// Create a new unique IPFS.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_ipfs_asset())]
		pub fn create_ipfs_asset_raw(
			origin: OriginFor<T>,
			addr: Vec<u8>,
			data: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				!<IpfsAssetOwned<T>>::get(&sender).contains(&data),
				<Error<T>>::IpfsAlreadyOwned
			);

			let multiaddr = OpaqueMultiaddr(addr);

			<DataQueue<T>>::mutate(|queue| {
				queue.push(DataCommand::AddBytesRaw(multiaddr, data.clone(), sender.clone(), true))
			});

			Self::deposit_event(Event::QueuedDataToAdd(sender.clone()));

			Ok(())
		}

		/// Extends the duration of an Ipfs asset
		#[pallet::weight(0)]
		pub fn extend_duration(
			origin: OriginFor<T>,
			cid: Vec<u8>,
			fee: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				Self::determine_account_ownership_layer(&cid, &sender)? == OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			ensure!(fee >= 1000u32.into(), <Error<T>>::FeeOutOfBounds);

			if let Some(value) = TryInto::<u32>::try_into(fee).ok() {
				let extra_duration = 100 * (value / 1000);
				let mut ipfs_asset = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;
				ipfs_asset.deleting_at += extra_duration.into();
				<IpfsAsset<T>>::insert(cid.clone(), ipfs_asset);
			}

			Ok(())
		}

		/// Pins an IPFS.
		#[pallet::weight(0)]
		pub fn pin_ipfs_asset(origin: OriginFor<T>, addr: Vec<u8>, cid: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				Self::determine_account_ownership_layer(&cid, &sender)? == OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			let multiaddr = OpaqueMultiaddr(addr);
			let ipfs_asset = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;

			ensure!(ipfs_asset.pinned != true, <Error<T>>::IpfsAlreadyPinned);

			<DataQueue<T>>::mutate(|queue| {
				queue.push(DataCommand::InsertPin(multiaddr, cid.clone(), sender.clone(), true))
			});

			Self::deposit_event(Event::AddPin(sender.clone(), cid.clone()));

			Ok(())
		}

		/// Unpins an IPFS.
		#[pallet::weight(0)]
		pub fn unpin_ipfs_asset(
			origin: OriginFor<T>,
			addr: Vec<u8>,
			cid: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				Self::determine_account_ownership_layer(&cid, &sender)? == OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			let multiaddr = OpaqueMultiaddr(addr);
			let ipfs_asset = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;

			ensure!(ipfs_asset.pinned == true, <Error<T>>::IpfsNotPinned);

			<DataQueue<T>>::mutate(|queue| {
				queue.push(DataCommand::RemovePin(multiaddr, cid.clone(), sender.clone(), true))
			});

			Self::deposit_event(Event::UnpinIpfsAsset(sender.clone(), cid.clone()));

			Ok(())
		}

		/// Deletes an IPFS.
		#[pallet::weight(0)]
		pub fn delete_ipfs_asset(
			origin: OriginFor<T>,
			addr: Vec<u8>,
			cid: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				Self::determine_account_ownership_layer(&cid, &sender)? == OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			let multiaddr = OpaqueMultiaddr(addr);
			let ipfs_asset = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;

			if ipfs_asset.pinned == true {
				<DataQueue<T>>::mutate(|queue| {
					queue.push(DataCommand::RemovePin(
						multiaddr.clone(),
						cid.clone(),
						sender.clone(),
						true,
					))
				});
			}

			<DataQueue<T>>::mutate(|queue| {
				queue.push(DataCommand::RemoveBlock(multiaddr, cid.clone(), sender.clone()))
			});

			Self::deposit_event(Event::DeleteIpfsAsset(sender.clone(), cid.clone()));

			Ok(())
		}

		/// TODO: Read an IPFS asset.
		#[pallet::weight(0)]
		pub fn get_ipfs_asset(origin: OriginFor<T>, addr: Vec<u8>, cid: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				Self::determine_account_ownership_layer(&cid, &sender)? == OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			let multiaddr = OpaqueMultiaddr(addr);

			<DataQueue<T>>::mutate(|queue| {
				queue.push(DataCommand::CatBytes(multiaddr, cid.clone(), sender.clone()))
			});

			Self::deposit_event(Event::QueuedDataToCat(sender.clone()));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_ipfs_identity(
			origin: OriginFor<T>,
			public_key: Vec<u8>,
			multiaddress: Vec<OpaqueMultiaddr>,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			<IPFSNodes<T>>::insert(public_key.clone(), multiaddress.clone());

			Self::deposit_event(Event::PublishedIdentity(signer.clone()));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_ipfs_add_results(
			origin: OriginFor<T>,
			admin: AccountOf<T>,
			cid: Vec<u8>,
			size: u64,
			extra_lifetime: u32,
		) -> DispatchResult {
			ensure_signed(origin)?;

			<DataQueue<T>>::take();

			let current_block = <frame_system::Pallet<T>>::block_number();
			let asset_lifetime = current_block + T::DefaultAssetLifetime::get();
			let mut gateway_url = "http://15.188.14.75:8080/ipfs/".as_bytes().to_vec();
			gateway_url.append(&mut cid.clone());

			let mut ipfs = Ipfs::<T> {
				cid: cid.clone(),
				size,
				gateway_url,
				owners: BTreeMap::<AccountOf<T>, OwnershipLayer>::new(),
				created_at: current_block,
				deleting_at: asset_lifetime + extra_lifetime.into(),
				pinned: true, // true by default.
			};

			ipfs.owners.insert(admin.clone(), OwnershipLayer::default());

			let new_cnt = Self::ipfs_cnt().checked_add(1).ok_or(<Error<T>>::IpfsCntOverflow)?;

			<IpfsAssetOwned<T>>::try_mutate(&admin, |ipfs_vec| ipfs_vec.try_push(cid.clone()))
				.map_err(|_| <Error<T>>::ExceedMaxIpfsOwned)?;
			<IpfsAsset<T>>::insert(cid.clone(), ipfs);
			<IpfsCnt<T>>::put(new_cnt);

			Self::deposit_event(Event::CreatedIpfsAsset(admin.clone(), cid.clone()));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_ipfs_pin_results(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
			ensure_signed(origin)?;

			let mut ipfs_asset = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;

			ipfs_asset.pinned = true;
			<IpfsAsset<T>>::insert(cid.clone(), ipfs_asset);

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_ipfs_unpin_results(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
			ensure_signed(origin)?;

			let mut ipfs_asset = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;

			ipfs_asset.pinned = false;
			<IpfsAsset<T>>::insert(cid.clone(), ipfs_asset);

			<DataQueue<T>>::take();

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_ipfs_delete_results(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			<DataQueue<T>>::take();

			let mut ipfs_asset = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;
			for user in ipfs_asset.owners.iter_mut() {
				<IpfsAssetOwned<T>>::try_mutate(&user.0, |ipfs_vec| {
					if let Some(index) = ipfs_vec.iter().position(|i| *i == cid.clone()) {
						ipfs_vec.swap_remove(index);
						Ok(true)
					} else {
						Ok(false)
					}
				})
				.map_err(|_: bool| <Error<T>>::ExceedMaxIpfsOwned)?;
			}

			let new_cnt = Self::ipfs_cnt().checked_sub(1).unwrap();

			<IpfsAsset<T>>::remove(cid.clone());
			<IpfsCnt<T>>::put(new_cnt);

			Self::deposit_event(Event::DeleteIpfsAsset(signer.clone(), cid.clone()));

			Ok(())
		}

		/// Give ownership of an asset to user.
		#[pallet::weight(0)]
		pub fn add_owner(
			origin: OriginFor<T>,
			cid: Vec<u8>,
			add_acct: T::AccountId,
			ownership_layer: OwnershipLayer,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			ensure!(
				Self::determine_account_ownership_layer(&cid, &signer)? == OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			ensure!(
				!<IpfsAssetOwned<T>>::get(&add_acct).contains(&cid),
				<Error<T>>::IpfsAlreadyOwned
			);

			let mut ipfs = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;
			ipfs.owners.insert(add_acct.clone(), ownership_layer.clone());

			<IpfsAsset<T>>::insert(&cid, ipfs);
			<IpfsAssetOwned<T>>::try_mutate(&add_acct, |ipfs_vec| ipfs_vec.try_push(cid.clone()))
				.map_err(|_| <Error<T>>::ExceedMaxIpfsOwned)?;

			Self::deposit_event(Event::AddOwner(signer, cid, add_acct));

			Ok(())
		}

		/// Remove the ownership layer of a user.
		#[pallet::weight(0)]
		pub fn remove_ownership(
			origin: OriginFor<T>,
			cid: Vec<u8>,
			remove_acct: T::AccountId,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			ensure!(signer != remove_acct, <Error<T>>::SameAccount);
			ensure!(
				Self::determine_account_ownership_layer(&cid, &signer)? == OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			let mut ipfs = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;
			ensure!(ipfs.owners.contains_key(&remove_acct), <Error<T>>::AccNotExist);

			ipfs.owners.remove(&remove_acct);

			<IpfsAsset<T>>::insert(&cid, ipfs);
			<IpfsAssetOwned<T>>::try_mutate(&remove_acct, |ipfs_vec| {
				if let Some(index) = ipfs_vec.iter().position(|i| *i == cid) {
					ipfs_vec.swap_remove(index);
					Ok(true)
				} else {
					Ok(false)
				}
			})
			.map_err(|_: bool| <Error<T>>::ExceedMaxIpfsOwned)?;

			Self::deposit_event(Event::RemoveOwner(remove_acct, cid));

			Ok(())
		}

		/// Change the ownership layer of a user
		#[pallet::weight(0)]
		pub fn change_ownership(
			origin: OriginFor<T>,
			cid: Vec<u8>,
			acct_to_change: T::AccountId,
			ownership_layer: OwnershipLayer,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				Self::determine_account_ownership_layer(&cid, &signer)? == OwnershipLayer::Owner,
				<Error<T>>::NotIpfsOwner
			);

			let mut ipfs = Self::ipfs_asset(&cid).ok_or(<Error<T>>::IpfsNotExist)?;
			let ownership = Self::determine_account_ownership_layer(&cid, &acct_to_change)?;

			ensure!(ownership != ownership_layer, <Error<T>>::SameOwnershipLayer);

			ipfs.owners.insert(acct_to_change.clone(), ownership_layer);

			<IpfsAsset<T>>::insert(&cid, ipfs);
			<IpfsAssetOwned<T>>::try_mutate(&acct_to_change, |ipfs_vec| {
				ipfs_vec.try_push(cid.clone())
			})
			.map_err(|_| <Error<T>>::ExceedMaxIpfsOwned)?;

			Self::deposit_event(Event::ChangeOwnershipLayer(signer, cid, acct_to_change));

			Ok(())
		}
	}
}

impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
	// Plan a new session and provide new validator set.
	fn new_session(new_index: u32) -> Option<Vec<T::AccountId>> {
		// TODO(elsuizo: 2022-05-03): this could be the next step
		// Remove any offline validators. This will only work when the runtime
		// also has the im-online pallet.
		// Self::remove_offline_validators();

		log::info!("Starting session with index: {:?}", new_index);

		CurrentEra::<T>::mutate(|s| *s = Some(new_index));
		Self::remove_offline_validators();

		// TODO(charmitro <2022-05-19 Thu>): Select new candidates;
		log::debug!(target: LOG_TARGET, "New session called; updated validator set provided.");

		Some(Self::validators())
	}

	fn end_session(end_index: u32) {
		log::info!("Ending session with index: {:?}", end_index);
		// TODO(charmitro: <2022-05-19 Thu>): Self::mark_dead_validators();
	}

	fn start_session(start_index: u32) {
		log::info!("Starting session with index: {:?}", start_index);
		ActiveEra::<T>::mutate(|s| *s = Some(start_index));
	}
}

impl<T: Config> EstimateNextSessionRotation<T::BlockNumber> for Pallet<T> {
	fn average_session_length() -> T::BlockNumber {
		Zero::zero()
	}

	fn estimate_current_session_progress(
		_now: T::BlockNumber,
	) -> (Option<sp_runtime::Permill>, frame_support::dispatch::Weight) {
		(None, Zero::zero())
	}

	fn estimate_next_session_rotation(
		_now: T::BlockNumber,
	) -> (Option<T::BlockNumber>, frame_support::dispatch::Weight) {
		(None, Zero::zero())
	}
}

pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Convert<T::ValidatorId, Option<T::ValidatorId>> for ValidatorOf<T> {
	fn convert(account: T::ValidatorId) -> Option<T::ValidatorId> {
		Some(account)
	}
}

impl<T: Config> ValidatorSet<T::AccountId> for Pallet<T> {
	type ValidatorId = T::ValidatorId;
	type ValidatorIdOf = T::ValidatorIdOf;

	fn session_index() -> sp_staking::SessionIndex {
		pallet_session::Pallet::<T>::current_index()
	}

	fn validators() -> Vec<Self::ValidatorId> {
		pallet_session::Pallet::<T>::validators()
	}
}

impl<T: Config> ValidatorSetWithIdentification<T::AccountId> for Pallet<T> {
	type Identification = T::ValidatorId;
	type IdentificationOf = ValidatorOf<T>;
}

impl<T: Config, O: Offence<(T::AccountId, T::AccountId)>>
	ReportOffence<T::AccountId, (T::AccountId, T::AccountId), O> for Pallet<T>
{
	fn report_offence(_reporters: Vec<T::AccountId>, offence: O) -> Result<(), OffenceError> {
		let offenders = offence.offenders();

		for (v, _) in offenders.into_iter() {
			Self::mark_for_removal(v);
		}

		Ok(())
	}

	fn is_known_offence(
		_offenders: &[(T::AccountId, T::AccountId)],
		_time_slot: &O::TimeSlot,
	) -> bool {
		false
	}
}
