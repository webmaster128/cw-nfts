mod error;
mod execute;
#[allow(deprecated)]
pub mod msg;
mod query;
pub mod state;

#[cfg(test)]
mod contract_tests;

use cosmwasm_std::Empty;
use cw721::DefaultOptionalNftExtension;

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cw721-expiration";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type MinterResponse = cw721::msg::MinterResponse;

pub type NftInfo = cw721::state::NftInfo<DefaultOptionalNftExtension>;

pub mod entry {
    use crate::{
        error::ContractError,
        msg::{InstantiateMsg, QueryMsg},
        state::DefaultCw721ExpirationContract,
    };

    use super::*;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};
    use cw721::{
        msg::Cw721ExecuteMsg, DefaultOptionalCollectionExtensionMsg, DefaultOptionalNftExtensionMsg,
    };

    // This makes a conscious choice on the various generics used by the contract
    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let contract = DefaultCw721ExpirationContract::default();
        contract.instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw721ExecuteMsg<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >,
    ) -> Result<Response, ContractError> {
        let contract = DefaultCw721ExpirationContract::default();
        contract.execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg<Empty>) -> Result<Binary, ContractError> {
        let contract = DefaultCw721ExpirationContract::default();
        contract.query(deps, env, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn migrate(
        deps: DepsMut,
        env: Env,
        msg: cw721::msg::Cw721MigrateMsg,
    ) -> Result<Response, ContractError> {
        let contract = DefaultCw721ExpirationContract::default();
        contract.migrate(deps, env, msg, CONTRACT_NAME, CONTRACT_VERSION)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cw2::ContractVersion;

    use crate::{error::ContractError, msg::InstantiateMsg, state::DefaultCw721ExpirationContract};

    use super::*;

    #[test]
    fn proper_cw2_initialization() {
        let mut deps = mock_dependencies();
        let info = message_info(&deps.api.addr_make("mrt"), &[]);
        let creator = deps.api.addr_make("creator");
        let minter = deps.api.addr_make("minter");
        // assert min expiration
        let error = entry::instantiate(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            InstantiateMsg {
                expiration_days: 0,
                name: "collection_name".into(),
                symbol: "collection_symbol".into(),
                collection_info_extension: None,
                minter: Some(minter.to_string()),
                creator: Some(creator.to_string()),
                withdraw_address: None,
            },
        )
        .unwrap_err();
        assert_eq!(error, ContractError::MinExpiration {});

        // Make sure cw2 version info is properly initialized during instantiation.
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            info,
            InstantiateMsg {
                expiration_days: 1,
                name: "name".into(),
                symbol: "symbol".into(),
                collection_info_extension: None,
                minter: Some(minter.to_string()),
                creator: Some(creator.to_string()),
                withdraw_address: None,
            },
        )
        .unwrap();
        let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(
            version,
            ContractVersion {
                contract: CONTRACT_NAME.into(),
                version: CONTRACT_VERSION.into(),
            },
        );

        assert_eq!(
            1,
            DefaultCw721ExpirationContract::default()
                .expiration_days
                .load(deps.as_ref().storage)
                .unwrap()
        );
    }
}
