//! Runtime API definition for  cherry pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::Bytes;

// declare the runtime API
// it is implemented in the 'impl' block in the runtime amalgamator file (runtime/src/lib.rs)
sp_api::decl_runtime_apis! {
	pub trait RpcIpfsApi
	{
	fn retrieve_bytes(
		message: Bytes,
	) -> Bytes;
	}
}
