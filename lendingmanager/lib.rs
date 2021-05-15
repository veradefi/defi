#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod lendingmanager {
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

    type TokenId = u32;
    type LoanId = u64;
    #[derive(Encode, Decode, Debug, Default, Copy, Clone, SpreadLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    struct Ownable {
        owner: AccountId,
    }
    #[derive(Encode, Decode, Debug, Default, Copy, Clone, SpreadLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    pub struct Administration {
        interest_rate: u64,
        enabled: bool,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum LoanStatus {
        Available,
        Borrowed,
        Repaid,
        Liquidated,
        Cancelled,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NoSuchToken,
        ERC721TransferFailed,
        ERC20TransferFailed,
        InsufficientBalance,
    }

    #[derive(Clone, Default, Copy, Encode, Decode, Debug, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
    pub struct Loan {
        id: LoanId,
        token_id: TokenId,
        nft_address: AccountId,
        beneficiary_address: AccountId,
        amount: u64,
        borrower_address: AccountId,
        investor_address: Option<AccountId>,
        duration: u64,
        created_at: u64,
        fulfilled_at: Option<u64>,
        repaid_at: Option<u64>,
        status: u8,
        interest_rate: u64,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct LendingManager {
        owner: Ownable,
        loans: StorageHashMap<LoanId, Loan>,
        investors: StorageHashMap<AccountId, Vec<LoanId>>,
        borrowers: StorageHashMap<AccountId, Vec<LoanId>>,
        administration: Administration,
        total_loans: u32,
        erc20: Lazy<Erc20>,
        erc721: Lazy<Erc721>,
    }

    #[ink(event)]
    pub struct LoanListed {
        #[ink(topic)]
        borrower: AccountId,
        #[ink(topic)]
        nft_address: AccountId,
        token_id: u32,
        #[ink(topic)]
        beneficiary_address: AccountId,
        amount: Balance,
        loan_duration: u64,
    }

    #[ink(event)]
    pub struct LoanBorrowed {
        #[ink(topic)]
        investor: AccountId,
        #[ink(topic)]
        loan_id: LoanId,
        #[ink(topic)]
        nft_address: AccountId,
        token_id: u32,
    }

    #[ink(event)]
    pub struct LoanRepaid {
        #[ink(topic)]
        borrower: AccountId,
        #[ink(topic)]
        loan_id: LoanId,
        #[ink(topic)]
        nft_address: AccountId,
        token_id: u32,
    }

    #[ink(event)]
    pub struct LoanExpired {
        #[ink(topic)]
        borrower: AccountId,
        #[ink(topic)]
        loan_id: LoanId,
        #[ink(topic)]
        nft_address: AccountId,
        token_id: u32,
    }

    #[ink(event)]
    pub struct LoanLiquidated {
        #[ink(topic)]
        investor: AccountId,
        #[ink(topic)]
        loan_id: LoanId,
        #[ink(topic)]
        nft_address: AccountId,
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
    pub struct OwnershipTransferred {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
    }

    impl LendingManager {
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn new(
            erc20_address: AccountId,
            erc721_address: AccountId,
            interest_rate: u64,
            enabled: bool,
        ) -> Self {
            let owner = Self::env().caller();

            let erc20 = Erc20::from_account_id(erc20_address);
            let erc721 = Erc721::from_account_id(erc721_address);

            let instance = Self {
                owner: Ownable { owner },
                administration: Administration {
                    interest_rate,
                    enabled,
                },
                loans: Default::default(),
                investors: Default::default(),
                borrowers: Default::default(),
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
        
        /// To list your token for lending
        #[ink(message)]
        pub fn list_token(
            &mut self,
            erc721_address: AccountId,
            token_id: TokenId,
            beneficiary_address: AccountId,
            loan_amount: u64,
            loan_duration: u64,
        ) -> Result<(), Error> {
            assert_eq!(self.is_enabled(), true, "Listing is not enabled");
            let caller = self.env().caller();
            let contract_address = self.env().account_id();
            
            // Transfer tokens from caller to contract

            let erc721_transfer = self
                .erc721
                .transfer_from(caller, contract_address, token_id);
            assert_eq!(
                erc721_transfer.is_ok(),
                true,
                "ERC721 Token transfer failed"
            );

            let loan_id = self.total_loans as LoanId;
            // Add trade into current active list
            let loan = Loan {
                id: loan_id,
                amount: loan_amount,
                nft_address: erc721_address,
                token_id: token_id,
                borrower_address: caller,
                beneficiary_address: beneficiary_address,
                investor_address: None,
                status: LoanStatus::Available as u8,
                duration: loan_duration,
                created_at: self.get_current_time(),
                fulfilled_at: None,
                repaid_at: None,
                interest_rate: self.administration.interest_rate,
            };

            self.loans.insert(loan_id, loan);
            self.total_loans += 1;

            let mut borrowed: Vec<LoanId> = Vec::new();
            let borrower_opt = self.borrowers.get_mut(&caller);
            if borrower_opt.is_some() {
                borrowed = borrower_opt.unwrap().to_vec();
            }
            borrowed.push(loan_id);

            self.borrowers.insert(caller, borrowed);
            Ok(())
        }
        
        /// Lend vt against NFT as collateral
        #[ink(message)]
        pub fn lend(&mut self, loan_id: u64) -> Result<(), Error> {
            assert_eq!(self.is_enabled(), true, "Lending is not enabled");
            let current_time = self.get_current_time();
            let caller = self.env().caller();

            let loan_opt = self.loans.get_mut(&loan_id);
            assert_eq!(loan_opt.is_some(), true, "Loan not available");

            let loan = loan_opt.unwrap();

            // Transfer tokens to contract
            let erc20_transfer =
                self.erc20
                    .transfer_from(caller, loan.beneficiary_address, loan.amount as u128);
            assert_eq!(erc20_transfer.is_ok(), true, "ERC20 Token transfer failed");

            // Mark loan as done
            loan.investor_address = Some(caller);
            loan.fulfilled_at = Some(current_time);
            loan.status = LoanStatus::Borrowed as u8;

            let mut lent: Vec<LoanId> = Vec::new();
            let investor_opt = self.investors.get_mut(&caller);
            if investor_opt.is_some() {
                lent = investor_opt.unwrap().to_vec();
            }
            lent.push(loan_id);

            self.investors.insert(caller, lent);

            Ok(())
        }

        #[ink(message)]
        pub fn expire_loan(&mut self, loan_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let contract_address = self.env().account_id();

            let loan_opt = self.loans.get_mut(&loan_id);
            assert_eq!(loan_opt.is_some(), true, "Loan not available");

            let loan = loan_opt.unwrap();
            assert_eq!(loan.borrower_address, caller, "Only owner can expire loan");
            assert_eq!(
                loan.status,
                LoanStatus::Available as u8,
                "Only non-fulfilled loans can be expired"
            );

            //Transfer token back to seller
            let erc721_transfer =
                self.erc721
                    .transfer_from(contract_address, caller, loan.token_id);
            assert_eq!(
                erc721_transfer.is_ok(),
                true,
                "ERC721 Token transfer failed"
            );

            loan.status = LoanStatus::Cancelled as u8;

            Ok(())
        }

        #[ink(message)]
        pub fn withdraw(&mut self, loan_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let current_time = self.get_current_time();

            let loan_opt = self.loans.get_mut(&loan_id);
            assert_eq!(loan_opt.is_some(), true, "Loan not available");

            let loan = loan_opt.unwrap();
            assert_eq!(
                loan.borrower_address, caller,
                "Only owner can withdraw loan"
            );
            assert_eq!(
                loan.status,
                LoanStatus::Borrowed as u8,
                "Only borrowed loans can be withdrawn"
            );

            // Calculate interest
            let final_amount = Self::calculate_interest(
                loan.amount as u128,
                10,
                current_time,
                loan.fulfilled_at.unwrap(),
            ) + loan.amount as u128;

            // Transfer tokens to contract
            let erc20_transfer =
                self.erc20
                    .transfer_from(caller, loan.investor_address.unwrap(), final_amount);
            assert_eq!(erc20_transfer.is_ok(), true, "ERC20 Token transfer failed");

            // Transfer nft to borrower
            let erc721_transfer = self.erc721.transfer(caller, loan.token_id);
            assert_eq!(
                erc721_transfer.is_ok(),
                true,
                "ERC721 Token transfer failed"
            );

            // Mark loan as done
            loan.status = LoanStatus::Repaid as u8;
            loan.repaid_at = Some(current_time);

            Ok(())
        }

        #[ink(message)]
        pub fn liquidate(&mut self, loan_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();

            let loan_opt = self.loans.get_mut(&loan_id);
            assert_eq!(loan_opt.is_some(), true, "Loan not available");

            let loan = loan_opt.unwrap();
            assert_eq!(
                loan.investor_address.unwrap(),
                caller,
                "Only lender can liquidate loan"
            );
            assert_eq!(
                loan.status,
                LoanStatus::Borrowed as u8,
                "Only borrowed loans can be liquidated"
            );

            // Transfer nft to borrower
            let erc721_transfer = self.erc721.transfer(caller, loan.token_id);
            assert_eq!(
                erc721_transfer.is_ok(),
                true,
                "ERC721 Token transfer failed"
            );

            // Mark loan as done
            loan.status = LoanStatus::Liquidated as u8;

            Ok(())
        }

        #[ink(message)]
        pub fn list_loans_paginated(&self, start: u64, end: u64) -> Vec<Loan> {
            let mut loans: Vec<Loan> = Vec::new();

            for i in start..end {
                let loan_opt = self.loans.get(&i);
                if loan_opt.is_some() {
                    loans.push(*loan_opt.unwrap());
                }
            }
            loans
        }

        #[ink(message)]
        pub fn list_loans(&self) -> Vec<Loan> {
            let mut loans: Vec<Loan> = Vec::new();

            for (_i, loan) in self.loans.iter() {
                loans.push(*loan);
            }
            loans
        }

        #[ink(message)]
        pub fn list_loan(&self, loan_id: u64) -> Loan {
            let loan_opt = self.loans.get(&loan_id);
            assert_eq!(loan_opt.is_some(), true, "Loan not available");

            *loan_opt.unwrap()
        }

        #[ink(message)]
        pub fn get_borrowed_loans(&self, borrower: AccountId) -> Vec<LoanId> {
            let borrower_opt = self.borrowers.get(&borrower);
            let mut loans: Vec<LoanId> = Vec::new();

            if borrower_opt.is_some() {
                loans = borrower_opt.unwrap().to_vec();
            }
            loans
        }

        #[ink(message)]
        pub fn get_investor_loans(&self, investor: AccountId) -> Vec<LoanId> {
            let investor_opt = self.investors.get(&investor);
            let mut loans: Vec<LoanId> = Vec::new();

            if investor_opt.is_some() {
                loans = investor_opt.unwrap().to_vec();
            }
            loans
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

        fn get_current_time(&self) -> u64 {
            self.env().block_timestamp()
        }

        fn calculate_interest(
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
    }

    /// Testcases
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
            let lendingmanager = LendingManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                10,
                true,
            );
            assert_eq!(lendingmanager.is_enabled(), true);
            assert_eq!(lendingmanager.get_interest_rate(), 10);
        }

        #[ink::test]
        fn enable_works() {
            let mut lendingmanager = LendingManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                7,
                false,
            );
            assert_eq!(lendingmanager.is_enabled(), false);
            assert_eq!(lendingmanager.get_interest_rate(), 7);

            lendingmanager.enable();
            assert_eq!(lendingmanager.is_enabled(), true);
        }

        #[ink::test]
        fn disable_works() {
            let mut lendingmanager = LendingManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                7,
                true,
            );
            assert_eq!(lendingmanager.is_enabled(), true);
            assert_eq!(lendingmanager.get_interest_rate(), 7);

            lendingmanager.disable();
            assert_eq!(lendingmanager.is_enabled(), false);
        }

        #[ink::test]
        fn set_interest_rate_works() {
            let mut lendingmanager = LendingManager::new(
                instantiate_erc20_contract(),
                instantiate_erc721_contract(),
                7,
                true,
            );

            assert_eq!(lendingmanager.is_enabled(), true);
            assert_eq!(lendingmanager.get_interest_rate(), 7);

            lendingmanager.set_interest_rate(8);
            assert_eq!(lendingmanager.get_interest_rate(), 8);
        }

        #[ink::test]
        #[should_panic]
        fn listing_disabled_works() {
            // Disabled should panic
            let erc721 = instantiate_erc721_contract();
            let erc20 = instantiate_erc20_contract();
            let mut lendingmanager = LendingManager::new(erc20, erc721, 10, false);
            assert_eq!(lendingmanager.is_enabled(), false);
            let owner = AccountId::from([0x01; 32]);
            assert!(
                lendingmanager
                    .list_token(erc721, 1, owner, 1000, 10)
                    .is_err(),
                "Should not allow deposit in disabled state"
            );

            lendingmanager.enable();
            assert_eq!(lendingmanager.is_enabled(), true);
            assert!(
                lendingmanager
                    .list_token(erc721, 1, owner, 1000, 10)
                    .is_err(),
                "Should not allow deposit when erc721 allowance is not made"
            );
        }

        #[ink::test]
        fn calculate_interest_works() {
            let erc20_decimals = 1000_000_000_000;

            assert_eq!(
                LendingManager::calculate_interest(
                    1 * erc20_decimals,
                    10,
                    86400 * 365 * 1000,
                    86400 * 1000
                ),
                105_155_781_613
            ); // Total 365 day borrowed with yearly interest rate of 10

            assert_eq!(
                LendingManager::calculate_interest(
                    1 * erc20_decimals,
                    10,
                    86400 * 30 * 1000,
                    86400 * 1000
                ),
                8_251_913_257
            ); // Total 30 day borrowed with yearly interest rate of 10

            assert_eq!(
                LendingManager::calculate_interest(
                    1 * erc20_decimals,
                    10,
                    86400 * 182 * 1000,
                    86400 * 1000
                ),
                51_119_918_056
            ); // Total 6 month (182 days) borrowed with yearly interest rate of 10

            assert_eq!(
                LendingManager::calculate_interest(
                    1 * erc20_decimals,
                    7,
                    86400 * 365 * 1000,
                    86400 * 1000
                ),
                72_505_096_314
            ); // Total 1 year borrowed with yearly interest rate of 7

            assert_eq!(
                LendingManager::calculate_interest(
                    1 * erc20_decimals,
                    7,
                    86401 * 1000,
                    86400 * 1000
                ),
                191_791_331
            ); // Total 1 day borrowed with yearly interest rate of 7

            assert_eq!(
                LendingManager::calculate_interest(
                    2 * erc20_decimals,
                    7,
                    86401 * 1000,
                    86400 * 1000
                ),
                383_582_662
            ); // Total 1 day borrowed with yearly interest rate of 7
        }
    }
}
