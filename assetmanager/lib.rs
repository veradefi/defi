#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod assetmanager {
    use administration::Administration;
    use ink_storage::collections::HashMap as StorageHashMap;
    use ink_storage::{
        traits::{PackedLayout, SpreadLayout, StorageLayout},
        Lazy,
    };
    use scale::{Decode, Encode};

    #[derive(Clone, Default, scale::Encode, scale::Decode, Debug, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
    struct Borrower {
        balance: Balance,
        date_borrowed: Option<u64>,
        date_repaid: Option<u64>,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct AssetManager {
        owner: AccountId,
        paused: bool,
        total_assets: u64,
        borrowers: StorageHashMap<AccountId, Borrower>,
        administration: Lazy<Administration>,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotOwner,
        TokenNotFound,
        NotAllowed,
    }

    #[ink(event)]
    pub struct Borrowed {
        #[ink(topic)]
        asset: AccountId,
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        amount: Balance,
        #[ink(topic)]
        borrow_rate: u64,
    }

    #[ink(event)]
    pub struct Repaid {
        #[ink(topic)]
        asset: AccountId,
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    #[ink(event)]
    pub struct Paused {}

    #[ink(event)]
    pub struct Unpaused {}

    impl AssetManager {
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn new(administration_code_hash: Hash) -> Self {
            let owner = Self::env().caller();
            let total_balance = Self::env().balance();

            let administration = Administration::new(0, 0, false)
                .endowment(total_balance / 2)
                .code_hash(administration_code_hash)
                .instantiate()
                .expect("failed at instantiating the `Administration` contract");

            let instance = Self {
                owner: owner,
                paused: false,
                total_assets: 1,
                borrowers: Default::default(),
                administration: Lazy::new(administration),
            };
            instance
        }

        #[ink(message)]
        pub fn borrow(&mut self, asset: AccountId, amount: Balance) -> Result<(), Error> {
            let owner = self.env().caller();
            let current_time = self.get_current_time();
            let borrower_opt = self.borrowers.get(&owner);
            assert_eq!(borrower_opt.is_some(), false, "Has already borrowed");

            self.borrowers.insert(
                owner,
                Borrower {
                    balance: amount,
                    date_borrowed: Some(current_time),
                    date_repaid: None,
                },
            );

            self.env().emit_event(Borrowed {
                asset: asset,
                user: owner,
                amount: amount,
                borrow_rate: 1_0,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn repay(&mut self, asset: AccountId, amount: Balance) -> Result<(), Error> {
            let owner = self.env().caller();
            let current_time = self.get_current_time();

            let borrower_opt = self.borrowers.get_mut(&owner);
            assert_eq!(borrower_opt.is_some(), true, "Borrower does not exist");

            let borrower = borrower_opt.unwrap();
            borrower.balance -= amount;
            borrower.date_repaid = Some(current_time);

            self.env().emit_event(Repaid {
                asset: asset,
                user: owner,
                amount: amount,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn get_principal_balance(&self, owner: AccountId) -> Balance {
            self.borrowers
                .get(&owner)
                .unwrap_or(&Borrower {
                    balance: 0,
                    date_borrowed: None,
                    date_repaid: None,
                })
                .balance
        }

        #[ink(message)]
        pub fn get_total_balance(&self, owner: AccountId) -> Balance {
            let balance = self.get_principal_balance(owner);
            let debt = self.get_total_debt(owner);
            balance + debt
        }

        #[ink(message)]
        pub fn get_total_debt(&self, owner: AccountId) -> Balance {
            let borrower = self.borrowers.get(&owner).unwrap_or(&Borrower {
                balance: 0,
                date_borrowed: None,
                date_repaid: None,
            });
            let interest = self.calculate_interest(
                10,
                self.administration.get_interest_rate(),
                borrower.date_borrowed.unwrap_or(0),
            );
            Balance::from(interest)
        }

        fn get_current_time(&self) -> u64 {
            self.env().block_timestamp()
        }

        fn calculate_interest(&self, amount: u64, rate: u64, timestamp: u64) -> u64 {
            let ct: u64 = self.env().block_timestamp();
            let exp: u64 = ct - timestamp;

            let interest: u64 = amount * rate * exp / 3_153_6000;
            interest
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;
        use ink_lang as ink;
        /// We test if the constructor does its job.
        #[ink::test]
        fn new_works() {
            let assetmanager = AssetManager::new(Hash::default());
            assert_eq!(assetmanager.administration.is_enabled(), true);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn borrow_works() {
            let mut assetmanager = AssetManager::new(Hash::default());
            assert_eq!(assetmanager.administration.is_enabled(), true);

            let asset = AccountId::from([0x05; 32]);
            let owner = AccountId::from([0x01; 32]);

            assetmanager.borrow(asset, 1);

            // Borrowed event triggered
            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(1, emitted_events.len());

            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 1);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn repay_works() {
            let mut assetmanager = AssetManager::new(Hash::default());
            assert_eq!(assetmanager.administration.is_enabled(), true);

            let asset = AccountId::from([0x05; 32]);
            let owner = AccountId::from([0x01; 32]);

            assetmanager.borrow(asset, 5);

            assetmanager.repay(asset, 2);
            // Borrow and Repay events triggered
            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(2, emitted_events.len());

            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 3);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn get_principal_balance_works() {
            let mut assetmanager = AssetManager::new(Hash::default());
            assert_eq!(assetmanager.administration.is_enabled(), true);

            let asset = AccountId::from([0x05; 32]);
            let owner = AccountId::from([0x01; 32]);
            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 0);

            assetmanager.borrow(asset, 2);

            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 2);

            assetmanager.repay(asset, 1);

            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 1);

            assetmanager.repay(asset, 1);

            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 0);
        }

        // /// We test a simple use case of our contract.
        // #[ink::test]
        // fn get_total_debt_works() {
        //     let assetmanager = AssetManager::new();
        //     assert_eq!(assetmanager.config.enabled, true);

        //     let owner = AccountId::from([0x01; 32]);
        //     let balance = assetmanager.get_total_debt(owner);
        //     assert_eq!(balance, 0);
        // }
    }
}
