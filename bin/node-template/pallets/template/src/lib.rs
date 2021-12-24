#![cfg_attr(not(feature = "std"), no_std)]

//! # Iris Storage Pallet
//!
//!
//! ## Overview
//! Disclaimer: This pallet is in the tadpole state
//!
//! ### Goals
//! The Iris module provides functionality for creation and management of storage assets and access management
//! 
//! ### Dispatchable Functions 
//!
//! #### Permissionless functions
//! * create_storage_asset
//! * request_data
//!
//! #### Permissioned Functions
//! * submit_ipfs_add_results
//! * submit_ipfs_identity
//! * submit_rpc_ready
//! * destroy_ticket
//! * mint_tickets
//!

use scale_info::TypeInfo;
use codec::{Encode, Decode};
use frame_support::{
    ensure,
    traits::ReservableCurrency,
};
use frame_system::{
    self as system, ensure_signed,
    offchain::{
        SendSignedTransaction, 
        Signer,
    },
};

use sp_core::{
    offchain::{
        Duration, IpfsRequest, IpfsResponse, OpaqueMultiaddr, Timestamp, StorageKind,
    },
    crypto::KeyTypeId,
    Bytes,
};

use sp_io::offchain::timestamp;
use sp_runtime::{
    offchain::{ 
        ipfs,
    },
    RuntimeDebug,
    traits::StaticLookup,
};
use sp_std::{
    str,
    vec::Vec,
    prelude::*,
    convert::TryInto,
};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"iris");


pub mod crypto {
	use crate::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::app_crypto::{app_crypto, sr25519};
	use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};

	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;
	// implemented for untime
	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[derive(Encode, Decode, RuntimeDebug, PartialEq, TypeInfo)]
pub enum DataCommand<LookupSource, AssetId, Balance, AccountId> {
    /// (ipfs_address, cid, requesting node address, filename, asset id, balance)
    AddBytes(OpaqueMultiaddr, Vec<u8>, LookupSource, Vec<u8>, AssetId, Balance),
    // /// owner, cid
    CatBytes(AccountId, Vec<u8>, AccountId),
}

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::{
        pallet_prelude::*,
        offchain::{
            AppCrypto,
            CreateSignedTransaction,
        },
    };
	use sp_core::offchain::OpaqueMultiaddr;
	use sp_std::{str, vec::Vec};

	#[pallet::config]
    /// the module configuration trait
	pub trait Config:CreateSignedTransaction<Call<Self>> + frame_system::Config + pallet_assets::Config {
        /// The overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// the authority id used for sending signed txs
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
        /// the overarching call type
	    type Call: From<Call<Self>>;
        /// the currency used
        type Currency: ReservableCurrency<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

    /// map the public key to a list of multiaddresses
    #[pallet::storage]
    #[pallet::getter(fn bootstrap_nodes)]
    pub(super) type BootstrapNodes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,
        Vec<OpaqueMultiaddr>,
        ValueQuery,
    >;

    /// A queue of data to publish or obtain on IPFS.
	#[pallet::storage]
    #[pallet::getter(fn data_queue)]
	pub(super) type DataQueue<T: Config> = StorageValue<
        _,
        Vec<DataCommand<
            <T::Lookup as StaticLookup>::Source, 
            T::AssetId,
            T::Balance,
            T::AccountId>
        >,
        ValueQuery
    >;

    /// Store the map associating owned CID to a specific asset ID
    ///
    /// asset_admin_accountid -> CID -> asset id
    #[pallet::storage]
    #[pallet::getter(fn created_asset_classes)]
    pub(super) type AssetClassOwnership<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        Vec<u8>,
        T::AssetId,
        ValueQuery,
    >;

    /// Store the map associated a node with the assets to which they have access
    ///
    /// asset_owner_accountid -> CID -> asset_class_owner_accountid
    #[pallet::storage]
    #[pallet::getter(fn asset_access)]
    pub(super) type AssetAccess<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        Vec<u8>,
        T::AccountId,
        ValueQuery,
    >;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        /// A request to add bytes was queued
        QueuedDataToAdd(T::AccountId),
        /// A request to retrieve bytes was queued
        QueuedDataToCat(T::AccountId),
        /// A new asset class was created (add bytes command processed)
        AssetClassCreated(T::AssetId),
        /// A new asset was created (tickets minted)
        AssetCreated(T::AssetId),
        /// A node's request to access data via the RPC endpoint has been processed
        DataReady(T::AccountId),
        /// A node has published ipfs identity results on chain
        PublishedIdentity(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
        /// could not build the ipfs request
		CantCreateRequest,
        /// the request to IPFS timed out
        RequestTimeout,
        /// the request to IPFS failed
        RequestFailed,
        /// The tx could not be signed
        OffchainSignedTxError,
        /// you cannot sign a tx
        NoLocalAcctForSigning,
        /// could not create a new asset
        CantCreateAssetClass,
        /// could not mint a new asset
        CantMintAssets,
        /// there is no asset associated with the specified cid
        NoSuchOwnedContent,
        /// the specified asset class does not exist
        NoSuchAssetClass,
        /// the account does not have a sufficient balance
        InsufficientBalance,
	}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
         fn on_initialize(block_number: T::BlockNumber) -> Weight {
            // needs to be synchronized with offchain_worker actitivies
            if block_number % 2u32.into() == 1u32.into() {
                <DataQueue<T>>::kill();
            }

            0
        }
        fn offchain_worker(block_number: T::BlockNumber) {
            // every 5 blocks
            if block_number % 5u32.into() == 0u32.into() {
                if let Err(e) = Self::connection_housekeeping() {
                    log::error!("IPFS: Encountered an error while processing data requests: {:?}", e);
                }
            }

            // handle data requests each block
            if let Err(e) = Self::handle_data_requests() {
                log::error!("IPFS: Encountered an error while processing data requests: {:?}", e);
            }

            // every 5 blocks
            if block_number % 5u32.into() == 0u32.into() {
                if let Err(e) = Self::print_metadata() {
                    log::error!("IPFS: Encountered an error while obtaining metadata: {:?}", e);
                }
            }
        }
    }

	#[pallet::call]
	impl<T: Config> Pallet<T> {

        /// submits an on-chain request to fetch data and add it to iris 
        /// 
        /// * `addr`: the multiaddress where the data exists
        ///       example: /ip4/192.168.1.170/tcp/4001/p2p/12D3KooWMvyvKxYcy9mjbFbXcogFSCvENzQ62ogRxHKZaksFCkAp
        /// * `cid`: the cid to fetch from the multiaddress
        ///       example: QmPZv7P8nQUSh2CpqTvUeYemFyjvMjgWEs8H1Tm8b3zAm9
        /// * `id`: (temp) the unique id of the asset class -> should be generated instead
        /// * `balance`: the balance the owner is willing to use to back the asset class which will be created
        ///
        #[pallet::weight(0)]
        pub fn create_storage_asset(
            origin: OriginFor<T>,
            admin: <T::Lookup as StaticLookup>::Source,
            addr: Vec<u8>,
            cid: Vec<u8>,
            name: Vec<u8>,
            id: T::AssetId,
            balance: T::Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let multiaddr = OpaqueMultiaddr(addr);
            <DataQueue<T>>::mutate(
                |queue| queue.push(DataCommand::AddBytes(
                    multiaddr,
                    cid,
                    admin.clone(),
                    name.clone(),
                    id.clone(),
                    balance.clone(),
                )));
            Self::deposit_event(Event::QueuedDataToAdd(who.clone()));
			Ok(())
        }

        /// Queue a request to retrieve data behind some owned CID from the IPFS network
        ///
        /// * owner: The owner node
        /// * cid: the cid to which you are requesting access
        ///
		#[pallet::weight(0)]
        pub fn request_data(
            origin: OriginFor<T>,
            owner: <T::Lookup as StaticLookup>::Source,
            cid: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let owner_account = T::Lookup::lookup(owner)?;

            <DataQueue<T>>::mutate(
                |queue| queue.push(DataCommand::CatBytes(
                    owner_account.clone(),
                    cid.clone(),
                    who.clone(),
                )
            ));

            Self::deposit_event(Event::QueuedDataToCat(who.clone()));
            
            Ok(())
        }

        /// should only be called by offchain workers... how to ensure this?
        /// submits IPFS results on chain and creates new ticket config in runtime storage
        ///
        /// * `admin`: The admin account
        /// * `cid`: The cid generated by the OCW
        /// * `id`: The AssetId (passed through from the create_storage_asset call)
        /// * `balance`: The balance (passed through from the create_storage_asset call)
        ///
        #[pallet::weight(0)]
        pub fn submit_ipfs_add_results(
            origin: OriginFor<T>,
            admin: <T::Lookup as StaticLookup>::Source,
            cid: Vec<u8>,
            id: T::AssetId,
            balance: T::Balance,
        ) -> DispatchResult {
            // DANGER: This can currently be called by anyone, not just an OCW.
            // if we send an unsigned transaction then we can ensure there is no origin
            // however, the call to create the asset requires an origin, which is a little problematic
            // ensure_none(origin)?;
            let who = ensure_signed(origin)?;
            let new_origin = system::RawOrigin::Signed(who).into();

            <pallet_assets::Pallet<T>>::create(new_origin, id.clone(), admin.clone(), balance)
                .map_err(|_| Error::<T>::CantCreateAssetClass)?;
            
            let which_admin = T::Lookup::lookup(admin.clone())?;
            <AssetClassOwnership<T>>::insert(which_admin, cid.clone(), id.clone());
            
            Self::deposit_event(Event::AssetClassCreated(id.clone()));
            
            Ok(())
        }

        /// Should only be callable by OCWs (TODO)
        /// Submit the results of an `ipfs identity` call to be stored on chain
        ///
        /// * origin: a validator node
        /// * public_key: The IPFS node's public key
        /// * multiaddresses: A vector of multiaddresses associate with the public key
        ///
        #[pallet::weight(0)]
        pub fn submit_ipfs_identity(
            origin: OriginFor<T>,
            public_key: Vec<u8>,
            multiaddresses: Vec<OpaqueMultiaddr>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <BootstrapNodes::<T>>::insert(public_key.clone(), multiaddresses.clone());
            Self::deposit_event(Event::PublishedIdentity(who.clone()));
            Ok(())
        }

        /// Should only be callable by OCWs (TODO)
        /// Submit the results onchain to notify a beneficiary that their data is available: TODO: how to safely share host? spam protection on rpc endpoints?
        ///
        /// * `beneficiary`: The account that requested the data
        /// * `host`: The node's host where the data has been made available (RPC endpoint)
        ///
        #[pallet::weight(0)]
        pub fn submit_rpc_ready(
            origin: OriginFor<T>,
            beneficiary: T::AccountId,
            // host: Vec<u8>,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            Self::deposit_event(Event::DataReady(beneficiary));
            Ok(())
        }

        /// Only callable by the owner of the asset class 
        /// mint a static number of assets (tickets) for some asset class
        ///
        /// * origin: should be the owner of the asset class
        /// * beneficiary: the address to which the newly minted assets are assigned
        /// * cid: a cid owned by the origin, for which an asset class exists
        /// * amount: the number of tickets to mint
        ///
        #[pallet::weight(0)]
        pub fn mint_tickets(
            origin: OriginFor<T>,
            beneficiary: <T::Lookup as StaticLookup>::Source,
            cid: Vec<u8>,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let new_origin = system::RawOrigin::Signed(who.clone()).into();
            let beneficiary_accountid = T::Lookup::lookup(beneficiary.clone())?;

            ensure!(AssetClassOwnership::<T>::contains_key(who.clone(), cid.clone()), Error::<T>::NoSuchOwnedContent);
            
            let asset_id = AssetClassOwnership::<T>::get(who.clone(), cid.clone(),);
            <pallet_assets::Pallet<T>>::mint(new_origin, asset_id.clone(), beneficiary.clone(), amount)
                .map_err(|_| Error::<T>::CantMintAssets)?;
            
            <AssetAccess<T>>::insert(beneficiary_accountid.clone(), cid.clone(), who.clone());
        
            Self::deposit_event(Event::AssetCreated(asset_id.clone()));
            Ok(())
        }

        /// TODO: leaving this as is for now... I feel like this will require some further thought. We almost need a dex-like feature
        /// Purchase a ticket to access some content.
        ///
        /// * origin: any origin
        /// * owner: The owner to identify the asset class for which a ticket is to be purchased
        /// * cid: The CID to identify the asset class for which a ticket is to be purchased
        /// * amount: The number of tickets to purchase
        ///
        #[pallet::weight(0)]
        pub fn purchase_ticket(
            origin: OriginFor<T>,
            _owner: <T::Lookup as StaticLookup>::Source,
            _cid: Vec<u8>,
            #[pallet::compact] _amount: T::Balance,
            _test: T::Signature,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            // determine price for amount of asset and verify origin has a min balance
            // transfer native currency to asset class admin
            // admin transfers the requested amount of tokens to the buyer
            Ok(())
        }
	}
}

impl<T: Config> Pallet<T> {
    /// implementation for RPC runtime aPI to retrieve bytes from the node's local storage
    /// 
    /// * public_key: The account's public key as bytes
    /// * signature: The signer's signature as bytes
    /// * message: The signed message as bytes
    ///
    pub fn retrieve_bytes(
        _public_key: Bytes,
		_signature: Bytes,
		message: Bytes,
    ) -> Bytes {
        // TODO: Verify signature, update offchain storage keys...
        let message_vec: Vec<u8> = message.to_vec();
        if let Some(data) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &message_vec) {
            Bytes(data.clone())
        } else {
            Bytes(Vec::new())
        }
    }

    /// send a request to the local IPFS node; can only be called be an off-chain worker
    fn ipfs_request(
        req: IpfsRequest,
        deadline: impl Into<Option<Timestamp>>,
    ) -> Result<IpfsResponse, Error<T>> {
        let ipfs_request = ipfs::PendingRequest::new(req).map_err(|_| Error::<T>::CantCreateRequest)?;

        log::info!("{:?}", ipfs_request.request);

        ipfs_request.try_wait(deadline)
            .map_err(|_| Error::<T>::RequestTimeout)?
            .map(|r| r.response)
            .map_err(|e| {
                if let ipfs::Error::IoError(err) = e {
                    log::error!("IPFS: request failed: {}", str::from_utf8(&err).unwrap());
                } else {
                    log::error!("IPFS: request failed: {:?}", e);
                }
                Error::<T>::RequestFailed
            })
    }

    /// manage connection to the iris ipfs swarm
    ///
    /// If the node is already a bootstrap node, do nothing. Otherwise submits a signed tx 
    /// containing the public key and multiaddresses of the embedded ipfs node.
    /// 
    /// Returns an error if communication with the embedded IPFS fails
    fn connection_housekeeping() -> Result<(), Error<T>> {
        let deadline = Some(timestamp().add(Duration::from_millis(5_000)));
        
        let (public_key, addrs) = if let IpfsResponse::Identity(public_key, addrs) = Self::ipfs_request(IpfsRequest::Identity, deadline)? {
            (public_key, addrs)
        } else {
            unreachable!("only `Identity` is a valid response type.");
        };

        if !<BootstrapNodes::<T>>::contains_key(public_key.clone()) {
            if let Some(bootstrap_node) = &<BootstrapNodes::<T>>::iter().nth(0) {
                if let Some(bootnode_maddr) = bootstrap_node.1.clone().pop() {
                    if let IpfsResponse::Success = Self::ipfs_request(IpfsRequest::Connect(bootnode_maddr.clone()), deadline)? {
                        log::info!("Succesfully connected to a bootstrap node: {:?}", &bootnode_maddr.0);
                    } else {
                        log::info!("Failed to connect to the bootstrap node with multiaddress: {:?}", &bootnode_maddr.0);
                        // TODO: this should probably be some recursive function? but we should never exceed a depth of 2 so maybe not
                        if let Some(next_bootnode_maddr) = bootstrap_node.1.clone().pop() {
                            if let IpfsResponse::Success = Self::ipfs_request(IpfsRequest::Connect(next_bootnode_maddr.clone()), deadline)? {
                                log::info!("Succesfully connected to a bootstrap node: {:?}", &next_bootnode_maddr.0);
                            } else {
                                log::info!("Failed to connect to the bootstrap node with multiaddress: {:?}", &next_bootnode_maddr.0);
                            }       
                        }
                    }
                }
            }
            // TODO: should create func to encompass the below logic 
            let signer = Signer::<T, T::AuthorityId>::all_accounts();
            if !signer.can_sign() {
                log::error!(
                    "No local accounts available. Consider adding one via `author_insertKey` RPC.",
                );
            }
             
            let results = signer.send_signed_transaction(|_account| { 
                Call::submit_ipfs_identity{
                    public_key: public_key.clone(),
                    multiaddresses: addrs.clone(),
                }
            });
    
            for (_, res) in &results {
                match res {
                    Ok(()) => log::info!("Submitted ipfs identity results"),
                    Err(e) => log::error!("Failed to submit transaction: {:?}",  e),
                }
            }

        }
        Ok(())
    }

    /// process any requests in the DataQueue
    fn handle_data_requests() -> Result<(), Error<T>> {
        let data_queue = DataQueue::<T>::get();
        let len = data_queue.len();
        if len != 0 {
            log::info!("IPFS: {} entr{} in the data queue", len, if len == 1 { "y" } else { "ies" });
        }

        // TODO: Needs refactoring
        let deadline = Some(timestamp().add(Duration::from_millis(5_000)));
        for cmd in data_queue.into_iter() {
            match cmd {
                DataCommand::AddBytes(addr, cid, admin, _name, id, balance) => {
                    Self::ipfs_request(IpfsRequest::Connect(addr.clone()), deadline)?;
                    log::info!(
                        "IPFS: connected to {}",
                        str::from_utf8(&addr.0).expect("our own calls can be trusted to be UTF-8; qed")
                    );
                    match Self::ipfs_request(IpfsRequest::CatBytes(cid.clone()), deadline) {
                        Ok(IpfsResponse::CatBytes(data)) => {
                            log::info!("IPFS: fetched data");
                            Self::ipfs_request(IpfsRequest::Disconnect(addr.clone()), deadline)?;
                            log::info!(
                                "IPFS: disconnected from {}",
                                str::from_utf8(&addr.0).expect("our own calls can be trusted to be UTF-8; qed")
                            );
                            match Self::ipfs_request(IpfsRequest::AddBytes(data.clone()), deadline) {
                                Ok(IpfsResponse::AddBytes(new_cid)) => {
                                    log::info!(
                                        "IPFS: added data with Cid {}",
                                        str::from_utf8(&new_cid).expect("our own IPFS node can be trusted here; qed")
                                    );
                                    let signer = Signer::<T, T::AuthorityId>::all_accounts();
                                    if !signer.can_sign() {
                                        log::error!(
                                            "No local accounts available. Consider adding one via `author_insertKey` RPC.",
                                        );
                                    }
                                    let results = signer.send_signed_transaction(|_account| { 
                                        Call::submit_ipfs_add_results{
                                            admin: admin.clone(),
                                            cid: new_cid.clone(),
                                            id: id.clone(),
                                            balance: balance.clone(),
                                        }
                                     });
                            
                                    for (_, res) in &results {
                                        match res {
                                            Ok(()) => log::info!("Submitted ipfs results"),
                                            Err(e) => log::error!("Failed to submit transaction: {:?}",  e),
                                        }
                                    }
                                },
                                Ok(_) => unreachable!("only AddBytes can be a response for that request type."),
                                Err(e) => log::error!("IPFS: add error: {:?}", e),
                            }
                        },
                        Ok(_) => unreachable!("only CatBytes can be a response for that request type."),
                        Err(e) => log::error!("IPFS: cat error: {:?}", e),
                    }
                },


                DataCommand::CatBytes(owner, cid, recipient) => {
                    ensure!(AssetClassOwnership::<T>::contains_key(owner.clone(), cid.clone()), Error::<T>::NoSuchOwnedContent);
                    let asset_id = AssetClassOwnership::<T>::get(owner.clone(), cid.clone());
                    let balance = <pallet_assets::Pallet<T>>::balance(asset_id.clone(), recipient.clone());
                    let balance_primitive = TryInto::<u64>::try_into(balance).ok();
                    ensure!(balance_primitive != Some(0), Error::<T>::InsufficientBalance);
                    match Self::ipfs_request(IpfsRequest::CatBytes(cid.clone()), deadline) {
                        Ok(IpfsResponse::CatBytes(data)) => {
                            log::info!("IPFS: Fetched data from IPFS.");
                            // add to offchain index
                            sp_io::offchain::local_storage_set(
                                StorageKind::PERSISTENT,
                                &cid,
                                &data,
                            );
                            let signer = Signer::<T, T::AuthorityId>::all_accounts();
                            if !signer.can_sign() {
                                log::error!(
                                    "No local accounts available. Consider adding one via `author_insertKey` RPC.",
                                );
                            }
                            let results = signer.send_signed_transaction(|_account| { 
                                Call::submit_rpc_ready {
                                    beneficiary: recipient.clone(),
                                }
                            });
                    
                            for (_, res) in &results {
                                match res {
                                    Ok(()) => log::info!("Submitted ipfs results"),
                                    Err(e) => log::error!("Failed to submit transaction: {:?}",  e),
                                }
                            }
                        },
                        Ok(_) => unreachable!("only CatBytes can be a response for that request type."),
                        Err(e) => log::error!("IPFS: cat error: {:?}", e),
                    }
                }
            }
        }

        Ok(())
    }
    
    fn print_metadata() -> Result<(), Error<T>> {
        let deadline = Some(timestamp().add(Duration::from_millis(5_000)));

        let peers = if let IpfsResponse::Peers(peers) = Self::ipfs_request(IpfsRequest::Peers, deadline)? {
            peers
        } else {
            unreachable!("only Peers can be a response for that request type; qed");
        };
        let peer_count = peers.len();

        log::info!(
            "IPFS: currently connected to {} peer{}",
            peer_count,
            if peer_count == 1 { "" } else { "s" },
        );
        Ok(())
    }
}
