# Non-Custodial NFT Loans



## Instantiate Contract
```json
{
    "treasury_addr": "stars1...",
    "fee_rate": "0.5",
    "listing_fee_coins": [
        {"amount": "420", "denom":"ustars"},
        {"amount": "69", "denom":"uatom"}
        ],
    "name": "atlas-dao-nft-loan",
    "owner": "stars1..."
} 
```

## Actions
### `ListCollaterals`
*approve nft as collateral, and define loan terms being offered*
### `ModifyCollaterals`
*modify collateral listing, given it is able to be modified*
### `WithdrawCollaterals`
*withdraw collateral listing, given it is able to be withdrawn*
### `AcceptLoan`
*Accept a loan as a lender & begin the loan collateral escrow timeline*
### `AcceptOffer`
*Accept a loan off as the borrower & begin the loan collateral escrow timeline*
### `MakeOffer`
*Make a loan offer as the lender with loan terms of your choice*
### `CancelOffer`
*cancel a loan offer, given it is able to be cancelled*
### `RefuseOffer`
*refuse a loan offer, given it is able to be refused*
### `WithdrawRefusedOffer`
*withdraw assets in the loan offer, given it is able to be withdrawn*
### `RepayBorrowedFunds`
*repay borrowed funds in the loan offer, given it is able to be repaid*
### `WithdrawDefaultedLoan`
*withdraw collateral in the loan, given it has defaulted*
### `SetOwner` **admin-only*
*admin function to update collateral in the loan, given it has defaulted*
### `SetFeeDestination`**admin-only*
*admin function to set new destination for fees*
### `SetFeeRate`**admin-only*
*admin function to set new fee rate for % from interests on loans*
### Queries
The contract stores 

### `Config`
*returns contract params*
### `BorrowerInfo`
*returns the last loan id made by a borrower. Requires the borrowers address.*
### `CollateralInfo`
*queries and returns current state of a specific loan listing. Requires the borrowers address & the loan id.*
### `Collaterals`
*returns all of a borrowers collateral listings. Requires the borrowers address*
### `AllCollaterals`
*returns all collaterals* 
### `OfferInfo`
*returns info about a loan offer. Requires the global_offer_id*
### `Offers`
*returns offers for a loan given the borrower & the loan id*
### `LenderOffers`
*returns all offers made by a specific address*

## Migrate