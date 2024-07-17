pub mod error;
pub mod execute;
pub mod msg;
pub mod query;
pub mod state;

pub use crate::state::Cw721Contract;

// These types are re-exported so that contracts interacting with this
// one don't need a direct dependency on cw_ownable to use the API.
//
// `Action` is used in `ExecuteMsg::UpdateOwnership`, `Ownership` is
// used in `QueryMsg::Ownership`, and `OwnershipError` is used in
// `ContractError::Ownership`.
pub use cw_ownable::{Action, Ownership, OwnershipError};

use cosmwasm_std::Empty;

// Version info for migration
pub const CONTRACT_NAME: &str = "crates.io:cw721-base";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod entry {

    use super::*;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
    use cw721::{
        error::Cw721ContractError,
        execute::Cw721Execute,
        msg::{Cw721ExecuteMsg, Cw721InstantiateMsg, Cw721MigrateMsg, Cw721QueryMsg},
        query::Cw721Query,
        state::DefaultOptionMetadataExtension,
    };

    // This makes a conscious choice on the various generics used by the contract
    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw721InstantiateMsg,
    ) -> Result<Response, Cw721ContractError> {
        let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
        contract.instantiate(deps, env, info, msg, CONTRACT_NAME, CONTRACT_VERSION)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw721ExecuteMsg<DefaultOptionMetadataExtension, Empty>,
    ) -> Result<Response, Cw721ContractError> {
        let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
        contract.execute(deps, env, info, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn query(
        deps: Deps,
        env: Env,
        msg: Cw721QueryMsg<DefaultOptionMetadataExtension>,
    ) -> StdResult<Binary> {
        let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
        contract.query(deps, env, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn migrate(deps: DepsMut,
                   env: Env,
                   msg: Cw721MigrateMsg
    ) -> Result<Response, Cw721ContractError> {
        let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
        contract.migrate(deps, env, msg, CONTRACT_NAME, CONTRACT_VERSION)
    }
}
