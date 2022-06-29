use super::*;
use frame_support::{dispatch::DispatchResult, ensure, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::offchain::{ipfs, IpfsRequest, IpfsResponse};
use sp_std::collections::btree_set::BTreeSet;

impl<T: Config> Pallet<T> {
	pub fn retrieve_bytes(message: Bytes) -> Bytes {
		let message_vec: Vec<u8> = message.to_vec();
		if let Some(data) =
			sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &message_vec)
		{
			Bytes(data.clone())
		} else {
			Bytes(Vec::new())
		}
	}

	pub fn initialize_validators(validators: &[T::AccountId]) {
		assert!(validators.len() > 1, "At least 2 validators should be initialized.");
		assert!(<Validators<T>>::get().is_empty(), "Validators are already initialized.");
		<Validators<T>>::put(validators);
		<ApprovedValidators<T>>::put(validators);
	}

	pub fn approve_validator(validator_id: T::AccountId) -> DispatchResult {
		let approved_set: BTreeSet<_> = <ApprovedValidators<T>>::get().into_iter().collect();

		ensure!(!approved_set.contains(&validator_id), Error::<T>::Duplicate);
		<ApprovedValidators<T>>::mutate(|v| v.push(validator_id.clone()));
		Ok(())
	}

	pub fn do_add_validator(validator_id: T::AccountId) -> DispatchResult {
		let validator_set: BTreeSet<_> = <Validators<T>>::get().into_iter().collect();
		ensure!(!validator_set.contains(&validator_id), Error::<T>::Duplicate);
		<Validators<T>>::mutate(|v| v.push(validator_id.clone()));
		UnproductiveSessions::<T>::mutate(validator_id.clone(), |v| {
			*v = 0;
		});

		Self::deposit_event(Event::ValidatorAdditionInitiated(validator_id.clone()));
		log::debug!(target: LOG_TARGET, "Validator addition initiated.");

		Ok(())
	}

	pub fn add_approve_validator(acct: &T::AccountId) {
		if !Self::approved_validators().contains(acct) {
			ApprovedValidators::<T>::mutate(|approve_validators| {
				approve_validators.push(acct.clone())
			});
		} else {
			log::info!("The AccountId {:?} is already an approved Validator", acct)
		}
	}

	pub fn mark_for_removal(validator_id: T::AccountId) {
		<OfflineValidators<T>>::mutate(|v| v.push(validator_id));
		log::debug!(target: LOG_TARGET, "Offline validator marked for auto removal.");
	}

	pub fn remove_validator(acct: &T::AccountId) {
		if Self::validators().contains(acct) {
			Validators::<T>::mutate(|validators| validators.retain(|who| who != acct))
		} else {
			log::info!("The AccountId {:?} is not in the list of Validators", acct)
		}
	}

	pub fn remove_approve_validator(acct: &T::AccountId) {
		if Self::approved_validators().contains(acct) {
			ApprovedValidators::<T>::mutate(|approved_validators| {
				approved_validators.retain(|who| who != acct)
			})
		} else {
			log::info!("The AccountId {:?} is not in the list of approved Validators", acct)
		}
	}

	pub fn mark_offline_validator(acct: &T::AccountId) {
		OfflineValidators::<T>::mutate(|offline_validators| offline_validators.push(acct.clone()));
		log::info!("The AccountId {:?} was offline and is marked to auto-remove", acct)
	}

	pub fn mark_dead_validators(era_index: EraIndex) {
		let participating_validators = SessionParticipation::<T>::get(era_index.clone());
		for acct in Validators::<T>::get() {
			if !participating_validators.contains(&acct) {
				if UnproductiveSessions::<T>::get(acct.clone()) <= T::MaxDeadSessions::get() {
					UnproductiveSessions::<T>::mutate(acct.clone(), |v| {
						*v += 1;
					});
				} else {
					let mut validators = Validators::<T>::get();

					validators.retain(|v| *v != acct.clone());
					Validators::<T>::put(validators);

					log::info!("Validator removal just happened.");
				}
			}
		}
	}

	pub fn remove_offline_validators() {
		let bad_validators = OfflineValidators::<T>::get();
		Validators::<T>::mutate(|validators| {
			validators.retain(|good_validators| !bad_validators.contains(good_validators))
		});
		log::info!("removing the offline validators: {:?}", bad_validators);

		<OfflineValidators<T>>::put(Vec::<T::AccountId>::new());
	}

	pub fn determine_account_ownership_layer(
		cid: &Vec<u8>,
		acct: &T::AccountId,
	) -> Result<OwnershipLayer, Error<T>> {
		match Self::ipfs_asset(cid) {
			Some(ipfs) =>
				if let Some(layer) = ipfs.owners.get_key_value(acct) {
					Ok(layer.1.clone())
				} else {
					Err(<Error<T>>::AccNotExist)
				},
			None => Err(<Error<T>>::IpfsNotExist),
		}
	}

	pub fn ipfs_request(
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
					log::error!("IPFS Request failed: {}", sp_std::str::from_utf8(&err).unwrap());
				} else {
					log::error!("IPFS Request failed: {:?}", e);
				}
				Error::<T>::RequestFailed
			})
	}

	pub fn handle_data_requests() -> Result<(), Error<T>> {
		let data_queue = DataQueue::<T>::get();
		let len = data_queue.len();

		if len != 0 {
			log::info!("IPFS: {} entries in the data queue", len);
		}

		let deadline = Some(timestamp().add(Duration::from_millis(5_000)));

		for cmd in data_queue.into_iter() {
			match cmd {
				DataCommand::AddBytes(m_addr, cid, size, extra_lifetime, admin, is_recursive) => {
					// this should work for different CID's. If you try to
					// connect and upload the same CID, you will get a duplicate
					// conn error. @charmitro
					match Self::ipfs_request(IpfsRequest::Connect(m_addr.clone()), deadline) {
						Ok(IpfsResponse::Success) => {
							match Self::ipfs_request(IpfsRequest::CatBytes(cid.clone()), deadline) {
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
														// admin should be the actual account that
														// we is doing the transcation in the first
														// place(create_ipfs_asset)
														admin: admin.clone(),
														cid: cid.clone(),
														extra_lifetime,
														size: size.clone(),
													}
												});

											for (_, res) in &results {
												match res {
													Ok(()) => {
														// also this probably doesn't work.
														log::info!("Submited IPFS results")
													},
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
												},
												Ok(_) => {
													unreachable!("only Success can be a response for that request type")
												},
												Err(e) => log::error!("IPFS: Pin Error: {:?}", e),
											}

											match Self::ipfs_request(
												IpfsRequest::Disconnect(m_addr.clone()),
												deadline,
											) {
												Ok(IpfsResponse::Success) => {
													log::info!("IPFS: Disconeccted Succes")
												},
												Ok(_) => {
													unreachable!("only Success can be a response for that request type")
												},
												Err(e) => {
													log::error!("IPFS: Disconnect Error: {:?}", e)
												},
											}
										},
										Ok(_) => unreachable!(
											"only AddBytes can be a response for that request type"
										),
										Err(e) => log::error!("IPFS: Add Error: {:?}", e),
									}
								},
								Ok(_) => unreachable!(
									"only AddBytes can be a response for that request type."
								),
								Err(e) => log::error!("IPFS: add error: {:?}", e),
							}
						},
						Ok(_) => {
							unreachable!("only AddBytes can be a response for that request type.")
						},
						Err(e) => log::error!("IPFS: add error: {:?}", e),
					}
				},
				DataCommand::AddBytesRaw(m_addr, data, admin, is_recursive) => {
					match Self::ipfs_request(IpfsRequest::Connect(m_addr.clone()), deadline) {
						Ok(IpfsResponse::Success) => {
							match Self::ipfs_request(IpfsRequest::AddBytes(data.clone()), deadline)
							{
								Ok(IpfsResponse::AddBytes(cid)) => {
									log::info!("IPFS: added data");
									Self::ipfs_request(
										IpfsRequest::Disconnect(m_addr.clone()),
										deadline,
									)?;

									// signer is the probably the node account (often Alice)
									let signer = Signer::<T, T::AuthorityId>::all_accounts();
									if !signer.can_sign() {
										log::error!(
											"No local account available. Consider adding one via `author_insertKey` RPC.",
										);
									}

									let results = signer.send_signed_transaction(|_account| {
										Call::submit_ipfs_add_results {
											// admin should be the actual account that we is doing
											// the transcation in the first place(create_ipfs_asset)
											admin: admin.clone(),
											cid: cid.clone(),
											extra_lifetime: 0,
											size: data.len() as u64,
										}
									});

									for (_, res) in &results {
										match res {
											Ok(()) => {
												// also this probably doesn't work.
												log::info!("Submited IPFS results")
											},
											Err(e) => {
												log::error!("Failed to submit transaction: {:?}", e)
											},
										}
									}

									match Self::ipfs_request(
										IpfsRequest::InsertPin(cid.clone(), is_recursive),
										deadline,
									) {
										Ok(IpfsResponse::Success) => {
											log::info!(
												"IPFS: pinned data with CID: {}",
												sp_std::str::from_utf8(&cid).expect("trusted")
											)
										},
										Ok(_) => {
											unreachable!("only Success can be a response for that request type")
										},
										Err(e) => log::error!("IPFS: Pin Error: {:?}", e),
									}

									match Self::ipfs_request(
										IpfsRequest::Disconnect(m_addr.clone()),
										deadline,
									) {
										Ok(IpfsResponse::Success) => {
											log::info!("IPFS: Disconeccted Succes")
										},
										Ok(_) => {
											unreachable!("only Success can be a response for that request type")
										},
										Err(e) => {
											log::error!("IPFS: Disconnect Error: {:?}", e)
										},
									}
								},
								Ok(_) => unreachable!(
									"only AddBytes can be a response for that request type."
								),
								Err(e) => log::error!("IPFS: add error: {:?}", e),
							}
						},
						Ok(_) => {
							unreachable!("only AddBytes can be a response for that request type.")
						},
						Err(e) => log::error!("IPFS: add error: {:?}", e),
					}
				},

				DataCommand::CatBytes(m_addr, cid, _admin) => {
					match Self::ipfs_request(IpfsRequest::CatBytes(cid.clone()), deadline) {
						Ok(IpfsResponse::CatBytes(_data)) => {
							log::info!("IPFS: fetched data");
							Self::ipfs_request(IpfsRequest::Disconnect(m_addr.clone()), deadline)?;

							log::info!(
								"IPFS: disconnected from {}",
								sp_std::str::from_utf8(&m_addr.0)
									.expect("our own calls can be trusted to be UTF-8; qed")
							);
						},
						Ok(_) => {
							unreachable!("only AddBytes can be a response for that request type.")
						},
						Err(e) => log::error!("IPFS: add error: {:?}", e),
					}
				},

				DataCommand::InsertPin(_m_addr, cid, _admin, is_recursive) =>
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
								Call::submit_ipfs_pin_results { cid: cid.clone() }
							});

							for (_, res) in &results {
								match res {
									Ok(()) => {
										log::info!("Submited IPFS results")
									},
									Err(e) => {
										log::error!("Failed to submit transaction: {:?}", e)
									},
								}
							}
						},
						Ok(_) => {
							unreachable!("only Success can be a response for that request type")
						},
						Err(e) => log::error!("IPFS: Pin Error: {:?}", e),
					},

				DataCommand::RemovePin(_m_addr, cid, _admin, is_recursive) =>
					match Self::ipfs_request(
						IpfsRequest::RemovePin(cid.clone(), is_recursive),
						deadline,
					) {
						Ok(IpfsResponse::Success) => {
							log::info!(
								"IPFS: unpinned data with CID: {:?}",
								sp_std::str::from_utf8(&cid).expect("qrff")
							);

							let signer = Signer::<T, T::AuthorityId>::all_accounts();
							if !signer.can_sign() {
								log::error!(
									"No local account available. Consider adding one via `author_insertKey` RPC",
								);
							}

							let results = signer.send_signed_transaction(|_account| {
								Call::submit_ipfs_unpin_results { cid: cid.clone() }
							});

							for (_, res) in &results {
								match res {
									Ok(()) => {
										log::info!("Submited IPFS results")
									},
									Err(e) => {
										log::error!("Failed to submit transaction: {:?}", e)
									},
								}
							}
						},
						Ok(_) => {
							unreachable!("only Success can be a response for that request type")
						},
						Err(e) => log::error!("IPFS: Remove Pin Error: {:?}", e),
					},

				DataCommand::RemoveBlock(_m_addr, cid, _admin) => {
					match Self::ipfs_request(IpfsRequest::RemoveBlock(cid.clone()), deadline) {
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
								Call::submit_ipfs_delete_results { cid: cid.clone() }
							});

							for (_, res) in &results {
								match res {
									Ok(()) => {
										log::info!("Submited IPFS results")
									},
									Err(e) => {
										log::error!("Failed to submit transaction: {:?}", e)
									},
								}
							}
						},
						Ok(_) => {
							unreachable!("only RemoveBlock can be a response for that request type")
						},
						Err(e) => log::error!("IPFS: Remove Block Error: {:?}", e),
					}
				},
			}
		}
		Ok(())
	}

	/// IPFSNodes housekeeping TODO: this need to cleanup later - @charmitro
	pub fn ipfs_nodes_housekeeping() -> Result<(), Error<T>> {
		let deadline = Some(timestamp().add(Duration::from_millis(5_0000)));

		let (public_key, addrs) = if let IpfsResponse::Identity(public_key, addrs) =
			Self::ipfs_request(IpfsRequest::Identity, deadline)?
		{
			(public_key, addrs)
		} else {
			unreachable!("only `Identity` is a valid response type.");
		};

		if !IPFSNodes::<T>::contains_key(public_key.clone()) {
			if let Some(ipfs_node) = &IPFSNodes::<T>::iter().nth(0) {
				if let Some(ipfs_maddr) = ipfs_node.1.clone().pop() {
					if let IpfsResponse::Success =
						Self::ipfs_request(IpfsRequest::Connect(ipfs_maddr.clone()), deadline)?
					{
						log::info!("Succesfully connected to ipfs node: {:?}", &ipfs_node.0);
					} else {
						log::info!(
							"Failed t oconnect to the ipfs_node with multiaddress: {:?}",
							&ipfs_node.0
						);

						if let Some(next_ipfs_maddr) = ipfs_node.1.clone().pop() {
							if let IpfsResponse::Success = Self::ipfs_request(
								IpfsRequest::Connect(next_ipfs_maddr.clone()),
								deadline,
							)? {
								log::info!(
									"Succesfully connected to ipfs node: {:?}",
									&next_ipfs_maddr.0
								);
							} else {
								log::info!(
									"Failed t oconnect to the ipfs_node with multiaddress: {:?}",
									&next_ipfs_maddr.0,
								)
							}
						}
					}
				}
			}

			let signer = Signer::<T, T::AuthorityId>::all_accounts();
			if !signer.can_sign() {
				log::error!("No local accounts available. Consider adding one via `author_insertKey` RPC method.");
			}

			let results = signer.send_signed_transaction(|_account| Call::submit_ipfs_identity {
				public_key: public_key.clone(),
				multiaddress: addrs.clone(),
			});

			for (_, res) in &results {
				match res {
					Ok(()) => log::info!("Submitted ipfs identity results"),
					Err(e) => log::error!("Failed to submit tx: {:?}", e),
				}
			}
		}

		Ok(())
	}

	/// IPFS gargabe collector
	// Housekeeping is for us to delete `ipfs_assets` according to their
	// `deleting_at` attritube.
	// Needs to run everyblock via `#[pallet::hooks]`
	pub fn ipfs_garbage_collector(block_no: BlockNumberFor<T>) -> DispatchResult {
		for mut ipfs_asset in IpfsAsset::<T>::iter() {
			let deleting_at = ipfs_asset.1.deleting_at;
			if block_no.eq(&deleting_at) {
				if ipfs_asset.1.pinned {
					ipfs_asset.1.pinned = false;
				}

				let signer = Signer::<T, T::AuthorityId>::all_accounts();
				if !signer.can_sign() {
					log::error!(
						"No local account available. Consider adding one via `author_insertKey` RPC",
					);
				}

				let results = signer.send_signed_transaction(|_account| {
					Call::submit_ipfs_delete_results { cid: ipfs_asset.1.cid.clone() }
				});

				for (_, res) in &results {
					match res {
						Ok(()) => {
							log::info!("Submited IPFS results")
						},
						Err(e) => {
							log::error!("Failed to submit transaction: {:?}", e)
						},
					}
				}
			}
		}

		Ok(())
	}

	pub fn print_metadata() -> Result<(), Error<T>> {
		let deadline = Some(timestamp().add(Duration::from_millis(5_000)));

		let peers =
			if let IpfsResponse::Peers(peers) = Self::ipfs_request(IpfsRequest::Peers, deadline)? {
				peers
			} else {
				unreachable!("only Peers can be a response for that request type: qed");
			};

		let peer_count = peers.len();

		log::info!("IPFS: currently connencted to {} peers", &peer_count,);

		Ok(())
	}
}
