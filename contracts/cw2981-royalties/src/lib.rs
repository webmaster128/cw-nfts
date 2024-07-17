pub mod error;
pub mod msg;
pub mod query;

pub use query::{check_royalties, query_royalties_info};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Empty};
pub use cw721_base::{
    execute::Cw721Execute, msg::InstantiateMsg, query::Cw721Query, Cw721Contract,
};

use crate::error::ContractError;

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cw2981-royalties";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

// see: https://docs.opensea.io/docs/metadata-standards
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
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
    /// This is how much the minter takes as a cut when sold
    /// royalties are owed on this token if it is Some
    pub royalty_percentage: Option<u64>,
    /// The payment address, may be different to or the same
    /// as the minter addr
    /// question: how do we validate this?
    pub royalty_payment_address: Option<String>,
}

pub type Extension = Option<Metadata>;

pub type MintExtension = Option<Extension>;

pub type Cw2981Contract<'a> = Cw721Contract<'a, Extension, Empty, Empty>;
pub type ExecuteMsg = cw721_base::msg::ExecuteMsg<Extension, Empty>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use self::msg::QueryMsg;

    use super::*;

    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        Ok(Cw2981Contract::default().instantiate(
            deps.branch(),
            env,
            info,
            msg,
            CONTRACT_NAME,
            CONTRACT_VERSION,
        )?)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        if let ExecuteMsg::Mint {
            extension:
                Some(Metadata {
                    royalty_percentage: Some(royalty_percentage),
                    ..
                }),
            ..
        } = &msg
        {
            // validate royalty_percentage to be between 0 and 100
            // no need to check < 0 because royalty_percentage is u64
            if *royalty_percentage > 100 {
                return Err(ContractError::InvalidRoyaltyPercentage);
            }
        }

        Cw2981Contract::default()
            .execute(deps, env, info, msg)
            .map_err(Into::into)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::RoyaltyInfo {
                token_id,
                sale_price,
            } => to_json_binary(&query_royalties_info(deps, env, token_id, sale_price)?),
            QueryMsg::CheckRoyalties {} => to_json_binary(&check_royalties(deps)?),
            _ => Cw2981Contract::default().query(deps, env, msg.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{CheckRoyaltiesResponse, QueryMsg, RoyaltiesInfoResponse};

    use cosmwasm_std::{from_json, Uint128};

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    const CREATOR: &str = "creator";

    #[test]
    fn use_metadata_extension() {
        let mut deps = mock_dependencies();
        let contract = Cw2981Contract::default();
        let creator = deps.api.addr_make(CREATOR);

        let info = mock_info(creator.as_ref(), &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: None,
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
            owner: deps.api.addr_make("john").to_string(),
            token_uri: token_uri.clone(),
            extension: extension.clone(),
        };
        let env = mock_env();
        entry::execute(deps.as_mut(), env.clone(), info, exec_msg).unwrap();

        let res = contract
            .query_nft_info(deps.as_ref(), env, token_id.into())
            .unwrap();
        assert_eq!(res.token_uri, token_uri);
        assert_eq!(res.extension, extension);
    }

    #[test]
    fn validate_royalty_information() {
        let mut deps = mock_dependencies();
        let _contract = Cw2981Contract::default();
        let creator = deps.api.addr_make(CREATOR);

        let info = mock_info(creator.as_ref(), &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: None,
            withdraw_address: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: deps.api.addr_make("john").to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Enterprise".to_string()),
                royalty_percentage: Some(101),
                ..Metadata::default()
            }),
        };
        // mint will return StdError
        let err = entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidRoyaltyPercentage);
    }

    #[test]
    fn check_royalties_response() {
        let mut deps = mock_dependencies();
        let _contract = Cw2981Contract::default();

        let creator = deps.api.addr_make(CREATOR);

        let info = mock_info(creator.as_ref(), &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: None,
            withdraw_address: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: deps.api.addr_make("john").to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Enterprise".to_string()),
                ..Metadata::default()
            }),
        };
        entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let expected = CheckRoyaltiesResponse {
            royalty_payments: true,
        };
        let res = check_royalties(deps.as_ref()).unwrap();
        assert_eq!(res, expected);

        // also check the longhand way
        let query_msg = QueryMsg::CheckRoyalties {};
        let query_res: CheckRoyaltiesResponse =
            from_json(entry::query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(query_res, expected);
    }

    #[test]
    fn check_token_royalties() {
        let mut deps = mock_dependencies();
        let creator = deps.api.addr_make(CREATOR);

        let info = mock_info(creator.as_ref(), &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: None,
            withdraw_address: None,
        };
        let env = mock_env();
        entry::instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let owner = deps.api.addr_make("jeanluc");
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: owner.to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Enterprise".to_string()),
                royalty_payment_address: Some(deps.api.addr_make("jeanluc").to_string()),
                royalty_percentage: Some(10),
                ..Metadata::default()
            }),
        };
        entry::execute(deps.as_mut(), mock_env(), info.clone(), exec_msg).unwrap();

        let expected = RoyaltiesInfoResponse {
            address: owner.to_string(),
            royalty_amount: Uint128::new(10),
        };
        let res = query_royalties_info(
            deps.as_ref(),
            env.clone(),
            token_id.to_string(),
            Uint128::new(100),
        )
        .unwrap();
        assert_eq!(res, expected);

        // also check the longhand way
        let query_msg = QueryMsg::RoyaltyInfo {
            token_id: token_id.to_string(),
            sale_price: Uint128::new(100),
        };
        let query_res: RoyaltiesInfoResponse =
            from_json(entry::query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(query_res, expected);

        // check for rounding down
        // which is the default behaviour
        let voyager_token_id = "Voyager";
        let owner = deps.api.addr_make("janeway");
        let voyager_exec_msg = ExecuteMsg::Mint {
            token_id: voyager_token_id.to_string(),
            owner: owner.to_string(),
            token_uri: Some("https://starships.example.com/Starship/Voyager.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Voyager".to_string()),
                royalty_payment_address: Some(deps.api.addr_make("janeway").to_string()),
                royalty_percentage: Some(4),
                ..Metadata::default()
            }),
        };
        entry::execute(deps.as_mut(), mock_env(), info, voyager_exec_msg).unwrap();

        // 43 x 0.04 (i.e., 4%) should be 1.72
        // we expect this to be rounded down to 1
        let voyager_expected = RoyaltiesInfoResponse {
            address: owner.to_string(),
            royalty_amount: Uint128::new(1),
        };

        let res = query_royalties_info(
            deps.as_ref(),
            env,
            voyager_token_id.to_string(),
            Uint128::new(43),
        )
        .unwrap();
        assert_eq!(res, voyager_expected);
    }
}
