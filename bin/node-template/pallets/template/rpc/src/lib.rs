//! RPC interface for the iris storage pallet.

pub use self::gen_client::Client as IrisClient;
use codec::{Codec, Decode};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use pallet_iris_rpc_runtime_api::IrisApi as IrisRuntimeApi;
use pallet_iris_rpc_runtime_api::{FeeDetails, InclusionFee, RuntimeDispatchInfo};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, MaybeDisplay},
};
use std::{convert::TryInto, sync::Arc};

#[rpc]
pub trait IrisApi<BlockHash, ResponseType> {
	#[rpc(name = "iris_retrieveBytes")]
	fn retrieve_bytes(&self, signed_msg: Bytes) -> Result<ResponseType>;
}

/// A struct that implements the [`IrisApi`].
pub struct Iris<C, P> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> Iris<C, P> {
	/// Create new `Iris` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// The transaction was not decodable.
	DecodeError,
	/// The call to runtime failed.
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
			Error::DecodeError => 2,
		}
	}
}

impl<C, Block, Balance> IrisApi<<Block as BlockT>::Hash, RuntimeDispatchInfo<Balance>>
	for Iris<C, Block>
where
	Block: BlockT,
	C: 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: IrisRuntimeApi<Block, Balance>,
	Balance: Codec + MaybeDisplay + Copy + TryInto<NumberOrHex>,
{
	fn retrieve_bytes(
		&self,
		signed_msg : Bytes,
	) -> Result<RuntimeDispatchInfo<Balance>> {
		// self.client.runtime_api();
		// let api = self.client.runtime_api();
		// // verify signer of the message
		// let at = BlockId::hash(at.unwrap_or_else(||
		// 	// If the block hash is not supplied assume the best block.
		// 	self.client.info().best_hash));

		// let encoded_len = encoded_xt.len() as u32;

		// let uxt: Block::Extrinsic = Decode::decode(&mut &*encoded_xt).map_err(|e| RpcError {
		// 	code: ErrorCode::ServerError(Error::DecodeError.into()),
		// 	message: "Unable to query dispatch info.".into(),
		// 	data: Some(format!("{:?}", e).into()),
		// })?;
		// api.request_bytes(&at, uxt, encoded_len).map_err(|e| RpcError {
		// 	code: ErrorCode::ServerError(Error::RuntimeError.into()),
		// 	message: "Unable to query dispatch info.".into(),
		// 	data: Some(format!("{:?}", e).into()),
		// })
	}

}
