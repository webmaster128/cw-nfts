use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Base(#[from] cw721_base::ContractError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error("NFT is Non-transferable")]
    Unauthorized {},
}
