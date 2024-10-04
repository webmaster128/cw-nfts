// Version info for migration
pub const CONTRACT_NAME: &str = "crates.io:{{project-name}}";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(not(feature = "library"))]
pub mod entry {
    use crate::error::ContractError;
    use crate::msg::{Cw721Contract, ExecuteMsg, InstantiateMsg, QueryMsg};

    use super::*;

    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    // This makes a conscious choice on the various generics used by the contract
    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        // Instantiate the base contract
        Cw721Contract::default().instantiate(deps.branch(), env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        match msg {
            // Optionally override the default cw721-base behavior
            // ExecuteMsg::Burn { token_id } => unimplemented!(),

            // Implment extension messages here, remove if you don't wish to use
            // An ExecuteExt extension
            ExecuteMsg::Extension { msg } => match msg {
                _ => unimplemented!(),
            },

            // Use the default cw721-base implementation
            _ => Cw721Contract::default()
                .execute(deps, env, info, msg)
                .map_err(Into::into),
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            // Optionally override a default cw721-base query
            // QueryMsg::Minter {} => unimplemented!(),
            QueryMsg::Extension { msg } => match msg {
                _ => unimplemented!(),
            },

            // Use default cw721-base query implementation
            _ => Cw721Contract::default().query(deps, env, msg),
        }
    }
}
