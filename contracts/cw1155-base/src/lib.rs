mod contract_tests;
mod error;
mod execute;
mod msg;
mod query;
mod state;

pub use crate::error::ContractError;
pub use crate::msg::{ExecuteMsg, InstantiateMsg, MintMsg};
pub use crate::state::Cw1155Contract;
use cosmwasm_std::Empty;
use cw1155::Cw1155QueryMsg;

// This is a simple type to let us handle empty extensions
pub type Extension = Option<Empty>;

pub mod entry {
    use super::*;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    // This makes a conscious choice on the various generics used by the contract
    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        let tract = Cw1155Contract::<Extension>::default();
        tract.instantiate(deps, env, info, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<Extension>,
    ) -> Result<Response, ContractError> {
        let tract = Cw1155Contract::<Extension>::default();
        tract.execute(deps, env, info, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn query(deps: Deps, env: Env, msg: Cw1155QueryMsg) -> StdResult<Binary> {
        let tract = Cw1155Contract::<Extension>::default();
        tract.query(deps, env, msg)
    }
}
