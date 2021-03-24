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
    struct Ownable {
        owner: AccountId,
    }

    #[derive(Encode, Decode, Debug, Default, Copy, Clone, SpreadLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    pub struct AddressManager {
        erc20_address: AccountId,
        erc721_address: AccountId,
        erc20_owner: AccountId,
        erc721_owner: AccountId,
    }

    #[derive(Encode, Decode, Debug, Default, Copy, Clone, SpreadLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    pub struct Administration {
        interest_rate: u64,
        transfer_rate: u128,
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
        InsufficientBalance,
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
        transfer_rate: u128,
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
        owner: Ownable,
        borrowers: StorageHashMap<AccountId, Borrower>,
        loans: StorageHashMap<(AccountId, TokenId), Loan>,
        administration: Administration,
        address_manager: AddressManager,
        total_loans: u64,
        erc20: Lazy<Erc20>,
        erc721: Lazy<Erc721>,
    }

    #[ink(event)]
    pub struct Borrowed {
        #[ink(topic)]
        borrower: AccountId,
        #[ink(topic)]
        amount: Balance,
        #[ink(topic)]
        borrow_rate: u64,
        token_id: u32,
    }

    #[ink(event)]
    pub struct Repaid {
        #[ink(topic)]
        borrower: AccountId,
        #[ink(topic)]
        amount: Balance,
        token_id: u32,
    }

    #[ink(event)]
    pub struct Enabled {}

    #[ink(event)]
    pub struct Disbaled {}

    #[ink(event)]
    pub struct InterestRateChanged {
        #[ink(topic)]
        old_value: u64,
        #[ink(topic)]
        new_value: u64,
    }

    #[ink(event)]
    pub struct TransferRateChanged {
        #[ink(topic)]
        old_value: Balance,
        #[ink(topic)]
        new_value: Balance,
    }

    #[ink(event)]
    pub struct OwnershipTransferred {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
    }

    impl AssetManager {
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn new(
            erc20_address: AccountId,
            erc721_address: AccountId,
            interest_rate: u64,
            transfer_rate: Balance,
            enabled: bool,
        ) -> Self {
            let owner = Self::env().caller();

            let erc20 = Erc20::from_account_id(erc20_address);
            let erc721 = Erc721::from_account_id(erc721_address);
            let instance = Self {
                owner: Ownable { owner },
                administration: Administration {
                    interest_rate,
                    transfer_rate,
                    enabled,
                },
                address_manager: AddressManager {
                    erc20_address: erc20_address,
                    erc721_address: erc721_address,
                    erc20_owner: owner,
                    erc721_owner: owner,
                },
                borrowers: Default::default(),
                loans: Default::default(),
                total_loans: 0,
                erc20: Lazy::new(erc20),
                erc721: Lazy::new(erc721),
            };
            instance
        }

        /// Checks if caller is owner of AssetManager contract
        #[ink(message)]
        pub fn is_owner(&self) -> bool {
            return self.env().caller() == self.owner.owner;
        }

        /// Gets owner address of AssetManager contract
        #[ink(message)]
        pub fn get_owner(&self) -> AccountId {
            self.owner.owner
        }

        /// Transfers ownership from current owner to new_owner address
        /// Can only be called by the current owner
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> bool {
            let caller = self.env().caller();
            assert!(self.only_owner(caller));
            self.owner.owner = new_owner;
            self.env().emit_event(OwnershipTransferred {
                from: caller,
                to: new_owner,
            });
            true
        }

        fn only_owner(&self, caller: AccountId) -> bool {
            caller == self.owner.owner
        }

        /// Sets owner address of erc20 contract
        #[ink(message)]
        pub fn set_erc20_owner(&mut self, erc20_owner: AccountId) {
            assert!(self.only_owner(self.env().caller()));
            self.address_manager.erc20_owner = erc20_owner;
        }

        /// Returns owner address of erc20 contract
        #[ink(message)]
        pub fn get_erc20_owner(&self) -> AccountId {
            self.address_manager.erc20_owner
        }

        /// Sets owner address of erc721 contract
        #[ink(message)]
        pub fn set_erc721_owner(&mut self, erc721_owner: AccountId) {
            assert!(self.only_owner(self.env().caller()));
            self.address_manager.erc721_owner = erc721_owner;
        }

        /// Returns owner address of erc721 contract
        #[ink(message)]
        pub fn get_erc721_owner(&self) -> AccountId {
            self.address_manager.erc721_owner
        }

        /// Allows borrowing on behalf of another account
        /// erc20_owner should have granted approval to assetmanager contract to make transfer on their behalf and have sufficient balance
        /// Caller should have granted approval to erc721 token before executing this function
        #[ink(message)]
        pub fn deposit(&mut self, token_id: u32, on_behalf_of: AccountId) -> Result<(), Error> {
            assert_eq!(self.is_enabled(), true, "Borrowing is not enabled");
            let current_time = self.get_current_time();
            let caller = self.env().caller();

            let interest_rate = self.get_interest_rate();
            let transfer_rate = self.get_transfer_rate();
            let AddressManager {
                erc20_owner,
                erc721_owner,
                ..
            } = self.address_manager;

            let erc20_amount = Balance::from(transfer_rate);

            // Contract does not have enough erc20 balance for loan
            if self.erc20.balance_of(erc20_owner) < erc20_amount {
                return Err(Error::InsufficientBalance);
            }

            // Handles borrowing
            let db_transfer =
                self.handle_borrow(caller, token_id, interest_rate, transfer_rate, current_time);
            assert_eq!(db_transfer.is_ok(), true, "Error storing transaction");

            let erc721_transfer = self.erc721.transfer_from(caller, erc721_owner, token_id);
            assert_eq!(
                erc721_transfer.is_ok(),
                true,
                "ERC721 Token transfer failed"
            );

            let erc20_transfer = self
                .erc20
                .transfer_from(erc20_owner, on_behalf_of, erc20_amount);
            assert_eq!(erc20_transfer.is_ok(), true, "ERC20 Token transfer failed");

            // self.env().emit_event(Borrowed {
            //     borrower: on_behalf_of,
            //     amount: erc20_amount,
            //     borrow_rate: interest_rate,
            //     token_id: token_id,
            // });

            Ok(())
        }

        // Allows repayment on behalf of another account
        /// erc721_owner should have granted approval to assetmanager contract to make transfer on their behalf
        // Caller should have granted approval to erc20 before executing this function
        #[ink(message)]
        pub fn withdraw(&mut self, token_id: u32, on_behalf_of: AccountId) -> Result<(), Error> {
            let current_time = self.get_current_time();
            let caller = self.env().caller();

            // Validate operation
            let AddressManager {
                erc20_owner,
                erc721_owner,
                ..
            } = self.address_manager;

            let total_balance = self.get_total_balance_of_loan(on_behalf_of, token_id);
            let db_transfer = self.handle_repayment(on_behalf_of, token_id, current_time);
            assert_eq!(db_transfer.is_ok(), true, "Error storing transaction");

            let erc20_amount = total_balance;

            let erc20_transfer = self.erc20.transfer_from(caller, erc20_owner, erc20_amount);
            assert_eq!(erc20_transfer.is_ok(), true, "ERC20 Token transfer failed");

            let erc721_transfer = self
                .erc721
                .transfer_from(erc721_owner, on_behalf_of, token_id);
            assert_eq!(
                erc721_transfer.is_ok(),
                true,
                "ERC721 Token transfer failed"
            );

            // self.env().emit_event(Repaid {
            //     borrower: on_behalf_of,
            //     amount: erc20_amount,
            //     token_id: token_id,
            // });

            Ok(())
        }

        /// Returns principal amount borrowed by the address
        #[ink(message)]
        pub fn get_principal_balance_of_borrower(&self, owner: AccountId) -> Balance {
            let borrower_opt = self.borrowers.get(&owner);
            if borrower_opt.is_some() {
                return borrower_opt.unwrap().balance;
            }
            0
        }

        /// Returns total amount borrowed including interest by the address
        #[ink(message)]
        pub fn get_total_balance_of_borrower(&self, owner: AccountId) -> Balance {
            let balance = self.get_principal_balance_of_borrower(owner);
            let debt = self.get_total_debt_of_borrower(owner);
            balance + debt
        }

        /// Returns total interest incurred by the address
        #[ink(message)]
        pub fn get_total_debt_of_borrower(&self, owner: AccountId) -> Balance {
            let borrower_opt = self.borrowers.get(&owner);
            if !borrower_opt.is_some() {
                return 0;
            }

            let borrower = borrower_opt.unwrap();
            let mut interest: u128 = 0;
            for token_id in borrower.loans.to_vec() {
                interest = interest + self.get_total_debt_of_loan(owner, token_id);
            }
            interest
        }

        /// Returns principal amount borrowed against by address against token_id
        #[ink(message)]
        pub fn get_principal_balance_of_loan(&self, owner: AccountId, token_id: u32) -> Balance {
            let loan_opt = self.loans.get(&(owner, token_id));
            if loan_opt.is_some() {
                let loan = loan_opt.unwrap();
                if !loan.is_repaid {
                    return loan.amount;
                }
            }
            0
        }

        /// Returns total amount including interest borrowed against by address against token_id
        #[ink(message)]
        pub fn get_total_balance_of_loan(&self, owner: AccountId, token_id: u32) -> Balance {
            let balance = self.get_principal_balance_of_loan(owner, token_id);
            let debt = self.get_total_debt_of_loan(owner, token_id);
            balance + debt
        }

        /// Returns interest incurred against by address against token_id

        #[ink(message)]
        pub fn get_total_debt_of_loan(&self, owner: AccountId, token_id: u32) -> Balance {
            let loan_opt = self.loans.get(&(owner, token_id));
            if !loan_opt.is_some() {
                return 0;
            }
            let loan = loan_opt.unwrap();
            if loan.is_repaid {
                return 0;
            }
            let ct: u64 = self.env().block_timestamp(); // Gets timstamp in milliseconds

            let interest =
                self.calculate_interest(loan.amount, loan.interest_rate, ct, loan.date_borrowed);
            interest
        }

        /// Allows owner to set interest rate
        /// Only affects future borrowing
        #[ink(message)]
        pub fn set_interest_rate(&mut self, _interest_rate: u64) {
            assert!(self.only_owner(self.env().caller()));
            self.env().emit_event(InterestRateChanged {
                old_value: self.administration.interest_rate,
                new_value: _interest_rate,
            });
            self.administration.interest_rate = _interest_rate;
        }

        /// Returns current yearly interest rate
        #[ink(message)]
        pub fn get_interest_rate(&self) -> u64 {
            self.administration.interest_rate
        }

        /// Allows owner to set transfer rate
        /// Only affects future borrowing
        #[ink(message)]
        pub fn set_transfer_rate(&mut self, _transfer_rate: Balance) {
            assert!(self.only_owner(self.env().caller()));
            self.env().emit_event(TransferRateChanged {
                old_value: self.administration.transfer_rate,
                new_value: _transfer_rate,
            });
            self.administration.transfer_rate = _transfer_rate;
        }

        /// Returns current transfer rate
        #[ink(message)]
        pub fn get_transfer_rate(&self) -> Balance {
            self.administration.transfer_rate
        }

        /// Allows owner to enable borrowing
        #[ink(message)]
        pub fn enable(&mut self) {
            assert!(self.only_owner(self.env().caller()));
            self.administration.enabled = true;
            self.env().emit_event(Enabled {});
        }

        /// Allows owner to disable borrowing
        #[ink(message)]
        pub fn disable(&mut self) {
            assert!(self.only_owner(self.env().caller()));
            self.administration.enabled = false;
            self.env().emit_event(Disbaled {});
        }

        /// Checks if borrowing is enabled
        #[ink(message)]
        pub fn is_enabled(&self) -> bool {
            self.administration.enabled
        }

        fn handle_borrow(
            &mut self,
            borrower_address: AccountId,
            token_id: TokenId,
            interest_rate: u64,
            transfer_rate: Balance,
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
            assert_eq!(borrower_opt.is_some(), true, "Borrower does not exist");
            let loan_opt = self.loans.get_mut(&(borrower_address, token_id));
            assert_eq!(loan_opt.is_some(), true, "Loan does not exist");

            let loan = loan_opt.unwrap();
            assert_eq!(loan.is_repaid, false, "Loan has already been paid");

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

        fn calculate_interest(
            &self,
            amount: u128,
            interest_rate: u64,
            current_timestamp: u64,
            date_borrowed: u64,
        ) -> Balance {
            let difference_in_secs: u128 =
                ((current_timestamp - date_borrowed) as u128 / 1000_u128).into(); // Total time elapsed in seconds
            let secs_in_day: u128 = 24 * 60 * 60;
            let difference_in_days: u128 = difference_in_secs / secs_in_day;
            let mut days_since_borrowed = difference_in_days;
            if difference_in_secs - (difference_in_days * days_since_borrowed) > 0 {
                days_since_borrowed = days_since_borrowed + 1;
            }

            let mut s = 0;
            let mut n = 1;
            let mut b = 1;
            let q: u128 = 365 * 100 / interest_rate as u128;

            // let mut p = 8_u32;
            // if p < days_since_borrowed as u32 {
            //     p = days_since_borrowed as u32;
            // }
            for x in 0..8 {
                s = s + amount * n / b / (q.pow(x));
                if days_since_borrowed < x.into() {
                    break;
                }
                n = n * (days_since_borrowed - x as u128);
                b = b * (x as u128 + 1);
            }
            s - amount
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
        fn instantiate_erc20_contract() -> AccountId {
            let erc20 = Erc20::new(1000000);
            let callee =
                ink_env::account_id::<ink_env::DefaultEnvironment>().unwrap_or([0x0; 32].into());
            callee
        }
        fn instantiate_erc721_contract() -> AccountId {
            let erc20 = Erc721::new();
            let callee =
                ink_env::account_id::<ink_env::DefaultEnvironment>().unwrap_or([0x0; 32].into());
            callee
        }
        #[ink::test]
        fn new_works() {
            let assetmanager = AssetManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                10,
                1000,
                true,
            );
            assert_eq!(assetmanager.is_enabled(), true);
            assert_eq!(assetmanager.get_interest_rate(), 10);
            assert_eq!(assetmanager.get_transfer_rate(), 1000);
        }

        #[ink::test]
        fn enable_works() {
            let mut assetmanager = AssetManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                7,
                100,
                false,
            );
            assert_eq!(assetmanager.is_enabled(), false);
            assert_eq!(assetmanager.get_interest_rate(), 7);
            assert_eq!(assetmanager.get_transfer_rate(), 100);

            assetmanager.enable();
            assert_eq!(assetmanager.is_enabled(), true);
        }

        #[ink::test]
        fn disable_works() {
            let mut assetmanager = AssetManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                7,
                100,
                true,
            );
            assert_eq!(assetmanager.is_enabled(), true);
            assert_eq!(assetmanager.get_interest_rate(), 7);
            assert_eq!(assetmanager.get_transfer_rate(), 100);

            assetmanager.disable();
            assert_eq!(assetmanager.is_enabled(), false);
        }

        #[ink::test]
        fn set_interest_rate_works() {
            let mut assetmanager = AssetManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                7,
                100,
                true,
            );

            assert_eq!(assetmanager.is_enabled(), true);
            assert_eq!(assetmanager.get_interest_rate(), 7);
            assert_eq!(assetmanager.get_transfer_rate(), 100);

            assetmanager.set_interest_rate(8);
            assert_eq!(assetmanager.get_interest_rate(), 8);
        }

        #[ink::test]
        fn set_transfer_rate_works() {
            let mut assetmanager = AssetManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                7,
                100,
                true,
            );

            assert_eq!(assetmanager.is_enabled(), true);
            assert_eq!(assetmanager.get_interest_rate(), 7);
            assert_eq!(assetmanager.get_transfer_rate(), 100);

            assetmanager.set_transfer_rate(110);
            assert_eq!(assetmanager.get_transfer_rate(), 110);
        }

        #[ink::test]
        #[should_panic]
        fn borrow_disabled_works() {
            // Disabled should panic
            let mut assetmanager = AssetManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                10,
                1000,
                false,
            );
            assert_eq!(assetmanager.is_enabled(), false);
            let owner = AccountId::from([0x01; 32]);
            assert!(
                assetmanager.deposit(1, owner).is_err(),
                "Should not allow deposit in disabled state"
            );

            assetmanager.enable();
            assert_eq!(assetmanager.is_enabled(), true);
            assert!(
                assetmanager.deposit(1, owner).is_err(),
                "Should not allow deposit when erc721 allowance is not made"
            );
        }

        #[ink::test]
        fn calculate_interest_works() {
            let assetmanager = AssetManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                10,
                1000,
                true,
            );
            assert_eq!(assetmanager.is_enabled(), true);

            let erc20_decimals = 1000_000_000_000;

            assert_eq!(
                assetmanager.calculate_interest(
                    1 * erc20_decimals,
                    10,
                    86400 * 365 * 1000,
                    86400 * 1000
                ),
                105_155_781_613
            ); // Total 365 day borrowed with yearly interest rate of 10

            assert_eq!(
                assetmanager.calculate_interest(
                    1 * erc20_decimals,
                    10,
                    86400 * 30 * 1000,
                    86400 * 1000
                ),
                8_251_913_257
            ); // Total 30 day borrowed with yearly interest rate of 10

            assert_eq!(
                assetmanager.calculate_interest(
                    1 * erc20_decimals,
                    10,
                    86400 * 182 * 1000,
                    86400 * 1000
                ),
                51_119_918_056
            ); // Total 6 month (182 days) borrowed with yearly interest rate of 10

            assert_eq!(
                assetmanager.calculate_interest(
                    1 * erc20_decimals,
                    7,
                    86400 * 365 * 1000,
                    86400 * 1000
                ),
                72_505_096_314
            ); // Total 1 year borrowed with yearly interest rate of 7

            assert_eq!(
                assetmanager.calculate_interest(1 * erc20_decimals, 7, 86401 * 1000, 86400 * 1000),
                191_791_331
            ); // Total 1 day borrowed with yearly interest rate of 7

            assert_eq!(
                assetmanager.calculate_interest(2 * erc20_decimals, 7, 86401 * 1000, 86400 * 1000),
                383_582_662
            ); // Total 1 day borrowed with yearly interest rate of 7
        }
    }
}
