use super::*;
use frame_support::dispatch::DispatchResult;

impl<T: Config> Pallet<T> {
	pub fn set_code(code: Vec<u8>) -> DispatchResult {
        <Pallet<T>>::update_code_in_storage(&code)?;
        Ok(())
	}

    pub fn can_set_code(code: &[u8]) -> Result<(), sp_runtime::DispatchError> {
        // Check whether or not it is possible to update the code.

        Ok(())
    }

    pub fn update_code_in_storage(code: &[u8]) -> DispatchResult {
        // Write code to the storage and emit related events and digest items.
        
        Ok(())
    }
}
