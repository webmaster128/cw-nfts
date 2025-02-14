pub mod error;
pub mod msg;
pub mod state;

use cw721::traits::{Cw721Execute, Cw721Query};

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cw721-metadata-onchain";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// This is an opionated `cw721` explicitly defining `NftExtension` for metadata on chain for ease of use.
/// There are 2 possibiities for using metadata on chain:
///
/// cw721-metadata-onchain:
///
/// ```rust
/// // instantiate:
/// let contract = Cw721MetadataContract::default();
/// let info = mock_info(CREATOR, &[]);
/// let init_msg = InstantiateMsg {
///     name: "SpaceShips".to_string(),
///     symbol: "SPACE".to_string(),
///     collection_info_extension: None,
///     minter: None,
///     creator: None,
///     withdraw_address: None,
/// };
/// // ...
/// // mint:
/// let token_id = "Enterprise";
/// let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
/// let extension = Some(NftExtensionMsg {
///     description: Some("description1".into()),
///     name: Some("name1".to_string()),
///     attributes: Some(vec![Trait {
///         display_type: None,
///         trait_type: "type1".to_string(),
///         value: "value1".to_string(),
///     }]),
///     ..NftExtensionMsg::default()
/// });
/// let exec_msg = ExecuteMsg::Mint {
///     token_id: token_id.to_string(),
///     owner: "john".to_string(),
///     token_uri: token_uri.clone(),
///     extension: extension.clone(),
/// };
/// // ...
/// ```
pub type Cw721MetadataContract<'a> = cw721::extension::Cw721OnchainExtensions<'a>;

pub mod entry {
    use super::*;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};
    use cw721::msg::Cw721MigrateMsg;
    use error::ContractError;
    use msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        Cw721MetadataContract::default().instantiate_with_version(
            deps.branch(),
            &env,
            &info,
            msg,
            CONTRACT_NAME,
            CONTRACT_VERSION,
        )
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        Cw721MetadataContract::default().execute(deps, &env, &info, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        Cw721MetadataContract::default().query(deps, &env, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn migrate(
        deps: DepsMut,
        env: Env,
        msg: Cw721MigrateMsg,
    ) -> Result<Response, ContractError> {
        let contract = Cw721MetadataContract::default();
        contract.migrate(deps, env, msg, CONTRACT_NAME, CONTRACT_VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cw721::{
        msg::{Cw721InstantiateMsg, NftExtensionMsg},
        state::Trait,
        NftExtension,
    };
    use msg::ExecuteMsg;

    const CREATOR: &str = "creator";

    /// Make sure cw2 version info is properly initialized during instantiation,
    /// and NOT overwritten by the base contract.
    #[test]
    fn proper_cw2_initialization() {
        let mut deps = mock_dependencies();
        let sender = deps.api.addr_make("sender");
        let info = message_info(&sender, &[]);
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            info,
            Cw721InstantiateMsg {
                name: "collection_name".into(),
                symbol: "collection_symbol".into(),
                collection_info_extension: None,
                minter: None,
                creator: None,
                withdraw_address: None,
            },
        )
        .unwrap();

        let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(version.contract, CONTRACT_NAME);
    }

    #[test]
    fn use_metadata_extension() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();
        let creator = deps.api.addr_make(CREATOR);
        let info = message_info(&creator, &[]);
        let init_msg = Cw721InstantiateMsg {
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
        let extension = Some(NftExtensionMsg {
            description: Some("description1".into()),
            name: Some("name1".to_string()),
            attributes: Some(vec![Trait {
                display_type: None,
                trait_type: "type1".to_string(),
                value: "value1".to_string(),
            }]),
            ..NftExtensionMsg::default()
        });
        let owner = deps.api.addr_make("owner");
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
        assert_eq!(
            res.extension,
            Some(NftExtension {
                description: Some("description1".into()),
                name: Some("name1".to_string()),
                attributes: Some(vec![Trait {
                    display_type: None,
                    trait_type: "type1".to_string(),
                    value: "value1".to_string(),
                }]),
                ..NftExtension::default()
            })
        );
    }
}
