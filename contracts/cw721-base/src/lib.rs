pub mod error;
pub mod msg;
pub mod state;

// expose so other libs dont need to import cw721
pub use cw721::*;

// These types are re-exported so that contracts interacting with this
// one don't need a direct dependency on cw_ownable to use the API.
//
// `Action` is used in `ExecuteMsg::UpdateMinterOwnership` and `ExecuteMsg::UpdateCreatorOwnership`, `Ownership` is
// used in `QueryMsg::GetMinterOwnership`, `QueryMsg::GetCreatorOwnership`, and `OwnershipError` is used in
// `ContractError::Ownership`.
pub use cw_ownable::{Action, Ownership, OwnershipError};
use extension::Cw721BaseExtensions;

// Version info for migration
pub const CONTRACT_NAME: &str = "crates.io:cw721-base";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[deprecated(
    since = "0.19.0",
    note = "Please use `EmptyOptionalNftExtension` instead"
)]
pub type Extension = EmptyOptionalNftExtension;

pub type Cw721BaseContract<'a> = Cw721BaseExtensions<'a>;

pub mod entry {

    use super::*;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};
    use cw721::traits::{Cw721Execute, Cw721Query};
    use error::ContractError;
    use msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let contract = Cw721BaseContract::default();
        contract.instantiate_with_version(deps, &env, &info, msg, CONTRACT_NAME, CONTRACT_VERSION)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        let contract = Cw721BaseContract::default();
        contract.execute(deps, &env, &info, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        let contract = Cw721BaseContract::default();
        contract.query(deps, &env, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
        let contract = Cw721BaseContract::default();
        contract.migrate(deps, env, msg, CONTRACT_NAME, CONTRACT_VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::{
        testing::{message_info, mock_dependencies, mock_env},
        Empty,
    };

    use cw721::traits::{Cw721Execute, Cw721Query};
    use msg::{ExecuteMsg, InstantiateMsg};

    const CREATOR: &str = "creator";

    // here we test cw721-base can be used with nft extension, test without nft extension is already covered in package tests
    #[test]
    fn use_empty_metadata_extension() {
        let mut deps = mock_dependencies();
        let contract = Cw721BaseExtensions::default();
        let creator = deps.api.addr_make(CREATOR);
        let info = message_info(&creator, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            collection_info_extension: None,
            minter: None,
            creator: None,
            withdraw_address: None,
        };
        contract
            .instantiate(deps.as_mut(), &mock_env(), &info.clone(), init_msg)
            .unwrap();

        let token_id = "Enterprise";
        let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let extension = Some(Empty {});
        let owner = deps.api.addr_make("john");
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: owner.to_string(),
            token_uri: token_uri.clone(),
            extension: extension.clone(),
        };
        contract
            .execute(deps.as_mut(), &mock_env(), &info, exec_msg)
            .unwrap();

        let res = contract
            .query_nft_info(deps.as_ref().storage, token_id.into())
            .unwrap();
        assert_eq!(res.token_uri, token_uri);
        assert_eq!(res.extension, Some(Empty {}));
    }
}
