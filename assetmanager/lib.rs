#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod assetmanager {
    use erc20::Erc20;
    use erc721::Erc721;
    use ink_env::call::FromAccountId;
    use ink_prelude::vec::Vec;
    use ink_storage::{
        collections::HashMap as StorageHashMap,
        traits::{PackedLayout, SpreadLayout, StorageLayout},
        Lazy,
    };
    use scale::{Decode, Encode};
    #[derive(Encode, Decode, Debug, Default, Copy, Clone, SpreadLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    pub struct Administration {
        interest_rate: u64,
        transfer_rate: u64,
        enabled: bool,
    }

    pub type LoanId = u64;
    pub type TokenId = u32;

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NoSuchLoan,
        ERC721TransferFailed,
        ERC20TransferFailed,
    }

    #[derive(Clone, Default, Encode, Decode, Debug, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
    pub struct Borrower {
        balance: Balance,
        last_updated_at: u64,
        loans: Vec<TokenId>,
    }

    #[derive(Clone, Default, Copy, Encode, Decode, Debug, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
    pub struct Loan {
        id: LoanId,
        amount: Balance,
        transfer_rate: u64,
        interest_rate: u64,
        date_borrowed: u64,
        date_repaid: Option<u64>,
        is_repaid: bool,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct AssetManager {
        owner: AccountId,
        borrowers: StorageHashMap<AccountId, Borrower>,
        loans: StorageHashMap<(AccountId, TokenId), Loan>,
        administration: Administration,
        total_loans: u64,
        erc20: Lazy<Erc20>,
        erc721: Lazy<Erc721>,
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
        pub fn new(erc20_address: AccountId, erc721_address: AccountId) -> Self {
            let owner = Self::env().caller();

            let erc20 = Erc20::from_account_id(erc20_address);
            let erc721 = Erc721::from_account_id(erc721_address);
            let instance = Self {
                owner: owner,
                administration: Administration {
                    interest_rate: 10,
                    transfer_rate: 100,
                    enabled: true,
                },
                borrowers: Default::default(),
                loans: Default::default(),
                total_loans: 0,
                erc20: Lazy::new(erc20),
                erc721: Lazy::new(erc721),
            };
            instance
        }

        #[ink(message)]
        pub fn borrow(&mut self, token_id: u32) -> Result<(), Error> {
            let current_time = self.get_current_time();
            let caller = self.env().caller();
            let interest_rate = self.get_interest_rate();
            let transfer_rate = self.get_transfer_rate();

            // Validate operation
            match self.handle_borrow(caller, token_id, interest_rate, transfer_rate, current_time) {
                Err(e) => return Err(e),
                Ok(_) => {}
            };

            let owner = self.env().account_id();
            let erc20_amount = Balance::from(transfer_rate);
            match self.erc721.transfer_from(caller, owner, token_id) {
                Err(_) => return Err(Error::ERC721TransferFailed),
                Ok(_) => {}
            };

            match self.erc20.transfer(caller, erc20_amount) {
                Err(_) => return Err(Error::ERC20TransferFailed),
                Ok(_) => {}
            };

            // self.env().emit_event(Borrowed {
            //     asset: asset,
            //     user: borrower,
            //     amount: amount,
            //     borrow_rate: interest_rate,
            // });

            Ok(())
        }

        #[ink(message)]
        pub fn repay(&mut self, token_id: u32) -> Result<(), Error> {
            let current_time = self.get_current_time();
            let caller = self.env().caller();
            let transfer_rate = self.get_transfer_rate();

            // Validate operation
            let owner = self.env().account_id();
            match self.handle_repayment(caller, token_id, current_time) {
                Err(e) => return Err(e),
                Ok(_) => {}
            }

            // let total_balance = self.get_total_balance(caller);
            let erc20_amount = Balance::from(transfer_rate);

            match self.erc721.transfer(caller, token_id) {
                Err(_) => return Err(Error::ERC721TransferFailed),
                Ok(_) => {}
            }

            match self.erc20.transfer_from(caller, owner, erc20_amount) {
                Err(_) => return Err(Error::ERC20TransferFailed),
                Ok(_) => {}
            }
            // self.env().emit_event(Repaid {
            //     asset: asset,
            //     user: borrower,
            //     amount: amount,
            // });

            Ok(())
        }

        #[ink(message)]
        pub fn get_principal_balance(&self, owner: AccountId) -> Balance {
            let borrower_opt = self.borrowers.get(&owner);
            if borrower_opt.is_some() {
                return borrower_opt.unwrap().balance;
            }
            0
        }

        #[ink(message)]
        pub fn get_total_balance(&self, owner: AccountId) -> Balance {
            let balance = self.get_principal_balance(owner);
            let debt = self.get_total_debt(owner);
            balance + debt
        }

        #[ink(message)]
        pub fn get_total_debt(&self, owner: AccountId) -> Balance {
            let borrower_opt = self.borrowers.get(&owner);
            let interest_rate = self.get_interest_rate();
            if !borrower_opt.is_some() {
                return 0;
            }

            let borrower = borrower_opt.unwrap();
            let interest = self.calculate_interest(
                borrower.balance,
                interest_rate,
                borrower.last_updated_at as u128,
            );
            interest
        }

        #[ink(message)]
        pub fn set_interest_rate(&mut self, _interest_rate: u64) {
            self.administration.interest_rate = _interest_rate;
        }

        #[ink(message)]
        pub fn get_interest_rate(&self) -> u64 {
            0
            // self.administration.interest_rate
        }

        #[ink(message)]
        pub fn set_transfer_rate(&mut self, _transfer_rate: u64) {
            self.administration.transfer_rate = _transfer_rate;
        }

        #[ink(message)]
        pub fn get_transfer_rate(&self) -> u64 {
            self.administration.transfer_rate
        }

        #[ink(message)]
        pub fn enable(&mut self) {
            self.administration.enabled = true;
        }

        #[ink(message)]
        pub fn disable(&mut self) {
            self.administration.enabled = false;
        }

        #[ink(message)]
        pub fn is_enabled(&self) -> bool {
            self.administration.enabled
        }

        fn handle_borrow(
            &mut self,
            borrower_address: AccountId,
            token_id: TokenId,
            interest_rate: u64,
            transfer_rate: u64,
            time: u64,
        ) -> Result<(), Error> {
            let borrower_opt = self.borrowers.get(&borrower_address);
            // assert_eq!(borrower_opt.is_some(), false, "Has already borrowed");

            let mut balance = Balance::from(transfer_rate);

            self.total_loans += 1;
            let loan = Loan {
                id: self.total_loans,
                amount: balance,
                interest_rate: interest_rate,
                transfer_rate: transfer_rate,
                date_borrowed: time,
                date_repaid: None,
                is_repaid: false,
            };

            self.loans.insert((borrower_address, token_id), loan);

            let mut loans: Vec<TokenId> = Vec::new();
            if borrower_opt.is_some() {
                let borrower = self.borrowers.get_mut(&borrower_address).unwrap();
                balance = balance + borrower.balance;
                loans = borrower.loans.to_vec();
            }
            loans.push(token_id);

            self.borrowers.insert(
                borrower_address,
                Borrower {
                    balance: balance,
                    last_updated_at: time,
                    loans: loans,
                },
            );

            Ok(())
        }

        fn handle_repayment(
            &mut self,
            borrower_address: AccountId,
            token_id: TokenId,
            time: u64,
        ) -> Result<(), Error> {
            let borrower_opt = self.borrowers.get_mut(&borrower_address);
            let loan_opt = self.loans.get_mut(&(borrower_address, token_id));

            // assert_eq!(borrower_opt.is_some(), true, "Borrower does not exist");
            let loan = loan_opt.unwrap();
            loan.is_repaid = true;
            loan.date_repaid = Some(time);

            let borrower = borrower_opt.unwrap();
            borrower.balance = borrower.balance - loan.amount;
            borrower.last_updated_at = time;

            Ok(())
        }

        #[ink(message)]
        pub fn get_debt_details(
            &self,
            borrower: AccountId,
            token_id: TokenId,
        ) -> Result<Loan, Error> {
            let loan = self.loans.get(&(borrower, token_id));
            if !loan.is_some() {
                return Err(Error::NoSuchLoan);
            }

            Ok(*loan.clone().unwrap())
        }

        // TODO: Calculate compound interest
        fn calculate_interest(&self, amount: u128, interest_rate: u64, timestamp: u128) -> Balance {
            let ct: u64 = self.env().block_timestamp();
            let exp: u128 = ct as u128 - timestamp;

            let interest: u128 = amount * (interest_rate as u128) * exp / 3_153_6000;
            interest
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
            let assetmanager = AssetManager::new(AccountId::default(), AccountId::default());
            assert_eq!(assetmanager.is_enabled(), true);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn borrow_works() {
            let mut assetmanager = AssetManager::new(AccountId::default(), AccountId::default());
            assert_eq!(assetmanager.is_enabled(), true);

            let asset = AccountId::from([0x05; 32]);
            let owner = AccountId::from([0x01; 32]);

            assetmanager.borrow(1);

            // Borrowed event triggered
            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(1, emitted_events.len());

            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 1);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn repay_works() {
            let mut assetmanager = AssetManager::new(AccountId::default(), AccountId::default());
            assert_eq!(assetmanager.is_enabled(), true);

            let asset = AccountId::from([0x05; 32]);
            let owner = AccountId::from([0x01; 32]);

            assetmanager.borrow(1);

            assetmanager.repay(1);
            // Borrow and Repay events triggered
            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(2, emitted_events.len());

            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 3);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn get_principal_balance_works() {
            let mut assetmanager = AssetManager::new(AccountId::default(), AccountId::default());
            assert_eq!(assetmanager.is_enabled(), true);

            let asset = AccountId::from([0x05; 32]);
            let owner = AccountId::from([0x01; 32]);
            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 0);

            assetmanager.borrow(1);

            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 2);

            assetmanager.repay(1);

            let balance = assetmanager.get_principal_balance(owner);
            assert_eq!(balance, 1);

            assetmanager.repay(1);

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
