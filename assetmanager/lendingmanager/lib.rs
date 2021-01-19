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

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotOwner,
        TokenNotFound,
        NotAllowed,
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
        borrower_address: AccountId,
        transfer_rate: u64,
        interest_rate: u64,
        date_borrowed: u64,
        date_repaid: Option<u64>,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct LendingManager {
        borrowers: StorageHashMap<AccountId, Borrower>,
        loans: StorageHashMap<AccountId, Borrower>,
    }

    impl LendingManager {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            let instance = Self {
                borrowers: Default::default(),
                loans: Default::default(),
            };
            instance
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new()
        }


        #[ink(message)]
        pub fn handle_borrow(&mut self, asset: AccountId, borrower: AccountId, amount: u64, interest_rate: u64, transfer_rate: u64, time: u64) -> Result<(), Error> {
            let borrower_opt = self.borrowers.get(&borrower);
            // assert_eq!(borrower_opt.is_some(), false, "Has already borrowed");

            self.borrowers.insert(
                borrower,
                Borrower {
                    balance: Balance::from(amount),
                    last_updated_at: time,
                },
            );

            Ok(())
        }

        #[ink(message)]
        pub fn handle_repayment(&mut self, asset: AccountId, borrower: AccountId, amount: u64, time: u64) -> Result<(), Error> {
            let borrower_opt = self.borrowers.get_mut(&borrower);
            // assert_eq!(borrower_opt.is_some(), true, "Borrower does not exist");

            let borrower = borrower_opt.unwrap();
            borrower.balance = borrower.balance - amount as u128;
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
