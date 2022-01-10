pub use self::gen_client::Client as CherryClient;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use pallet_cherry_rpc_runtime_api::CherryApi as CherryRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT},
};
use std::sync::Arc;

#[rpc]
pub trait CherryApi<BlockHash> {
	#[rpc(name = "cherry_retrieveBytes")]
	fn retrieve_bytes(
		&self,
		public_key: Bytes,
		signature: Bytes,
		message: Bytes,
		at: Option<BlockHash>,
	) -> Result<Bytes>;
}

/// A struct that implements CherryRpc
pub struct Cherry<C, M> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> Cherry<C, M> {
	/// create new 'Cherry' instance with the given reference	to the client
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

pub enum Error {
	RuntimeError,
	DecodeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
			Error::DecodeError => 2,
		}
	}
}

impl<C, Block> CherryApi<<Block as BlockT>::Hash>
	for Cherry<C, Block>
where 
	Block: BlockT,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: CherryRuntimeApi<Block>,
{
	fn retrieve_bytes(
		&self,
		public_key: Bytes,
		signature: Bytes,
		message: Bytes,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Bytes> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			self.client.info().best_hash
		));
		let runtime_api_result = api.retrieve_bytes(&at, public_key, signature, message);
		runtime_api_result.map_err(|e| RpcError{
			code: ErrorCode::ServerError(Error::DecodeError.into()),
			message: "unable to query runtime api".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}