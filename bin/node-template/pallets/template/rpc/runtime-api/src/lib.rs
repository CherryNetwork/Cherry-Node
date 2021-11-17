//! Runtime API definition for iris  pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_runtime::traits::MaybeDisplay;

// declare the runtime API
// it is implemented in the 'impl' block in the runtime amalgamator file (runtime/src/lib.rs)
sp_api::decl_runtime_apis! {
	pub trait IrisApi
	{
		fn retrieve_bytes() -> u64;
	}
}
