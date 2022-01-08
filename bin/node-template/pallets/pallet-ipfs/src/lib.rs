#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use frame_system::offchain::{SendSignedTransaction, Signer};
use scale_info::TypeInfo;
use sp_core::crypto::KeyTypeId;
use sp_core::offchain::{Duration, OpaqueMultiaddr, Timestamp};
use sp_io::offchain::timestamp;
use sp_std::vec::Vec;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"chfs");

pub mod crypto {
	use crate::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::app_crypto::{app_crypto, sr25519};
	use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};

	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
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
	AddBytes(OpaqueMultiaddr, Vec<u8>, AccountId, bool),
	CatBytes(OpaqueMultiaddr, Vec<u8>, AccountId),
	InsertPin(OpaqueMultiaddr, Vec<u8>, AccountId, bool),
	RemovePin(OpaqueMultiaddr, Vec<u8>, AccountId, bool),
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, sp_runtime::traits::Hash, traits::Currency};
	use frame_system::{
		offchain::{AppCrypto, CreateSignedTransaction},
		pallet_prelude::*,
	};
	use scale_info::TypeInfo;
	use sp_core::offchain::OpaqueMultiaddr;
	use sp_runtime::offchain::{ipfs, IpfsRequest, IpfsResponse};
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
			OwnershipLayer::Owner
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn bootstrap_nodes)]
	pub(super) type BootstrapNodes<T: Config> =
		StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<OpaqueMultiaddr>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn data_queue)]
	pub(super) type DataQueue<T: Config> =
		StorageValue<_, Vec<DataCommand<T::AccountId>>, ValueQuery>;

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the IPFS pallet.
		type Currency: Currency<Self::AccountId>;

		/// The maximum amount of IPFS Assets a single account can own.
		#[pallet::constant]
		type MaxIpfsOwned: Get<u32>;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		type WeightInfo: WeightInfo;
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
		/// Ensures that an IPFS is not already owned by the account.
		IpfsAlreadyOwned,
		CantCreateRequest,
		RequestTimeout,
		RequestFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A request to add bytes was queued
		QueuedDataToAdd(T::AccountId),
		QueuedDataToCat(T::AccountId),
		PublishedIdentity(T::AccountId),
		Created(T::AccountId, T::Hash),
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		Transferred(T::AccountId, T::AccountId, T::Hash),
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
		AddOwner(T::AccountId, Vec<u8>, T::AccountId),
		RemoveOwner(T::AccountId, Vec<u8>),
		ChangeOwnershipLayer(T::AccountId, Vec<u8>, T::AccountId),
		WriteIpfsAsset(T::AccountId, T::Hash),
		ReadIpfsAsset(T::AccountId, T::Hash),
		DeleteIpfsAsset(T::AccountId, Vec<u8>),
		AddPin(T::AccountId, Vec<u8>),
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
				<DataQueue<T>>::kill();
			}

			0
		}

		fn offchain_worker(block_no: BlockNumberFor<T>) {
			if block_no % 5u32.into() == 0u32.into() {
				if let Err(e) = Self::connection_housekeeping() {
					log::error!(
						"IPFS: Encountered an error while processing data requests: {:?}",
						e
					);
				}
			}

			// handle data requests each block
			if let Err(e) = Self::handle_data_requests() {
				log::error!("IPFS: Encountered an error while processing data requests: {:?}", e);
			}

			if block_no % 5u32.into() == 0u32.into() {
				if let Err(e) = Self::print_metadata() {
					log::error!("IPFS: Encountered an error while obtaining metadata: {:?}", e);
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new unique IPFS.
		///
		/// The actual IPFS creation is done in the `mint()` function.
		#[pallet::weight(T::WeightInfo::create_ipfs_asset())]
		pub fn create_ipfs_asset(
			origin: OriginFor<T>,
			addr: Vec<u8>,
			ci_address: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				!<IpfsAssetOwned<T>>::get(&sender).contains(&ci_address),
				<Error<T>>::IpfsAlreadyOwned
			);

			let multiaddr = OpaqueMultiaddr(addr);

			<DataQueue<T>>::mutate(|queue| {
				queue.push(DataCommand::AddBytes(
					multiaddr,
					ci_address.clone(),
					sender.clone(),
					true,
				))
			});

			Self::deposit_event(Event::QueuedDataToAdd(sender.clone()));

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

			<DataQueue<T>>::mutate(|queue| {
				queue.push(DataCommand::InsertPin(multiaddr, cid.clone(), sender.clone(), true))
			});

			Self::deposit_event(Event::AddPin(sender.clone(), cid.clone()));

			Ok(())
		}

		/// Unpins and deletes an IPFS.
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

			<DataQueue<T>>::mutate(|queue| {
				queue.push(DataCommand::RemovePin(multiaddr, cid.clone(), sender.clone(), true))
			});

			Self::deposit_event(Event::DeleteIpfsAsset(sender.clone(), cid.clone()));

			Ok(())
		}

		/// TODO: Read an IPFS asset.
		#[pallet::weight(0)]
		pub fn read_file(origin: OriginFor<T>, addr: Vec<u8>, cid: Vec<u8>) -> DispatchResult {
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
			<BootstrapNodes<T>>::insert(public_key.clone(), multiaddress.clone());

			Self::deposit_event(Event::PublishedIdentity(signer.clone()));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_ipfs_add_results(
			origin: OriginFor<T>,
			admin: AccountOf<T>,
			cid: Vec<u8>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			<DataQueue<T>>::take();
			let mut ipfs = Ipfs::<T> {
				cid_addr: cid.clone(),
				owners: BTreeMap::<AccountOf<T>, OwnershipLayer>::new(),
			};

			ipfs.owners.insert(admin.clone(), OwnershipLayer::default());
			let _ipfs_id = T::Hashing::hash_of(&ipfs);

			<IpfsAssetOwned<T>>::try_mutate(&admin, |ipfs_vec| ipfs_vec.try_push(cid.clone()))
				.map_err(|_| <Error<T>>::ExceedMaxIpfsOwned)?;
			<IpfsAsset<T>>::insert(cid.clone(), ipfs);

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_ipfs_pin_results(
			origin: OriginFor<T>,
			admin: AccountOf<T>,
			cid: Vec<u8>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			<DataQueue<T>>::take();

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_ipfs_delete_results(
			origin: OriginFor<T>,
			admin: AccountOf<T>,
			cid: Vec<u8>,
		) -> DispatchResult {
			ensure_signed(origin)?;

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
			<IpfsAsset<T>>::remove(cid.clone());

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
		pub fn remove_owner(
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

	impl<T: Config> Pallet<T> {
		pub fn determine_account_ownership_layer(
			cid: &Vec<u8>,
			acct: &T::AccountId,
		) -> Result<OwnershipLayer, Error<T>> {
			match Self::ipfs_asset(cid) {
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

		fn ipfs_request(
			req: IpfsRequest,
			deadline: impl Into<Option<Timestamp>>,
		) -> Result<IpfsResponse, Error<T>> {
			let ipfs_request =
				ipfs::PendingRequest::new(req).map_err(|_| Error::<T>::CantCreateRequest)?;

			log::info!("{:?}", ipfs_request.request);

			ipfs_request
				.try_wait(deadline)
				.map_err(|_| Error::<T>::RequestTimeout)?
				.map(|r| r.response)
				.map_err(|e| {
					if let ipfs::Error::IoError(err) = e {
						log::error!(
							"IPFS Request failed: {}",
							sp_std::str::from_utf8(&err).unwrap()
						);
					} else {
						log::error!("IPFS Request failed: {:?}", e);
					}
					Error::<T>::RequestFailed
				})
		}

		fn connection_housekeeping() -> Result<(), Error<T>> {
			let deadline = Some(timestamp().add(Duration::from_millis(5_000)));

			let (public_key, addrs) = if let IpfsResponse::Identity(public_key, addrs) =
				Self::ipfs_request(IpfsRequest::Identity, deadline)?
			{
				(public_key, addrs)
			} else {
				unreachable!("only `Identity` is a valid response type.");
			};

			if !<BootstrapNodes<T>>::contains_key(public_key.clone()) {
				if let Some(bootstrap_node) = &<BootstrapNodes<T>>::iter().nth(0) {
					if let Some(bootnode_maddr) = bootstrap_node.1.clone().pop() {
						if let IpfsResponse::Success = Self::ipfs_request(
							IpfsRequest::Connect(bootnode_maddr.clone()),
							deadline,
						)? {
							log::info!(
								"Succesfully connected to a bootstrap node: {:?}",
								&bootnode_maddr.0
							);
						} else {
							log::info!(
								"Failed to  connect to a bootstrap node with multi_address: {:?}",
								&bootnode_maddr.0
							);

							if let Some(next_bootnode_maddr) = bootstrap_node.1.clone().pop() {
								if let IpfsResponse::Success = Self::ipfs_request(
									IpfsRequest::Connect(next_bootnode_maddr.clone()),
									deadline,
								)? {
									log::info!(
										"Succesfully connected to a bootstrap node: {:?}",
										&next_bootnode_maddr.0
									);
								} else {
									log::info!("Failed to  connect to a bootstrap node with multi_address: {:?}", &next_bootnode_maddr.0);
								}
							}
						}
					}
				}

				let signer = Signer::<T, T::AuthorityId>::all_accounts();
				if !signer.can_sign() {
					log::error!(
						"No local accounts available. Consider adding one via `author_insertKey` RPC",
					);
				}

				let results =
					signer.send_signed_transaction(|_account| Call::submit_ipfs_identity {
						public_key: public_key.clone(),
						multiaddress: addrs.clone(),
					});

				for (_, res) in &results {
					match res {
						Ok(()) => log::info!("Submitted ipfs identity results"),
						Err(e) => log::error!("Failed to submit transcation: {:?}", e),
					}
				}
			}
			Ok(())
		}

		fn handle_data_requests() -> Result<(), Error<T>> {
			let data_queue = DataQueue::<T>::get();
			let len = data_queue.len();

			if len != 0 {
				log::info!("IPFS: {} entries in the data queue", len);
			}

			let deadline = Some(timestamp().add(Duration::from_millis(5_000)));

			for cmd in data_queue.into_iter() {
				match cmd {
					DataCommand::AddBytes(m_addr, cid, admin, is_recursive) => {
						// this should work for different CID's. If you try to
						// connect and upload the same CID, you will get a duplicate
						// conn error. @charmitro
						match Self::ipfs_request(IpfsRequest::Connect(m_addr.clone()), deadline) {
							Ok(IpfsResponse::Success) => match Self::ipfs_request(
								IpfsRequest::CatBytes(cid.clone()),
								deadline,
							) {
								Ok(IpfsResponse::CatBytes(data)) => {
									log::info!("IPFS: fetched data");
									Self::ipfs_request(
										IpfsRequest::Disconnect(m_addr.clone()),
										deadline,
									)?;

									log::info!(
										"IPFS: disconnected from {}",
										sp_std::str::from_utf8(&m_addr.0).expect(
											"our own calls can be trusted to be UTF-8; qed"
										)
									);

									match Self::ipfs_request(
										IpfsRequest::AddBytes(data.clone()),
										deadline,
									) {
										Ok(IpfsResponse::AddBytes(new_cid)) => {
											log::info!(
												"IPFS: added data with CID: {}",
												sp_std::str::from_utf8(&new_cid).expect(
													"our own IPFS node can be trunsted here; qed"
												)
											);

											// signer is the probably the node account (often Alice)
											let signer =
												Signer::<T, T::AuthorityId>::all_accounts();
											if !signer.can_sign() {
												log::error!(
												"No local account available. Consider adding one via `author_insertKey` RPC.",
											);
											}

											let results =
												signer.send_signed_transaction(|_account| {
													Call::submit_ipfs_add_results {
														// admin should be the actual account that we is doing
														// the transcation in the first place(create_ipfs_asset)
														admin: admin.clone(),
														cid: cid.clone(),
													}
												});

											for (_, res) in &results {
												match res {
													Ok(()) => {
														// also this probably doesn't work.
														log::info!("Submited IPFS results")
													}
													Err(e) => log::error!(
														"Failed to submit transaction: {:?}",
														e
													),
												}
											}

											match Self::ipfs_request(
												IpfsRequest::InsertPin(cid.clone(), is_recursive),
												deadline,
											) {
												Ok(IpfsResponse::Success) => {
													log::info!(
														"IPFS: pinned data with CID: {}",
														sp_std::str::from_utf8(&cid)
															.expect("trusted")
													)
												}
												Ok(_) => {
													unreachable!("only Success can be a response for that request type")
												}
												Err(e) => log::error!("IPFS: Pin Error: {:?}", e),
											}

											match Self::ipfs_request(
												IpfsRequest::Disconnect(m_addr.clone()),
												deadline,
											) {
												Ok(IpfsResponse::Success) => {
													log::info!("IPFS: Disconeccted Succes")
												}
												Ok(_) => {
													unreachable!("only Success can be a response for that request type")
												}
												Err(e) => {
													log::error!("IPFS: Disconnect Error: {:?}", e)
												}
											}
										}
										Ok(_) => unreachable!(
											"only AddBytes can be a response for that request type"
										),
										Err(e) => log::error!("IPFS: Add Error: {:?}", e),
									}
								}
								Ok(_) => unreachable!(
									"only AddBytes can be a response for that request type."
								),
								Err(e) => log::error!("IPFS: add error: {:?}", e),
							},
							Ok(_) => unreachable!(
								"only AddBytes can be a response for that request type."
							),
							Err(e) => log::error!("IPFS: add error: {:?}", e),
						}
					}

					DataCommand::CatBytes(m_addr, cid, _admin) => {
						match Self::ipfs_request(IpfsRequest::CatBytes(cid.clone()), deadline) {
							Ok(IpfsResponse::CatBytes(data)) => {
								log::info!("IPFS: fetched data");
								Self::ipfs_request(
									IpfsRequest::Disconnect(m_addr.clone()),
									deadline,
								)?;

								log::info!(
									"IPFS: disconnected from {}",
									sp_std::str::from_utf8(&m_addr.0)
										.expect("our own calls can be trusted to be UTF-8; qed")
								);
							}
							Ok(_) => unreachable!(
								"only AddBytes can be a response for that request type."
							),
							Err(e) => log::error!("IPFS: add error: {:?}", e),
						}
					}

					DataCommand::InsertPin(m_addr, cid, _admin, is_recursive) => {
						match Self::ipfs_request(
							IpfsRequest::InsertPin(cid.clone(), is_recursive),
							deadline,
						) {
							Ok(IpfsResponse::Success) => {
								log::info!(
									"IPFS: pinned data with CID: {}",
									sp_std::str::from_utf8(&cid).expect("trusted")
								);

								let signer = Signer::<T, T::AuthorityId>::all_accounts();
								if !signer.can_sign() {
									log::error!(
									"No local account available. Consider adding one via `author_insertKey` RPC",
								);
								}

								let results = signer.send_signed_transaction(|_account| {
									Call::submit_ipfs_pin_results {
										admin: _admin.clone(),
										cid: cid.clone(),
									}
								});

								for (_, res) in &results {
									match res {
										Ok(()) => {
											log::info!("Submited IPFS results")
										}
										Err(e) => log::error!(
											"Failed to submit transaction: {:?}",
											e
										),
									}
								}
							}
							Ok(_) => {
								unreachable!("only Success can be a response for that request type")
							}
							Err(e) => log::error!("IPFS: Pin Error: {:?}", e),
						}
					}

					DataCommand::RemovePin(_m_addr, cid, admin, is_recursive) => {
						match Self::ipfs_request(
							IpfsRequest::RemovePin(cid.clone(), is_recursive),
							deadline,
						) {
							Ok(IpfsResponse::Success) => {
								log::info!(
									"IPFS: unpinned data with CID: {:?}",
									sp_std::str::from_utf8(&cid).expect("qrff")
								);

								match Self::ipfs_request(
									IpfsRequest::RemoveBlock(cid.clone()),
									deadline,
								) {
									Ok(IpfsResponse::RemoveBlock(cid)) => {
										log::info!(
											"IPFS: block deleted with CID: {}",
											sp_std::str::from_utf8(&cid).expect("qyzc")
										);

										let signer = Signer::<T, T::AuthorityId>::all_accounts();
										if !signer.can_sign() {
											log::error!(
											"No local account available. Consider adding one via `author_insertKey` RPC",
										);
										}

										let results = signer.send_signed_transaction(|_account| {
											Call::submit_ipfs_delete_results {
												admin: admin.clone(),
												cid: cid.clone(),
											}
										});

										for (_, res) in &results {
											match res {
												Ok(()) => {
													log::info!("Submited IPFS results")
												}
												Err(e) => log::error!(
													"Failed to submit transaction: {:?}",
													e
												),
											}
										}
									}
									Ok(_) => unreachable!(
										"only RemoveBlock can be a response for that request type"
									),
									Err(e) => log::error!("IPFS: Remove Block Error: {:?}", e),
								}
							}
							Ok(_) => {
								unreachable!("only Success can be a response for that request type")
							}
							Err(e) => log::error!("IPFS: Remove Pin Error: {:?}", e),
						}
					}
				}
			}
			Ok(())
		}

		fn print_metadata() -> Result<(), Error<T>> {
			let deadline = Some(timestamp().add(Duration::from_millis(5_000)));

			let peers = if let IpfsResponse::Peers(peers) =
				Self::ipfs_request(IpfsRequest::Peers, deadline)?
			{
				peers
			} else {
				unreachable!("only Peers can be a response for that request type: qed");
			};

			let peer_count = peers.len();

			log::info!("IPFS: currently connencted to {} peers", &peer_count,);

			Ok(())
		}
	}
}
