#![cfg_attr(not(feature = "std"), no_std)]

pub use self::lendingmanager::LendingManager;
use ink_lang as ink;

#[ink::contract]
pub mod lendingmanager {

    use ink_storage::collections::HashMap as StorageHashMap;
    use ink_storage::{
        traits::{PackedLayout, SpreadLayout, StorageLayout},
        Lazy,
    };
    use scale::{Decode, Encode};

    pub type LoanId = u64;
    pub type TokenId = u32;

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        
    }

    #[derive(Clone, Default, Encode, Decode, Debug, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
    struct Borrower {
        balance: Balance,
        last_updated_at: u64,
    }

    #[derive(Clone, Default, Encode, Decode, Debug, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
    struct Loan {
        id: LoanId,
        amount: Balance,
        transfer_rate: u64,
        interest_rate: u64,
        date_borrowed: u64,
        date_repaid: Option<u64>,
        is_repaid: bool,
    }

    #[ink(storage)]
    pub struct LendingManager {
        borrowers: StorageHashMap<AccountId, Borrower>,
        loans: StorageHashMap<(AccountId, TokenId), Loan>,
        total_loans: u64,
    }

    impl LendingManager {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            let instance = Self {
                borrowers: Default::default(),
                loans: Default::default(),
                total_loans: 0,
            };
            instance
        }


        #[ink(message)]
        pub fn handle_borrow(&mut self, borrower_address: AccountId, token_id: TokenId, interest_rate: u64, transfer_rate: u64, time: u64) -> Result<(), Error> {
            let borrower_opt = self.borrowers.get(&borrower_address);
            // assert_eq!(borrower_opt.is_some(), false, "Has already borrowed");

            let mut balance = Balance::from(transfer_rate);
            if borrower_opt.is_some() {
                let borrower = self.borrowers.get_mut(&borrower_address).unwrap();
                balance = balance + borrower.balance;
            }
            self.borrowers.insert(
                borrower_address,
                Borrower {
                    balance: balance,
                    last_updated_at: time,
                },
            );

            self.total_loans += 1;
            let loan = Loan{
                id: self.total_loans,
                amount: balance,
                interest_rate: interest_rate,
                transfer_rate: transfer_rate,
                date_borrowed: time,
                date_repaid: None,
                is_repaid: false,
            };

            self.loans.insert(
                (borrower_address, token_id),
                loan,
            );

            Ok(())
        }

        #[ink(message)]
        pub fn handle_repayment(&mut self,borrower_address: AccountId, token_id: TokenId, time: u64) -> Result<(), Error> {
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
        pub fn get_principal_balance(&self, owner: AccountId) -> Balance {
            self.borrowers
                .get(&owner)
                .unwrap_or(&Borrower {
                    balance: 0,
                    last_updated_at: 0,
                })
                .balance
        }

        #[ink(message)]
        pub fn get_total_balance(&self, owner: AccountId, interest_rate: u64) -> Balance {
            let balance = self.get_principal_balance(owner);
            let debt = self.get_total_debt(owner, interest_rate);
            balance + debt
        }

        #[ink(message)]
        pub fn get_total_debt(&self, owner: AccountId, interest_rate: u64) -> Balance {
            let borrower = self.borrowers.get(&owner).unwrap_or(&Borrower {
                balance: 0,
                last_updated_at: 0,
            });
            let interest = self.calculate_interest(
                10,
                interest_rate,
                borrower.last_updated_at,
            );
            Balance::from(interest)
        }

        // TODO: Calculate compound interest
        fn calculate_interest(&self, amount: u64, interest_rate: u64, timestamp: u64) -> u64 {
            let ct: u64 = self.env().block_timestamp();
            let exp: u64 = ct - timestamp;

            let interest: u64 = amount * interest_rate * exp / 3_153_6000;
            interest
        }
    }
}
