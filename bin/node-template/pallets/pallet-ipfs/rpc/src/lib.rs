pub use self::gen_client::Client as IpfsClient;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use pallet_ipfs_rpc_runtime_api::RpcIpfsApi as IpfsRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait RpcIpfsApi<BlockHash> {
	#[rpc(name = "ipfs_retrieveBytes")]
	fn retrieve_bytes(
		&self,
		public_key: Bytes,
		signature: Bytes,
		message: Bytes,
		at: Option<BlockHash>,
	) -> Result<Bytes>;
}

/// A struct that implements IrisRpc
pub struct RpcIpfs<C, M> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> RpcIpfs<C, M> {
	/// create new 'Iris' instance with the given reference	to the client
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

impl<C, Block> RpcIpfsApi<<Block as BlockT>::Hash> for RpcIpfs<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: IpfsRuntimeApi<Block>,
{
	fn retrieve_bytes(
		&self,
		public_key: Bytes,
		signature: Bytes,
		message: Bytes,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Bytes> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
		let runtime_api_result = api.retrieve_bytes(&at, public_key, signature, message);
		runtime_api_result.map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::DecodeError.into()),
			message: "unable to query runtime api".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
