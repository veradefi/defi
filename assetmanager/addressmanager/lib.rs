#![cfg_attr(not(feature = "std"), no_std)]
pub use self::addressmanager::AddressManager;
use ink_lang as ink;

#[ink::contract]
pub mod addressmanager {

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct AddressManager {
        erc20_address: AccountId,
    }

    impl AddressManager {
        #[ink(constructor)]
        pub fn new(erc20_address: AccountId) -> Self {
            Self {
                erc20_address: erc20_address,
            }
        }

        #[ink(message)]
        pub fn get_erc20_address(&self) -> AccountId {
            self.erc20_address
        }
    }
}
