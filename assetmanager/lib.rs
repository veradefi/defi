#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod assetmanager {
    use administration::Administration;
    use erc20::Erc20;
    use lendingmanager::LendingManager;

    use ink_env::call::FromAccountId;
    use ink_storage::Lazy;
    use scale::{Decode, Encode};

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct AssetManager {
        owner: AccountId,
        paused: bool,
        total_assets: u64,
        administration: Lazy<Administration>,
        lendingmanager: Lazy<LendingManager>,
        erc20: Lazy<Erc20>,
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
        amount: u64,
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
        amount: u64,
    }

    #[ink(event)]
    pub struct Paused {}

    #[ink(event)]
    pub struct Unpaused {}

    impl AssetManager {
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn new(
            administration_code_hash: Hash,
            lendingmanager_code_hash: Hash,
            erc20_address: AccountId,
        ) -> Self {
            let owner = Self::env().caller();
            let total_balance = Self::env().balance();

            let administration = Administration::new(0, 0, false)
                .endowment(total_balance / 3)
                .code_hash(administration_code_hash)
                .instantiate()
                .expect("failed at instantiating the `Administration` contract");

            let lendingmanager = LendingManager::new()
                .endowment(total_balance / 3)
                .code_hash(lendingmanager_code_hash)
                .instantiate()
                .expect("failed at instantiating the `Administration` contract");

            let erc20 = Erc20::from_account_id(erc20_address);
            let instance = Self {
                owner: owner,
                paused: false,
                total_assets: 1,
                administration: Lazy::new(administration),
                lendingmanager: Lazy::new(lendingmanager),
                erc20: Lazy::new(erc20),
            };
            instance
        }

        #[ink(message)]
        pub fn borrow(&mut self, asset: AccountId, amount: u64) -> Result<(), Error> {
            let borrower = self.env().caller();
            let current_time = self.get_current_time();
            let interest_rate = self.administration.get_interest_rate();
            let transfer_rate = self.administration.get_transfer_rate();

            // Validate operation
            self.lendingmanager.handle_borrow(
                asset,
                borrower,
                amount,
                interest_rate,
                transfer_rate,
                current_time,
            );
            let owner = self.env().account_id();
            let erc_amount = amount * transfer_rate;

            self.erc20
                .transfer_from(owner, borrower, Balance::from(erc_amount));

            // TODO: Make ERC721 transfer from borrower based on amount borrowed
            // ERC721.transfer_from(borrower, current_contract, amount)
            self.env().emit_event(Borrowed {
                asset: asset,
                user: borrower,
                amount: amount,
                borrow_rate: 1_0,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn repay(&mut self, asset: AccountId, amount: u64) -> Result<(), Error> {
            let borrower = self.env().caller();
            let current_time = self.get_current_time();

            // Validate operation
            self.lendingmanager
                .handle_repayment(asset, borrower, amount, current_time);

            // TODO: Make ERC721 transfer to borrower based on amount repaid
            // ERC721.transfer_from(current_contract, borrower, erc721_tokens)

            // TODO: Make ERC20 transfer from borrower based on amount repaid
            // ERC721.transfer_from(borrower, current_contract, amount)
            self.env().emit_event(Repaid {
                asset: asset,
                user: borrower,
                amount: amount,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn get_principal_balance(&self, owner: AccountId) -> Balance {
            self.lendingmanager.get_principal_balance(owner)
        }

        #[ink(message)]
        pub fn get_total_balance(&self, owner: AccountId) -> Balance {
            self.lendingmanager
                .get_total_balance(owner, self.get_interest_rate())
        }

        #[ink(message)]
        pub fn get_total_debt(&self, owner: AccountId) -> Balance {
            self.lendingmanager
                .get_total_debt(owner, self.get_interest_rate())
        }

        #[ink(message)]
        pub fn get_interest_rate(&self) -> u64 {
            self.administration.get_interest_rate()
        }

        #[ink(message)]
        pub fn get_transfer_rate(&self) -> u64 {
            self.administration.get_transfer_rate()
        }

        #[ink(message)]
        pub fn is_enabled(&self) -> bool {
            self.administration.is_enabled()
        }

        fn get_current_time(&self) -> u64 {
            self.env().block_timestamp()
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
            let assetmanager =
                AssetManager::new(Hash::default(), Hash::default(), AccountId::default());
            assert_eq!(assetmanager.administration.is_enabled(), true);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn borrow_works() {
            let mut assetmanager =
                AssetManager::new(Hash::default(), Hash::default(), AccountId::default());
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
            let mut assetmanager =
                AssetManager::new(Hash::default(), Hash::default(), AccountId::default());
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
            let mut assetmanager =
                AssetManager::new(Hash::default(), Hash::default(), AccountId::default());
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
