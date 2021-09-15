#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use frame_support::debug;
use frame_system::{
    self as system, ensure_signed,
    offchain::{
        AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
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
    RuntimeDebug
};
use sp_std::{str, vec::Vec, prelude::*};

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

#[derive(Encode, Decode, RuntimeDebug, PartialEq)]
pub enum ConnectionCommand {
    ConnectTo(OpaqueMultiaddr),
    DisconnectFrom(OpaqueMultiaddr),
}

#[derive(Encode, Decode, RuntimeDebug, PartialEq)]
pub enum DataCommand {
    /// (data, filename)
    AddBytes(Vec<u8>, Vec<u8>),
    /// cid
    CatBytes(Vec<u8>),
    /// cid
    InsertPin(Vec<u8>),
    /// hash
    RemoveBlock(Vec<u8>),
    /// cid
    RemovePin(Vec<u8>),
}

#[derive(Encode, Decode, RuntimeDebug, PartialEq)]
pub enum DhtCommand {
    FindPeer(Vec<u8>),
    GetProviders(Vec<u8>),
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

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The identifier type for an offchain worker.
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	    /// The overarching dispatch call type.
	    type Call: From<Call<Self>>;
	}

    #[deprecated(note = "use `Event` instead")]
	pub type RawEvent<T> = Event<T>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
    #[pallet::getter(fn connection_queue)]
	// A list of addresses to connect to and disconnect from.
	pub type ConnectionQueue<T: Config> = StorageValue<_, Vec<ConnectionCommand>, ValueQuery>;

	#[pallet::storage]
    #[pallet::getter(fn data_queue)]
	// A queue of data to publish or obtain on IPFS.
	pub type DataQueue<T: Config> = StorageValue<_, Vec<DataCommand>, ValueQuery>;

	#[pallet::storage]
    #[pallet::getter(fn dht_queue)]
	// A list of requests to the DHT.
	pub type DhtQueue<T: Config> = StorageValue<_, Vec<DhtCommand>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn fs_map)]
	// A list of requests to the DHT.
	pub type FsMap<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<u8>, ValueQuery>;

    
	// #[pallet::genesis_config] -> init storage 
    // #[pallet::genesis_build]
	
	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
		ConnectionRequested(T::AccountId),
        DisconnectRequested(T::AccountId),
        QueuedDataToAdd(T::AccountId),
        QueuedDataToCat(T::AccountId),
        QueuedDataToPin(T::AccountId),
        QueuedDataToRemove(T::AccountId),
        QueuedDataToUnpin(T::AccountId),
        FindPeerIssued(T::AccountId),
        FindProvidersIssued(T::AccountId),
        // TODO: should cache this locally, just in case you miss the event being emitted?
        // should add to offchain storage
        /// signer, cid, filename
        NewCID(Option<T::AccountId>, Vec<u8>, Vec<u8>),
        // filedata, filename
        DataReady(Vec<u8>, Vec<u8>),
        // cid, peerids
        ProvidersResult(Vec<u8>, Vec<Vec<u8>>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		CantCreateRequest,
        RequestTimeout,
        RequestFailed,
        OffchainSignedTxError,
        NoLocalAcctForSigning,
	}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {

         // needs to be synchronized with offchain_worker actitivies
         fn on_initialize(block_number: T::BlockNumber) -> Weight {
            <ConnectionQueue<T>>::kill();
            <DhtQueue<T>>::kill();

            if block_number % 2u32.into() == 1u32.into() {
                <DataQueue<T>>::kill();
            }

            0
        }


        fn offchain_worker(block_number: T::BlockNumber) {
            // process connect/disconnect commands
            if let Err(e) = Self::connection_housekeeping() {    
            // Note that having logs compiled to WASM may cause the size of the blob to increase
			// significantly. You can use `RuntimeDebug` custom derive to hide details of the types
			// in WASM. The `sp-api` crate also provides a feature `disable-logging` to disable
			// all logging and thus, remove any logging from the WASM.
			    log::error!("IPFS: Encountered an error during connection housekeeping: {:?}", e);
            }

            // process requests to the DHT
            if let Err(e) = Self::handle_dht_requests() {
                log::error!("IPFS: Encountered an error while processing DHT requests: {:?}", e);
            }

            // process Ipfs::{add, cat, pin, remove block} queues every other block
            if block_number % 3u32.into() == 1u32.into() {
                // TODO: should let user specify a default account
                if let Err(e) = Self::handle_data_requests() {
                    log::error!("IPFS: Encountered an error while processing data requests: {:?}", e);
                }
            }

            // display some stats every 5 blocks
            if block_number % 5u32.into() == 0u32.into() {
                if let Err(e) = Self::print_metadata() {
                    log::error!("IPFS: Encountered an error while obtaining metadata: {:?}", e);
                }
            }
        }
    }

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		 /// Mark a `Multiaddr` as a desired connection target. The connection will be established
        /// during the next run of the off-chain `connection_housekeeping` process.
        #[pallet::weight(0)]
        pub fn ipfs_connect(origin: OriginFor<T>, addr: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let cmd = ConnectionCommand::ConnectTo(OpaqueMultiaddr(addr));
            <ConnectionQueue<T>>::mutate(|cmds| if !cmds.contains(&cmd) { cmds.push(cmd) });
            Self::deposit_event(Event::ConnectionRequested(who));
			Ok(())
        }

        /// Queues a `Multiaddr` to be disconnected. The connection will be severed during the next
        /// run of the off-chain `connection_housekeeping` process.
        #[pallet::weight(0)]
        pub fn ipfs_disconnect(origin: OriginFor<T>, addr: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let cmd = ConnectionCommand::DisconnectFrom(OpaqueMultiaddr(addr));
            <ConnectionQueue<T>>::mutate(|cmds| if !cmds.contains(&cmd) { cmds.push(cmd) });
            Self::deposit_event(Event::DisconnectRequested(who));
			Ok(())
        }

        /// Add bytes and associated data to IPFS.
        #[pallet::weight(0)]
        pub fn ipfs_add_bytes(origin: OriginFor<T>, data: Vec<u8>, name: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <DataQueue<T>>::mutate(|queue| queue.push(DataCommand::AddBytes(data, name)));
            Self::deposit_event(Event::QueuedDataToAdd(who));
			Ok(())
        }

        /// Find IPFS data pointed to by the given `Cid`; if it is valid UTF-8, it is printed in the
        /// logs verbatim; otherwise, the decimal representation of the bytes is displayed instead.
        #[pallet::weight(0)]
        pub fn ipfs_cat_bytes(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <DataQueue<T>>::mutate(|queue| queue.push(DataCommand::CatBytes(cid)));
            Self::deposit_event(Event::QueuedDataToCat(who));
			Ok(())
        }

        /// Add arbitrary bytes to the IPFS repository. The registered `Cid` is printed out in the
        /// logs.
        #[pallet::weight(0)]
        pub fn ipfs_remove_block(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <DataQueue<T>>::mutate(|queue| queue.push(DataCommand::RemoveBlock(cid)));
            Self::deposit_event(Event::QueuedDataToRemove(who));
			Ok(())
        }

        /// Pins a given `Cid` non-recursively.
        #[pallet::weight(0)]
        pub fn ipfs_insert_pin(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <DataQueue<T>>::mutate(|queue| queue.push(DataCommand::InsertPin(cid)));
            Self::deposit_event(Event::QueuedDataToPin(who));
			Ok(())
        }

        /// Unpins a given `Cid` non-recursively.
        #[pallet::weight(0)]
        pub fn ipfs_remove_pin(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <DataQueue<T>>::mutate(|queue| queue.push(DataCommand::RemovePin(cid)));
            Self::deposit_event(Event::QueuedDataToUnpin(who));
			Ok(())
        }

        /// Find addresses associated with the given `PeerId`.
        #[pallet::weight(0)]
        pub fn ipfs_dht_find_peer(origin: OriginFor<T>, peer_id: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <DhtQueue<T>>::mutate(|queue| queue.push(DhtCommand::FindPeer(peer_id)));
            Self::deposit_event(Event::FindPeerIssued(who));
			Ok(())
        }

        /// Find the list of `PeerId`s known to be hosting the given `Cid`.
        #[pallet::weight(0)]
        pub fn ipfs_dht_find_providers(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <DhtQueue<T>>::mutate(|queue| queue.push(DhtCommand::GetProviders(cid)));
            Self::deposit_event(Event::FindProvidersIssued(who));
			Ok(())
        }

        #[pallet::weight(0)]
        pub fn submit_ipfs_results(origin: OriginFor<T>, cid: Vec<u8>, name: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <FsMap::<T>>::insert(cid.clone(), name.clone());
			Self::deposit_event(Event::NewCID(Some(who), cid, name));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn trigger_download_event(origin: OriginFor<T>, data: Vec<u8>, filename: Vec<u8>) -> DispatchResult {
            Self::deposit_event(Event::DataReady(data, filename));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn submit_providers_result(origin: OriginFor<T>, cid: Vec<u8>, providers: Vec<Vec<u8>>) -> DispatchResult {
            Self::deposit_event(Event::ProvidersResult(cid, providers));
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

    fn connection_housekeeping() -> Result<(), Error<T>> {
        let mut deadline;

        for cmd in ConnectionQueue::<T>::get() {
            deadline = Some(timestamp().add(Duration::from_millis(1_000)));

            match cmd {
                // connect to the desired peers if not yet connected
                ConnectionCommand::ConnectTo(addr) => {
                    match Self::ipfs_request(IpfsRequest::Connect(addr.clone()), deadline) {
                        Ok(IpfsResponse::Success) => {
                            log::info!(
                                "IPFS: connected to {}",
                                str::from_utf8(&addr.0).expect("our own calls can be trusted to be UTF-8; qed")
                            );
                        }
                        Ok(_) => unreachable!("only Success can be a response for that request type; qed"),
                        Err(e) => log::error!("IPFS: connect error: {:?}", e),
                    }
                }
                // disconnect from peers that are no longer desired
                ConnectionCommand::DisconnectFrom(addr) => {
                    match Self::ipfs_request(IpfsRequest::Disconnect(addr.clone()), deadline) {
                        Ok(IpfsResponse::Success) => {
                            log::info!(
                                "IPFS: disconnected from {}",
                                str::from_utf8(&addr.0).expect("our own calls can be trusted to be UTF-8; qed")
                            );
                        }
                        Ok(_) => unreachable!("only Success can be a response for that request type; qed"),
                        Err(e) => log::error!("IPFS: disconnect error: {:?}", e),
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_dht_requests() -> Result<(), Error<T>> {
        let mut deadline;

        for cmd in DhtQueue::<T>::get() {
            deadline = Some(timestamp().add(Duration::from_millis(1_000)));

            match cmd {
                // find the known addresses of the given peer
                DhtCommand::FindPeer(peer_id) => {
                    match Self::ipfs_request(IpfsRequest::FindPeer(peer_id.clone()), deadline) {
                        Ok(IpfsResponse::FindPeer(addrs)) => {
                            log::info!(
                                "IPFS: found the following addresses of {}: {:?}",
                                str::from_utf8(&peer_id).expect("our own calls can be trusted to be UTF-8; qed"),
                                addrs.iter()
                                    .map(|addr| str::from_utf8(&addr.0)
                                        .expect("our node's results can be trusted to be UTF-8; qed"))
                                    .collect::<Vec<_>>()
                            );
                        }
                        Ok(_) => unreachable!("only FindPeer can be a response for that request type; qed"),
                        Err(e) => log::error!("IPFS: find peer error: {:?}", e),
                    }
                }
                // find the providers for a given cid
                DhtCommand::GetProviders(cid) => {
                    match Self::ipfs_request(IpfsRequest::GetProviders(cid.clone()), deadline) {
                        Ok(IpfsResponse::GetProviders(peer_ids)) => {
                            log::info!(
                                "IPFS: found the following providers of {}: {:?}",
                                str::from_utf8(&cid).expect("our own calls can be trusted to be UTF-8; qed"),
                                peer_ids.iter()
                                    .map(|peer_id| str::from_utf8(&peer_id)
                                        .expect("our node's results can be trusted to be UTF-8; qed"))
                                    .collect::<Vec<_>>()
                            );
                            let signer = Signer::<T, T::AuthorityId>::all_accounts();
                            if !signer.can_sign() {
                                log::error!("No local account available. Consider adding one via 'author_insertkey' RPC.");
                            }
                            let results = signer.send_signed_transaction(|_acct|
                                // This is the on-chain function
                                Call::submit_providers_result(cid.clone(), peer_ids.clone())
                            );
                            for (_acc, res) in &results {
                                match res {
                                    Ok(()) => log::info!(
                                        "IPFS: Sent signed transaction."
                                    ),
                                    Err(e) => log::error!("No local account available: {:?}", e),
                                }
                            }
                        }
                        Ok(_) => unreachable!("only GetProviders can be a response for that request type; qed"),
                        Err(e) => log::error!("IPFS: find providers error: {:?}", e),
                    }
                }
            }
        }

        Ok(())
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
                DataCommand::AddBytes(data, name) => {
                    match Self::ipfs_request(IpfsRequest::AddBytes(data.clone()), deadline) {
                        Ok(IpfsResponse::AddBytes(cid)) => {
                            log::info!(
                                "IPFS: added data with Cid {}",
                                str::from_utf8(&cid).expect("our own IPFS node can be trusted here; qed")
                            );
                            log::info!("IPFS: added data with name {}",
                                str::from_utf8(&name).expect("our own IPFS node can be trusted here; qed")
                            );
                            let signer = Signer::<T, T::AuthorityId>::all_accounts();
                            if !signer.can_sign() {
                                log::error!("No local account available. Consider adding one via 'author_insertkey' RPC.");
                            }
                            let results = signer.send_signed_transaction(|_acct|
                                // This is the on-chain function
                                Call::submit_ipfs_results(cid.clone(), name.clone())
                            );
                            for (_acc, res) in &results {
                                match res {
                                    Ok(()) => log::info!(
                                        "IPFS: Sent signed transaction."
                                    ),
                                    Err(e) => log::error!("No local account available: {:?}", e),
                                }
                            }
                        },
                        Ok(_) => unreachable!("only AddBytes can be a response for that request type; qed"),
                        Err(e) => log::error!("IPFS: add error: {:?}", e),
                    }
                }
                DataCommand::CatBytes(cid) => {
                    match Self::ipfs_request(IpfsRequest::CatBytes(cid.clone()), deadline) {
                        Ok(IpfsResponse::CatBytes(data)) => {
                            let filename = FsMap::<T>::get(cid);
                            let signer = Signer::<T, T::AuthorityId>::all_accounts();
                            if !signer.can_sign() {
                                log::error!("No local account available. Consider adding one via 'author_insertkey' RPC.");
                            }
                            let results = signer.send_signed_transaction(|_acct|
                                // This is the on-chain function
                                Call::trigger_download_event(data.clone(), filename.clone())
                            );
                            for (_acc, res) in &results {
                                match res {
                                    Ok(()) => log::info!(
                                        "IPFS: Sent signed transaction."
                                    ),
                                    Err(e) => log::error!("No local account available: {:?}", e),
                                }
                            }
                        },
                        Ok(_) => unreachable!("only CatBytes can be a response for that request type; qed"),
                        Err(e) => log::error!("IPFS: error: {:?}", e),
                    }
                }
                DataCommand::RemoveBlock(cid) => {
                    match Self::ipfs_request(IpfsRequest::RemoveBlock(cid), deadline) {
                        Ok(IpfsResponse::RemoveBlock(cid)) => {
                            log::info!(
                                "IPFS: removed a block with Cid {}",
                                str::from_utf8(&cid).expect("our own IPFS node can be trusted here; qed")
                            );
                        },
                        Ok(_) => unreachable!("only RemoveBlock can be a response for that request type; qed"),
                        Err(e) => log::error!("IPFS: remove block error: {:?}", e),
                    }
                }
                DataCommand::InsertPin(cid) => {
                    match Self::ipfs_request(IpfsRequest::InsertPin(cid.clone(), false), deadline) {
                        Ok(IpfsResponse::Success) => {
                            log::info!(
                                "IPFS: pinned data with Cid {}",
                                str::from_utf8(&cid).expect("our own request can be trusted to be UTF-8; qed")
                            );
                        },
                        Ok(_) => unreachable!("only Success can be a response for that request type; qed"),
                        Err(e) => log::error!("IPFS: insert pin error: {:?}", e),
                    }
                }
                DataCommand::RemovePin(cid) => {
                    match Self::ipfs_request(IpfsRequest::RemovePin(cid.clone(), false), deadline) {
                        Ok(IpfsResponse::Success) => {
                            log::info!(
                                "IPFS: unpinned data with Cid {}",
                                str::from_utf8(&cid).expect("our own request can be trusted to be UTF-8; qed")
                            );
                        },
                        Ok(_) => unreachable!("only Success can be a response for that request type; qed"),
                        Err(e) => log::error!("IPFS: remove pin error: {:?}", e),
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
}
