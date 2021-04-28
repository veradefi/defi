#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod exchangemanager {
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

    type TradeId = u64;
    type TokenId = u32;
    #[derive(Encode, Decode, Debug, Default, Copy, Clone, SpreadLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    struct Ownable {
        owner: AccountId,
    }

    #[derive(Encode, Decode, Debug, Default, Copy, Clone, SpreadLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    pub struct Administration {
        fee: u64,
        enabled: bool,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum TradeStatus {
        Available,
        Purchased,
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
    pub struct Trade {
        id: TradeId,
        price: u64,
        nft_address: AccountId,
        token_id: TokenId,
        seller_address: AccountId,
        beneficiary_address: AccountId,
        buyer_address: Option<AccountId>,
        expiration_date: u64,
        status: u8,
        fee: u64,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct ExchangeManager {
        owner: Ownable,
        trades: StorageHashMap<TradeId, Trade>,
        administration: Administration,
        total_trades: u32,
        erc20: Lazy<Erc20>,
        erc721: Lazy<Erc721>,
    }

    #[ink(event)]
    pub struct TradeListed {
        #[ink(topic)]
        seller: AccountId,
        #[ink(topic)]
        amount: Balance,
        #[ink(topic)]
        borrow_rate: u64,
        token_id: u32,
    }

    #[ink(event)]
    pub struct TradePurchased {
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
    pub struct FeeChanged {
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

    impl ExchangeManager {
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn new(
            erc20_address: AccountId,
            erc721_address: AccountId,
            fee: u64,
            enabled: bool,
        ) -> Self {
            let owner = Self::env().caller();

            let erc20 = Erc20::from_account_id(erc20_address);
            let erc721 = Erc721::from_account_id(erc721_address);
            let instance = Self {
                owner: Ownable { owner },
                administration: Administration { fee, enabled },
                trades: Default::default(),
                total_trades: 0,
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

        /// Allows borrowing on behalf of another account
        /// erc20_owner should have granted approval to assetmanager contract to make transfer on their behalf and have sufficient balance
        /// Caller should have granted approval to erc721 token before executing this function
        #[ink(message)]
        pub fn create_trade(
            &mut self,
            nft_address: AccountId,
            token_id: TokenId,
            beneficiary_address: AccountId,
            price: u64,
            expiration_date: u64,
        ) -> Result<(), Error> {
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

            self.total_trades += 1;
            let trade_id = self.total_trades as u64;
            // Add trade into current active list
            let trade = Trade {
                id: trade_id,
                price: price,
                nft_address: nft_address,
                token_id: token_id,
                seller_address: caller,
                beneficiary_address: beneficiary_address,
                buyer_address: None,
                status: TradeStatus::Available as u8,
                expiration_date: expiration_date,
                fee: self.administration.fee,
            };
            self.trades.insert(trade_id, trade);

            Ok(())
        }

        #[ink(message)]
        pub fn purchase(&mut self, trade_id: u64) -> Result<(), Error> {
            let current_time = self.get_current_time();
            let caller = self.env().caller();
            let contract_address = self.env().account_id();

            let trade_opt = self.trades.get_mut(&trade_id);
            assert_eq!(trade_opt.is_some(), true, "Trade not available");

            let trade = trade_opt.unwrap();
            // Deduct fee
            let fee = trade.fee * trade.price / 100;
            let erc20_amount = trade.price - fee;

            // Transfer tokens to contract
            let erc20_transfer =
                self.erc20
                    .transfer_from(caller, contract_address, trade.price as u128);
            assert_eq!(erc20_transfer.is_ok(), true, "ERC20 Token transfer failed");

            // Transfer tokens to seller deducting fee
            let fee_transfer = self
                .erc20
                .transfer(trade.beneficiary_address, erc20_amount as u128);
            assert_eq!(fee_transfer.is_ok(), true, "ERC20 Token transfer failed");

            // Transfer nft to buyer
            let erc721_transfer =
                self.erc721
                    .transfer_from(contract_address, caller, trade.token_id);
            assert_eq!(
                erc721_transfer.is_ok(),
                true,
                "ERC721 Token transfer failed"
            );

            // Mark trade as done
            trade.buyer_address = Some(caller);
            trade.status = TradeStatus::Purchased as u8;

            Ok(())
        }

        #[ink(message)]
        pub fn expire_trade(&mut self, trade_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let contract_address = self.env().account_id();

            let trade_opt = self.trades.get_mut(&trade_id);
            assert_eq!(trade_opt.is_some(), true, "Trade not available");

            let trade = trade_opt.unwrap();
            assert_eq!(trade.seller_address, caller, "Only seller can expire trade");

            //Transfer token back to seller
            let erc721_transfer =
                self.erc721
                    .transfer_from(contract_address, caller, trade.token_id);
            assert_eq!(
                erc721_transfer.is_ok(),
                true,
                "ERC721 Token transfer failed"
            );

            trade.status = TradeStatus::Cancelled as u8;

            Ok(())
        }

        #[ink(message)]
        pub fn list_trades(&self) -> Vec<Trade> {
            let mut trades: Vec<Trade> = Vec::new();

            for (_i, trade) in self.trades.iter() {
                trades.push(*trade);
            }
            trades
        }

        #[ink(message)]
        pub fn list_trade(&self, trade_id: u64) -> Trade {
            let trade_opt = self.trades.get(&trade_id);
            assert_eq!(trade_opt.is_some(), true, "Trade not available");

            *trade_opt.clone().unwrap()
        }

        #[ink(message)]
        pub fn withdraw_fees(&mut self, erc20_address: AccountId) {
            assert!(self.only_owner(self.env().caller()));
            let contract_address = self.env().account_id();

            let balance = self.erc20.balance_of(contract_address);
            self.erc20.transfer(erc20_address, balance);
        }

        /// Allows owner to set transfer rate
        /// Only affects future borrowing
        #[ink(message)]
        pub fn set_fee(&mut self, _fee: u64) {
            assert!(self.only_owner(self.env().caller()));
            self.env().emit_event(FeeChanged {
                old_value: self.administration.fee,
                new_value: _fee,
            });
            self.administration.fee = _fee;
        }

        /// Returns current transfer rate
        #[ink(message)]
        pub fn get_fee(&self) -> u64 {
            self.administration.fee
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

        fn get_current_time(&self) -> u64 {
            self.env().block_timestamp()
        }
    }
}
