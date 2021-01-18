#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod adminstration {

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Adminstration {
        interest_rate: u64,
        transfer_rate: u64,
        enabled: bool,
    }

    impl Adminstration {
        #[ink(constructor)]
        pub fn new(_interest_rate: u64, _transfer_rate: u64, _enabled: bool) -> Self {
            Self {
                interest_rate: _interest_rate,
                transfer_rate: _transfer_rate,
                enabled: _enabled,
            }
        }

        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default(), Default::default(), Default::default())
        }

        #[ink(message)]
        pub fn set_interest_rate(&mut self, _interest_rate: u64) {
            self.interest_rate = _interest_rate;
        }

        #[ink(message)]
        pub fn get_interest_rate(&self) -> u64{
            self.interest_rate
        }

        #[ink(message)]
        pub fn set_transfer_rate(&mut self, _transfer_rate: u64) {
            self.transfer_rate = _transfer_rate;
        }

        #[ink(message)]
        pub fn get_transfer_rate(&self) -> u64{
            self.transfer_rate
        }

        #[ink(message)]
        pub fn enable(&mut self) {
            self.enabled = true;
        }

        #[ink(message)]
        pub fn disable(&mut self) {
            self.enabled = false;
        }

        /// Simply returns the current value of our `bool`.
        #[ink(message)]
        pub fn is_enabled(&self) -> bool {
            self.enabled
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[test]
        fn default_works() {
            let adminstration = Adminstration::default();
            assert_eq!(adminstration.is_enabled(), false);
            assert_eq!(adminstration.get_interest_rate(), 0_0);
            assert_eq!(adminstration.get_transfer_rate(), 0_0);
        }

        /// We test a simple use case of our contract.
        #[test]
        fn enable_works() {
            let mut adminstration = Adminstration::new(7_0, 100_0, false);
            assert_eq!(adminstration.is_enabled(), false);
            assert_eq!(adminstration.get_interest_rate(), 7_0);
            assert_eq!(adminstration.get_transfer_rate(), 100_0);

            adminstration.enable();
            assert_eq!(adminstration.is_enabled(), true);
        }

        /// We test a simple use case of our contract.
        #[test]
        fn disable_works() {
            let mut adminstration = Adminstration::new(7_0, 100_0, true);
            assert_eq!(adminstration.is_enabled(), true);
            assert_eq!(adminstration.get_interest_rate(), 7_0);
            assert_eq!(adminstration.get_transfer_rate(), 100_0);

            adminstration.disable();
            assert_eq!(adminstration.is_enabled(), false);
        }

        /// We test a simple use case of our contract.
        #[test]
        fn set_interest_rate_works() {
            let mut adminstration = Adminstration::new(7_0, 100_0, true);
            assert_eq!(adminstration.is_enabled(), true);
            assert_eq!(adminstration.get_interest_rate(), 7_0);
            assert_eq!(adminstration.get_transfer_rate(), 100_0);

            adminstration.set_interest_rate(8_0);
            assert_eq!(adminstration.get_interest_rate(), 8_0);
        }

        /// We test a simple use case of our contract.
        #[test]
        fn set_transfer_rate_works() {
            let mut adminstration = Adminstration::new(7_0, 100_0, true);
            assert_eq!(adminstration.is_enabled(), true);
            assert_eq!(adminstration.get_interest_rate(), 7_0);
            assert_eq!(adminstration.get_transfer_rate(), 100_0);

            adminstration.set_transfer_rate(110_0);
            assert_eq!(adminstration.get_transfer_rate(), 110_0);
        }
    }
}
