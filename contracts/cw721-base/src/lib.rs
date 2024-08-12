pub mod error;
pub mod msg;
pub mod state;

// expose so other libs dont need to import cw721
pub use cw721::*;

// These types are re-exported so that contracts interacting with this
// one don't need a direct dependency on cw_ownable to use the API.
//
// `Action` is used in `Cw721ExecuteMsg::UpdateMinterOwnership` and `Cw721ExecuteMsg::UpdateCreatorOwnership`, `Ownership` is
// used in `Cw721QueryMsg::GetMinterOwnership`, `Cw721QueryMsg::GetCreatorOwnership`, and `OwnershipError` is used in
// `Cw721ContractError::Ownership`.
pub use cw_ownable::{Action, Ownership, OwnershipError};

use cosmwasm_std::Empty;

// Version info for migration
pub const CONTRACT_NAME: &str = "crates.io:cw721-base";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[deprecated(
    since = "0.19.0",
    note = "Please use `DefaultOptionNftExtension` instead"
)]
pub type Extension = EmptyOptionalNftExtension;

pub mod entry {

    use super::*;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};
    use cw721::{
        error::Cw721ContractError,
        msg::{Cw721ExecuteMsg, Cw721InstantiateMsg, Cw721MigrateMsg, Cw721QueryMsg},
        traits::{Cw721Execute, Cw721Query},
    };
    use extension::Cw721BaseExtensions;

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw721InstantiateMsg<DefaultOptionalCollectionExtensionMsg>,
    ) -> Result<Response, Cw721ContractError> {
        let contract = Cw721BaseExtensions::default();
        contract.instantiate_with_version(deps, &env, &info, msg, CONTRACT_NAME, CONTRACT_VERSION)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw721ExecuteMsg<
            EmptyOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >,
    ) -> Result<Response, Cw721ContractError> {
        let contract = Cw721BaseExtensions::default();
        contract.execute(deps, &env, &info, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn query(
        deps: Deps,
        env: Env,
        msg: Cw721QueryMsg<EmptyOptionalNftExtension, DefaultOptionalCollectionExtension, Empty>,
    ) -> Result<Binary, Cw721ContractError> {
        let contract = Cw721BaseExtensions::default();
        contract.query(deps, &env, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn migrate(
        deps: DepsMut,
        env: Env,
        msg: Cw721MigrateMsg,
    ) -> Result<Response, Cw721ContractError> {
        let contract = Cw721BaseExtensions::default();
        contract.migrate(deps, env, msg, CONTRACT_NAME, CONTRACT_VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cw721::traits::{Cw721Execute, Cw721Query};
    use extension::Cw721BaseExtensions;
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
