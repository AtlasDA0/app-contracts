
// Stargaze Cosmwasm
#[cfg(feature = "sg")]
use sg_std::StargazeMsgWrapper;
#[cfg(feature = "sg")]
pub type Response = cosmwasm_std::Response<StargazeMsgWrapper>;
#[cfg(feature = "sg")]
pub type SubMsg = cosmwasm_std::SubMsg<StargazeMsgWrapper>;
#[cfg(feature = "sg")]
pub type CosmosMsg = sg_std::CosmosMsg;

// Vanilla Cosmwasm 
#[cfg(not(feature = "sg"))]
pub type Response = cosmwasm_std::Response;
#[cfg(not(feature = "sg"))]
pub type SubMsg = cosmwasm_std::SubMsg;
#[cfg(not(feature = "sg"))]
pub type CosmosMsg = cosmwasm_std::CosmosMsg;