use cosmwasm_schema::cw_serde;
use cosmwasm_std::Empty;

pub use cw721_base::{Cw721Contract, InstantiateMsg as Cw721BaseInstantiateMsg, MinterResponse};
use cw_ownable::{get_ownership, initialize_owner};

pub use crate::msg::InstantiateMsg;

pub mod error;
pub mod msg;

pub use crate::error::ContractError;

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

#[cw_serde]
#[derive(Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
}

pub type Extension = Option<Metadata>;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-non-transferable-with-metadata-onchain";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Cw721NonTransferableContract<'a> = Cw721Contract<'a, Extension, Empty, Empty, Empty>;
pub type ExecuteMsg = cw721_base::ExecuteMsg<Extension, Empty>;
pub type QueryMsg = cw721_base::QueryMsg<Empty>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        initialize_owner(deps.storage, deps.api, msg.admin.as_deref())?;

        let cw721_base_instantiate_msg = Cw721BaseInstantiateMsg {
            name: msg.name,
            symbol: msg.symbol,
            minter: msg.minter,
            withdraw_address: msg.withdraw_address,
        };

        Cw721NonTransferableContract::default().instantiate(
            deps.branch(),
            env,
            info,
            cw721_base_instantiate_msg,
        )?;

        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        Ok(Response::default()
            .add_attribute("contract_name", CONTRACT_NAME)
            .add_attribute("contract_version", CONTRACT_VERSION))
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        let owner = get_ownership(deps.storage)?.owner;
        match owner {
            Some(owner) => {
                if owner != info.sender {
                    return Err(ContractError::Unauthorized {});
                }
                Cw721NonTransferableContract::default()
                    .execute(deps, env, info, msg)
                    .map_err(Into::into)
            }
            None => match msg {
                ExecuteMsg::Mint {
                    token_id,
                    owner,
                    token_uri,
                    extension,
                } => Cw721NonTransferableContract::default()
                    .mint(deps, info, token_id, owner, token_uri, extension)
                    .map_err(Into::into),
                _ => Err(ContractError::Ownership(
                    cw721_base::OwnershipError::NotOwner,
                )),
            },
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        Cw721NonTransferableContract::default().query(deps, env, msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::to_json_binary;
    use cw721::Cw721Query;
    use cw_ownable;

    const CREATOR: &str = "creator";
    const MINTER: &str = "minter";

    #[test]
    fn no_owner() {
        let mut deps = mock_dependencies();
        let contract = Cw721NonTransferableContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            admin: None,
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: Some(MINTER.to_string()),
            withdraw_address: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let extension = Some(Metadata {
            description: Some("Spaceship with Warp Drive".into()),
            name: Some("Starship USS Enterprise".to_string()),
            ..Metadata::default()
        });
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: token_uri.clone(),
            extension: extension.clone(),
        };

        // random cannot mint
        let random = mock_info("random", &[]);
        let err = entry::execute(deps.as_mut(), mock_env(), random, exec_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // minter can mint
        let allowed = mock_info(MINTER, &[]);
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, exec_msg).unwrap();

        // ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, token_uri);
        assert_eq!(res.extension, extension);

        // minter cannot transfer
        let random = mock_info(MINTER, &[]);
        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: String::from("random"),
            token_id: token_id.to_string().clone(),
        };

        let err = entry::execute(deps.as_mut(), mock_env(), random, transfer_msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::Base(cw721_base::ContractError::Ownership(
                cw_ownable::OwnershipError::NotOwner
            ))
        );

        // minter cannot send

        let msg = to_json_binary("You now have the NFT").unwrap();
        let target = String::from("another_contract");
        let send_msg = ExecuteMsg::SendNft {
            contract: target.clone(),
            token_id: token_id.to_string().clone(),
            msg: msg.clone(),
        };

        let random = mock_info(MINTER, &[]);
        let err = entry::execute(deps.as_mut(), mock_env(), random, send_msg.clone()).unwrap_err();
        assert_eq!(
            err,
            ContractError::Base(cw721_base::ContractError::Ownership(
                cw_ownable::OwnershipError::NotOwner
            ))
        );
    }
    #[test]
    fn minter_and_admin_should_be_same() {
        let mut deps = mock_dependencies();
        let contract = Cw721NonTransferableContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            admin: Some(CREATOR.to_string()),
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: Some(MINTER.to_string()),
            withdraw_address: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let extension = Some(Metadata {
            description: Some("Spaceship with Warp Drive".into()),
            name: Some("Starship USS Enterprise".to_string()),
            ..Metadata::default()
        });
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: token_uri.clone(),
            extension: extension.clone(),
        };

        // admin cannot mint
        let random = mock_info(CREATOR, &[]);
        let err = entry::execute(deps.as_mut(), mock_env(), random, exec_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // minter can mint
        let allowed = mock_info(MINTER, &[]);
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, exec_msg).unwrap();

        // ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, token_uri);
        assert_eq!(res.extension, extension);
    }

    #[test]
    fn only_owner_can_mint() {
        let mut deps = mock_dependencies();
        let contract = Cw721NonTransferableContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            admin: Some(CREATOR.to_string()),
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: Some(CREATOR.to_string()),
            withdraw_address: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let extension = Some(Metadata {
            description: Some("Spaceship with Warp Drive".into()),
            name: Some("Starship USS Enterprise".to_string()),
            ..Metadata::default()
        });
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: token_uri.clone(),
            extension: extension.clone(),
        };

        // random cannot mint
        let random = mock_info("random", &[]);
        let err = entry::execute(deps.as_mut(), mock_env(), random, exec_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // creator can mint
        let allowed = mock_info(CREATOR, &[]);
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, exec_msg).unwrap();

        // ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, token_uri);
        assert_eq!(res.extension, extension);
    }

    #[test]
    fn transferring_nft() {
        let mut deps = mock_dependencies();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            admin: Some(CREATOR.to_string()),
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: Some(CREATOR.to_string()),
            withdraw_address: None,
        };

        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // Mint a token
        let token_id = "Enterprise";
        let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let extension = Some(Metadata {
            description: Some("Spaceship with Warp Drive".into()),
            name: Some("Starship USS Enterprise".to_string()),
            ..Metadata::default()
        });
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: token_uri.clone(),
            extension: extension.clone(),
        };

        entry::execute(deps.as_mut(), mock_env(), info.clone(), exec_msg).unwrap();

        // random cannot transfer
        let random = mock_info("random", &[]);
        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: String::from("random"),
            token_id: token_id.to_string().clone(),
        };

        let err = entry::execute(deps.as_mut(), mock_env(), random, transfer_msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // owner of the NFT also cannot transfer, i.e. it is non-transferable
        let john = mock_info("john", &[]);
        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: String::from("random"),
            token_id: token_id.to_string().clone(),
        };

        let err = entry::execute(deps.as_mut(), mock_env(), john, transfer_msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn sending_nft() {
        let mut deps = mock_dependencies();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            admin: Some(CREATOR.to_string()),
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: Some(CREATOR.to_string()),
            withdraw_address: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // Mint a token
        let token_id = "Enterprise";
        let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let extension = Some(Metadata {
            description: Some("Spaceship with Warp Drive".into()),
            name: Some("Starship USS Enterprise".to_string()),
            ..Metadata::default()
        });
        let mint_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: token_uri.clone(),
            extension: extension.clone(),
        };
        entry::execute(deps.as_mut(), mock_env(), info.clone(), mint_msg).unwrap();

        let msg = to_json_binary("You now have the NFT").unwrap();
        let target = String::from("another_contract");
        let send_msg = ExecuteMsg::SendNft {
            contract: target.clone(),
            token_id: token_id.to_string().clone(),
            msg: msg.clone(),
        };

        let random = mock_info("random", &[]);
        let err = entry::execute(deps.as_mut(), mock_env(), random, send_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // owner of the NFT also cannot transfer, i.e. it is non-transferable
        let random = mock_info("venus", &[]);
        let err = entry::execute(deps.as_mut(), mock_env(), random, send_msg.clone()).unwrap_err();

        assert_eq!(err, ContractError::Unauthorized {});
    }
}
