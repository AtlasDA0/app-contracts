use thiserror::Error;

use cosmwasm_std::{StdError, Timestamp};
use utils::state::AssetInfo;

use crate::state::RaffleState;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized.")]
    Unauthorized,

    #[error("Proxy address is not valid")]
    InvalidProxyAddress,

    #[error("Merkle is immutable.")]
    MerkleImmutable,

    #[error("Register the Merkle root before requesting randomness")]
    MerkleRootAbsent,

    #[error("Invalid input")]
    InvalidInput {},

    #[error("The Raffle Fee you have provided is invalid.")]
    InvalidRaffleFee {},

    #[error("invalid amount:")]
    InvalidAmount(String),

    #[error("Already claimed")]
    Claimed {},

    #[error("Wrong length")]
    WrongLength {},

    #[error("Verification failed")]
    VerificationFailed {},

    #[error("The sender is not randomly eligible for the randdrop")]
    NotRandomlyEligible {},

    #[error("The claiming phase did not start. The random beacon is yet to be fetched")]
    RandomnessUnavailable {},

    #[error("Cannot migrate from different contract type: {previous_contract}")]
    CannotMigrate { previous_contract: String },

    // callback should only be allowed to be called by the proxy contract
    // otherwise anyone can cut the randomness workflow and cheat the randomness
    #[error("Unauthorized Receive execution")]
    UnauthorizedReceive,

    #[error("Requesting randomness {random_beacon_after} in the past compared to  {block_time}. This is not safe, make sure the timestamp is in the future and in nanoseconds")]
    RandomAfterIsInThePast {
        block_time: Timestamp,
        random_beacon_after: Timestamp,
    },

    #[error(
        "Requesting randomness is too much in the future, max allowed is {max_allowed_beacon_time}"
    )]
    RandomAfterIsTooMuchInTheFuture { max_allowed_beacon_time: Timestamp },

    #[error("Randomness has already been provided to this raffle")]
    RandomnessAlreadyProvided {},

    #[error("Received invalid randomness")]
    InvalidRandomness,

    #[error("Immutable Randomness")]
    ImmutableRandomness,

    #[error("Unreachable code, something weird happened")]
    Unreachable {},

    #[error("An unplanned bug just happened :/")]
    ContractBug {},

    #[error("Error when parsing a value for {0}")]
    ParseError(String),

    #[error("{0} not found in context")]
    NotFoundError(String),

    #[error("The Message sender has to be the owner of the NFT to prevent hacks")]
    SenderNotOwner {},

    #[error("This action is not allowed, the contract is locked")]
    ContractIsLocked {},

    #[error("Key already exists in RaffleInfo")]
    ExistsInRaffleInfo {},

    #[error("Raffle ID does not exist")]
    NotFoundInRaffleInfo {},

    #[error("You can't buy tickets on this raffle anymore")]
    CantBuyTickets {},

    #[error("A raffle can only be done with CW721 or SG721 assets")]
    WrongAssetType {},

    #[error("Tickets to a raffle can only be bought with native assets.")]
    WrongFundsType {},

    #[error("The sent asset doesn't match the asset in the message sent along with it")]
    AssetMismatch {},

    #[error("Please include at least one asset when creating a raffle")]
    NoAssets {},

    #[error("The sent assets ({assets_received:?}) don't match the required assets ({assets_wanted:?}) for this raffle")]
    PaymentNotSufficient {
        assets_wanted: AssetInfo,
        assets_received: AssetInfo,
    },

    #[error("Too much tickets were already purchased for this raffle. Max : {max:?}, Number before purchase : {nb_before:?}, Number after purchase : {nb_after:?}")]
    TooMuchTickets {
        max: u32,
        nb_before: u32,
        nb_after: u32,
    },

    #[error("Too much tickets were already purchased by this user for this raffle. Max : {max:?}, Number before purchase : {nb_before:?}, Number after purchase : {nb_after:?}")]
    TooMuchTicketsForUser {
        max: u32,
        nb_before: u32,
        nb_after: u32,
    },

    #[error("The provided randomness is invalid current round : {current_round:?}")]
    RandomnessNotAccepted { current_round: u64 },

    #[error("This raffle is not ready to accept new randomness. Only Closed raffles can be decided upon. Current status : {status:?}")]
    WrongStateForRandmness { status: RaffleState },

    #[error("This raffle is not ready to be claimed.  Current status : {status:?}")]
    WrongStateForClaim { status: RaffleState },

    #[error("This raffle cannot be cancelled anymore,   Current status : {status:?}")]
    WrongStateForCancel { status: RaffleState },

    #[error("This raffle has already started.")]
    RaffleAlreadyStarted {},

    #[error("The public key you indicated is invalid")]
    InvalidPubkey {},

    #[error("The randomness signatur is invalid")]
    InvalidSignature {},

    #[error("Wrong Format for the verify response")]
    ParseReplyError {},

    #[error("This parameter name was not found, you can't change it !")]
    ParameterNotFound {},

    #[error("The raffle comment is ({size}) bytes, must be <=  ({max}) bytes")]
    CommentTooLarge{ size: u64, max: u64 },
}
