# README #


### Core Functionality ###


-Implementation of ERC20:     VT Token

-Implementation of ERC721:    AssetToken

-Implementation of AssetManager Contract


### Transaction flow overview ###
Owner of 721 Token deposit asset worth X VT to the manage using AssetManager->Deposit Function

AssetManager send X VT Token to Owner as loan, loan balance increase daily by interest. 

When owner pay back the loan AssetManager send asset x123 back to Owner

### Example ###

Bob  walletB

AssetManager walletM

Bob owns 721 token x123 worth 100 VT

Bob deposit x123 to AssetManager walletM

AssetManager send 100 VT to walletB

Daily interest 0.05

Day 1 Bob loan balance 100

Day 2 Bob loan balance 100.05

Day 2 Bob loan balance 100.100025

When Bob pays back the balance the asset is released. 

