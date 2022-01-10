use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::traits::{AtLeast32BitUnsigned, Zero};
use sp_std::prelude::*;
use sp_core::Bytes;

use frame_support::weights::{DispatchClass, Weight};

#[derive(Eq, PartialEq, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct DataResponse {
    pub data: Bytes,
}