use cosmwasm_std::{Instantiate2AddressError, StdError};
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Asset(#[from] cw_asset::AssetError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("{0}")]
    Instantiate2AddressError(#[from] Instantiate2AddressError),

    #[error("The Message sender has to be the owner of the NFT to prevent hacks")]
    SenderNotOwner {},

    #[error("Bundle Not Accepted.")]
    BundleNotAccepted,

    #[error("Not enough bundles in nft.")]
    NotEnoughNFTsInBundle,

    #[error("Too many infusions specified. Have: {have}. Min: {min}, Max: {max}")]
    BadBundle { have: u64, min: u64, max: u64 },

    #[error("Too many infusions specified.")]
    TooManyInfusions,
}
