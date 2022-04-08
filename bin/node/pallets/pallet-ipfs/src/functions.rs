use super::*;
use sp_runtime::offchain::{ipfs, IpfsRequest, IpfsResponse};

impl<T: Config> Pallet<T> {
	pub fn retrieve_bytes(_public_key: Bytes, _signature: Bytes, message: Bytes) -> Bytes {
		let message_vec: Vec<u8> = message.to_vec();
		if let Some(data) =
			sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &message_vec)
		{
			Bytes(data.clone())
		} else {
			Bytes(Vec::new())
		}
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
