//! A high-level helpers for making IPFS requests from Offchain Workers.
//!
//! `sp-io` crate exposes a low level methods to make and control IPFS requests
//! available only for Offchain Workers. Those might be hard to use
//! and usually that level of control is not really necessary.
//! This module aims to provide high-level wrappers for those APIs
//! to simplify making IPFS requests.
//!
//!
//! Example:
//! ```rust,no_run
//! use sp_core::offchain::{IpfsRequest, IpfsResponse};
//! use sp_runtime::offchain::ipfs::{PendingRequest, Response};
//!
//! // initiate a `Peers` request
//! let pending = PendingRequest::new(IpfsRequest::Peers).unwrap();
//!
//! // wait for the response indefinitely
//! let mut response = pending.wait().unwrap();
//!
//! // then check the current peers
//! let peers = if let IpfsResponse::Peers(peers) = response.response {
//! 	 peers
//! } else {
//! 	 unreachable!();
//! };
//!
//! assert!(peers.is_empty());
//! ```

use sp_core::{
	offchain::{
		IpfsError, IpfsRequest, IpfsRequestId as RequestId, IpfsRequestStatus as RequestStatus,
		IpfsResponse, Timestamp,
	},
	RuntimeDebug,
};
#[cfg(not(feature = "std"))]
use sp_std::prelude::vec;
use sp_std::prelude::Vec;

/// A struct representing an uncompleted IPFS request.
#[derive(PartialEq, Eq, RuntimeDebug)]
pub struct PendingRequest {
	/// Request ID
	pub id: RequestId,
	/// Request type
	pub request: IpfsRequest,
}

impl PendingRequest {
	/// Creates amd starts a specified request for the IPFS node.
	pub fn new(request: IpfsRequest) -> Result<Self, IpfsError> {
		let id =
			sp_io::offchain::ipfs_request_start(request.clone()).map_err(|_| IpfsError::IoError)?;

		Ok(PendingRequest { id, request })
	}
}

/// A request error
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub enum Error {
	/// Deadline has been reached.
	DeadlineReached,
	/// Request had timed out.
	IoError(Vec<u8>),
	/// Unknown error has been encountered.
	Unknown,
}

/// A result of waiting for a pending request.
pub type IpfsResult = Result<Response, Error>;

impl PendingRequest {
	/// Wait for the request to complete.
	///
	/// NOTE this waits for the request indefinitely.
	pub fn wait(self) -> IpfsResult {
		match self.try_wait(None) {
			Ok(res) => res,
			Err(_) => panic!("Since `None` is passed we will never get a deadline error; qed"),
		}
	}

	/// Attempts to wait for the request to finish,
	/// but will return `Err` in case the deadline is reached.
	pub fn try_wait(
		self,
		deadline: impl Into<Option<Timestamp>>,
	) -> Result<IpfsResult, PendingRequest> {
		Self::try_wait_all(vec![self], deadline)
			.pop()
			.expect("One request passed, one status received; qed")
	}

	/// Wait for all provided requests.
	pub fn wait_all(requests: Vec<PendingRequest>) -> Vec<IpfsResult> {
		Self::try_wait_all(requests, None)
			.into_iter()
			.map(|r| match r {
				Ok(r) => r,
				Err(_) => panic!("Since `None` is passed we will never get a deadline error; qed"),
			})
			.collect()
	}

	/// Attempt to wait for all provided requests, but up to given deadline.
	///
	/// Requests that are complete will resolve to an `Ok` others will return a `DeadlineReached`
	/// error.
	pub fn try_wait_all(
		requests: Vec<PendingRequest>,
		deadline: impl Into<Option<Timestamp>>,
	) -> Vec<Result<IpfsResult, PendingRequest>> {
		let ids = requests.iter().map(|r| r.id).collect::<Vec<_>>();
		let statuses = sp_io::offchain::ipfs_response_wait(&ids, deadline.into());

		statuses
			.into_iter()
			.zip(requests.into_iter())
			.map(|(status, req)| match status {
				RequestStatus::DeadlineReached => Err(req),
				RequestStatus::IoError(e) => Ok(Err(Error::IoError(e))),
				RequestStatus::Invalid => Ok(Err(Error::Unknown)),
				RequestStatus::Finished(resp) => Ok(Ok(Response::new(req.id, resp))),
			})
			.collect()
	}
}

/// An IPFS response.
#[derive(RuntimeDebug)]
pub struct Response {
	/// Request id
	pub id: RequestId,
	/// Response value
	pub response: IpfsResponse,
}

impl Response {
	fn new(id: RequestId, response: IpfsResponse) -> Self {
		Self { id, response }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_core::offchain::{testing, OffchainDbExt};
	use sp_io::TestExternalities;

	#[test]
	fn basic_metadata_request_and_response() {
		let (offchain, _state) = testing::TestOffchainExt::new();
		let mut t = TestExternalities::default();
		t.register_extension(OffchainDbExt::new(offchain));

		t.execute_with(|| {
			let identity_request = PendingRequest::new(IpfsRequest::Identity).unwrap();
			let identity_response = identity_request.wait().unwrap();
			let local_refs_request = PendingRequest::new(IpfsRequest::LocalRefs).unwrap();
			let local_refs_response = local_refs_request.wait().unwrap();

			assert!(matches!(identity_response, Response { .. }));
			assert!(matches!(local_refs_response, Response { .. }));
		})
	}
}
