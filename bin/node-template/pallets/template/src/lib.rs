#![cfg_attr(not(feature = "std"), no_std)]

//! # Iris Storage Pallet
//!
//! A module to interact with Iris Storage
//!
//! ## Overview
//! Disclaimer: This pallet is in the tadpole state
//!
//! ### Goals
//! 
//! ## Interface
//!
//! The Iris module provides functionality for creation and management of storage assets 
//! ### Dispatchable Functions 
//!
//! #### Permissionless functions
//! * ipfs_add_bytes
//! * mint_ticket
//!
//! #### Permissioned Functions
//! * submit_ipfs_results (private?)
//! * destroy_ticket
//! * ipfs_cat_bytes
//!

use scale_info::TypeInfo;
use codec::{Encode, Decode};
use frame_support::{
    debug,
    traits::ReservableCurrency,
};
use frame_system::{
    self as system, ensure_signed,
    offchain::{
        AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer, SubmitTransaction,
    },
};

use sp_core::offchain::{
    Duration, IpfsRequest, IpfsResponse, OpaqueMultiaddr, Timestamp,
};

use sp_core::crypto::KeyTypeId;
use sp_io::{
    offchain::timestamp,
    offchain_index,
};
use sp_runtime::{
    offchain::ipfs,
    RuntimeDebug,
    transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
	},
    traits::{
        AtLeast32BitUnsigned, StaticLookup,
    }
};
use sp_std::{str, vec::Vec, prelude::*};
use codec::HasCompact;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ipfs");


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
    /// (ipfs_address, cid, requesting node address, ticket_config)
    AddBytes(OpaqueMultiaddr, Vec<u8>, LookupSource, AssetId, Balance),
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
	use frame_support::{debug, dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::{
        pallet_prelude::*,
        offchain::{
            AppCrypto,
            CreateSignedTransaction,
        },
    };
	use sp_core::offchain::{
		Duration, IpfsRequest, IpfsResponse, OpaqueMultiaddr, Timestamp,
	};
	use sp_std::{str, vec::Vec, prelude::*};

	#[pallet::config]
    /// the module configuration trait
	pub trait Config:CreateSignedTransaction<Call<Self>> + frame_system::Config + pallet_assets::Config {
        /// The overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// the authority id used for sending signed txs
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
        /// the overarching call type
	    type Call: From<Call<Self>>;
        /// the currency used (OBOL)
        type Currency: ReservableCurrency<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
    #[pallet::getter(fn data_queue)]
	/// A queue of data to publish or obtain on IPFS.
	pub(super) type DataQueue<T: Config> = StorageValue<
        _,
        Vec<DataCommand<<T::Lookup as StaticLookup>::Source, T::AssetId, T::Balance, T::AccountId>>,
        ValueQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn cid_map)]
    /// Store the map associating owned CID to a specific asset ID
    /// currently: cid -> accountid -> assetid
    /// might change: accountid -> cid -> assetid
    pub(super) type CidMap<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        Vec<u8>,
        Blake2_128Concat,
        T::AccountId,
        T::AssetId,
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
	}

	#[pallet::error]
	pub enum Error<T> {
		CantCreateRequest,
        RequestTimeout,
        RequestFailed,
        OffchainSignedTxError,
        NoLocalAcctForSigning,
        CantCreateAssetClass,
        CantMintAssets,
        NoSuchOwnedContent,
        NoSuchAssetClass,
        InsufficientBalance,
	}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
         // needs to be synchronized with offchain_worker actitivies
         fn on_initialize(block_number: T::BlockNumber) -> Weight {
            if block_number % 2u32.into() == 1u32.into() {
                <DataQueue<T>>::kill();
            }

            0
        }
        /// The offchain worker processes requests queued by other nodes
        fn offchain_worker(block_number: T::BlockNumber) {
            // process a request every three blocks
            if block_number % 3u32.into() == 1u32.into() {
                if let Err(e) = Self::handle_data_requests() {
                    log::error!("IPFS: Encountered an error while processing data requests: {:?}", e);
                }
            }
            // print metadata every five blocks
            if block_number % 5u32.into() == 0u32.into() {
                if let Err(e) = Self::print_metadata() {
                    log::error!("IPFS: Encountered an error while obtaining metadata: {:?}", e);
                }
            }
        }
    }

    #[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are marked as valid.
        ///
        /// * `_source`: The source of the transaction
        /// * `call`: The call creating the utxo
        ///
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit_ipfs_results{admin, cid, id, balance} = call {
				Self::validate_transaction_parameters(cid.to_vec())
			} else {
				InvalidTransaction::Call.into()
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
                    id.clone(),
                    balance.clone(),
                )));
            Self::deposit_event(Event::QueuedDataToAdd(who.clone()));
			Ok(())
        }

        /// Queue a request to retrieve data behind some owned CID from the IPFS network
        ///
        /// * origin: any origin
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
                )));
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
        /// TODO: should change to have an unsigned transaction with a signed payload
        #[pallet::weight(0)]
        pub fn submit_ipfs_results(
            origin: OriginFor<T>,
            admin: <T::Lookup as StaticLookup>::Source,
            cid: Vec<u8>,
            id: T::AssetId,
            balance: T::Balance,
        ) -> DispatchResult {
            // ensure_none(origin)?;
            let who = ensure_signed(origin)?;
            let new_origin = system::RawOrigin::Signed(who).into();
            <pallet_assets::Pallet<T>>::create(new_origin, id.clone(), admin.clone(), balance)
                .map_err(|_| Error::<T>::CantCreateAssetClass);
            let which_admin = T::Lookup::lookup(admin.clone())?;
            <CidMap<T>>::insert(cid.clone(), which_admin, id.clone());
            log::info!("inserted entry into storage");
            Self::deposit_event(Event::AssetClassCreated(id.clone()));
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
            ensure!(CidMap::<T>::contains_key(cid.clone(), who.clone()), Error::<T>::NoSuchOwnedContent);
            let asset_id = CidMap::<T>::get(cid.clone(), who.clone());
            <pallet_assets::Pallet<T>>::mint(new_origin, asset_id.clone(), beneficiary, amount)
                .map_err(|_| Error::<T>::CantMintAssets);
            log::info!("Minted {:?} tickets", amount);
            Self::deposit_event(Event::AssetCreated(asset_id.clone()));
            Ok(())
        }

        /// TODO: leaving this as is for now... I feel like this will require some further thought. We almost need a dex-like feature
        /// Purchase a ticket to access some content. The purchase is done in the native currency (OBOL).
        ///
        /// * origin: any origin
        /// * owner: The owner to identify the asset class for which a ticket is to be purchased
        /// * cid: The CID to identify the asset class for which a ticket is to be purchased
        /// * amount: The number of tickets to purchase
        #[pallet::weight(0)]
        pub fn purchase_ticket(
            origin: OriginFor<T>,
            owner: <T::Lookup as StaticLookup>::Source,
            cid: Vec<u8>,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin);
            // determine price for amount of asset and verify origin has a min balance
            // transfer native currency to asset class admin
            // admin transfers the requested amount of tokens to the buyer
            // for now, going with a simplified approach: tickets are all free
            // let asset_id = CidMap::<T>::get(cid.clone(), who.clone());
            // log.info!("found an asset id");
            // <pallet_assets::Pallet<T>>::transfer();
            Ok(())
        }
	}
}

impl<T: Config> Pallet<T> {
    // send a request to the local IPFS node; can only be called be an off-chain worker
    fn ipfs_request(req: IpfsRequest, deadline: impl Into<Option<Timestamp>>) -> Result<IpfsResponse, Error<T>> {
        let ipfs_request = ipfs::PendingRequest::new(req).map_err(|_| Error::<T>::CantCreateRequest)?;
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

    fn handle_data_requests() -> Result<(), Error<T>> {
        let data_queue = DataQueue::<T>::get();
        let len = data_queue.len();
        if len != 0 {
            log::info!("IPFS: {} entr{} in the data queue", len, if len == 1 { "y" } else { "ies" });
        }

        let deadline = Some(timestamp().add(Duration::from_millis(5_000)));
        for cmd in data_queue.into_iter() {
            match cmd {
                // ticket_config
                DataCommand::AddBytes(addr, cid, admin, id, balance) => {
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
                                    // let call = Call::submit_ipfs_results{
                                    //     admin: admin,
                                    //     cid: new_cid,
                                    //     id: id,
                                    //     balance: balance,
                                    // };
                                    // SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
                                    //     .map_err(|()| "Unable to submit unsigned transaction.")
                                    //     .map(|()| "done");
                                    let signer = Signer::<T, T::AuthorityId>::all_accounts();
                                    if !signer.can_sign() {
                                        log::error!(
                                            "No local accounts available. Consider adding one via `author_insertKey` RPC.",
                                        );
                                    }
                                    let results = signer.send_signed_transaction(|_account| { 
                                        Call::submit_ipfs_results{
                                            admin: admin.clone(),
                                            cid: new_cid.clone(),
                                            id: id.clone(),
                                            balance: balance.clone(),
                                        }
                                     });
                            
                                    for (acc, res) in &results {
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
                    // verify that the recipient owns at least one ticket
                    ensure!(CidMap::<T>::contains_key(cid.clone(), owner.clone()), Error::<T>::NoSuchOwnedContent);
                    let asset_id = CidMap::<T>::get(cid.clone(), owner.clone());
                    let balance = <pallet_assets::Pallet<T>>::balance(asset_id.clone(), recipient.clone());
                    ensure!(balance > 0, Error::<T>::InsufficientBalance);
                    log::info!("found balance {:?}", balance);
                    match Self::ipfs_request(IpfsRequest::CatBytes(cid.clone()), deadline) {
                        Ok(IpfsResponse::CatBytes(data)) => {
                            log::info!("IPFS: Fetched data from IPFS succesfully. What should I do with it now?");
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
        let deadline = Some(timestamp().add(Duration::from_millis(200)));

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

    fn validate_transaction_parameters(cid: Vec<u8>) -> TransactionValidity {
        /// for now assume everything is valid
        ValidTransaction::with_tag_prefix("ipfs")
            .longevity(5)
            .propagate(true)
            .build()
    }
}