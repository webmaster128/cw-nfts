use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, ReplyOn, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw721::helpers::DefaultCw721Helper;
use cw721::msg::{Cw721ExecuteMsg, Cw721InstantiateMsg, NftExtensionMsg};
use cw721::traits::Cw721Calls;
use cw721::{
    DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtensionMsg,
};
use cw_utils::parse_instantiate_response_data;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-fixed-price";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_TOKEN_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg<DefaultOptionalCollectionExtension>,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.unit_price == Uint128::new(0) {
        return Err(ContractError::InvalidUnitPrice {});
    }

    if msg.max_tokens == 0 {
        return Err(ContractError::InvalidMaxTokens {});
    }

    let config = Config {
        cw721_address: None,
        cw20_address: msg.cw20_address,
        unit_price: msg.unit_price,
        max_tokens: msg.max_tokens,
        owner: info.sender,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        token_uri: msg.token_uri.clone(),
        extension: msg.extension.clone(),
        unused_token_id: 0,
    };

    CONFIG.save(deps.storage, &config)?;

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.token_code_id,
            msg: to_json_binary(&Cw721InstantiateMsg {
                name: msg.name.clone(),
                symbol: msg.symbol,
                collection_info_extension: msg.collection_info_extension,
                minter: None,
                creator: None,
                withdraw_address: msg.withdraw_address,
            })?,
            funds: vec![],
            admin: None,
            label: String::from("Instantiate fixed price NFT contract"),
        }
        .into(),
        id: INSTANTIATE_TOKEN_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
        payload: Binary::new(vec![]),
    }];

    Ok(Response::new().add_submessages(sub_msg))
}

// Reply callback triggered from cw721 contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if config.cw721_address.is_some() {
        return Err(ContractError::Cw721AlreadyLinked {});
    }

    if msg.id != INSTANTIATE_TOKEN_REPLY_ID {
        return Err(ContractError::InvalidTokenReplyId {});
    }
    let result = msg.result.into_result().unwrap();
    let data = result.msg_responses.first().unwrap();
    let reply = parse_instantiate_response_data(data.value.as_slice()).unwrap();
    config.cw721_address = Addr::unchecked(reply.contract_address).into();
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner,
        cw20_address: config.cw20_address,
        cw721_address: config.cw721_address,
        max_tokens: config.max_tokens,
        unit_price: config.unit_price,
        name: config.name,
        symbol: config.symbol,
        token_uri: config.token_uri,
        extension: config.extension,
        unused_token_id: config.unused_token_id,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender,
            amount,
            msg,
        }) => execute_receive(deps, info, sender, amount, msg),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    info: MessageInfo,
    sender: String,
    amount: Uint128,
    _msg: Binary,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if config.cw20_address != info.sender {
        return Err(ContractError::UnauthorizedTokenContract {});
    }

    if config.cw721_address.is_none() {
        return Err(ContractError::Uninitialized {});
    }

    if config.unused_token_id >= config.max_tokens {
        return Err(ContractError::SoldOut {});
    }

    if amount != config.unit_price {
        return Err(ContractError::WrongPaymentAmount {});
    }

    let extension: Option<NftExtensionMsg> = config.extension.clone().map(|e| e.into());
    let mint_msg = Cw721ExecuteMsg::<
        DefaultOptionalNftExtensionMsg,
        DefaultOptionalCollectionExtensionMsg,
        Empty,
    >::Mint {
        token_id: config.unused_token_id.to_string(),
        owner: sender,
        token_uri: config.token_uri.clone().into(),
        extension,
    };

    match config.cw721_address.clone() {
        Some(cw721) => {
            let msg = DefaultCw721Helper::new(cw721).call(mint_msg)?;
            config.unused_token_id += 1;
            CONFIG.save(deps.storage, &config)?;

            Ok(Response::new().add_message(msg))
        }
        None => Err(ContractError::Cw721NotLinked {}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        message_info, mock_dependencies, mock_env, MockApi, MOCK_CONTRACT_ADDR,
    };
    use cosmwasm_std::{
        from_json, to_json_binary, CosmosMsg, MsgResponse, SubMsgResponse, SubMsgResult,
    };
    use cw721::DefaultOptionalNftExtensionMsg;
    use prost::Message;

    const NFT_CONTRACT_ADDR: &str = "nftcontract";

    // Type for replies to contract instantiate messes
    #[derive(Clone, PartialEq, Message)]
    struct MsgInstantiateContractResponse {
        #[prost(string, tag = "1")]
        pub contract_address: ::prost::alloc::string::String,
        #[prost(bytes, tag = "2")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }

    pub struct MockAddrFactory<'a> {
        api: MockApi,
        addrs: std::collections::BTreeMap<&'a str, Addr>,
    }

    impl<'a> MockAddrFactory<'a> {
        pub fn new(api: MockApi) -> Self {
            Self {
                api,
                addrs: std::collections::BTreeMap::new(),
            }
        }

        pub fn addr(&mut self, name: &'a str) -> Addr {
            self.addrs
                .entry(name)
                .or_insert(self.api.addr_make(name))
                .clone()
        }
    }

    #[test]
    fn initialization() {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let msg = InstantiateMsg {
            owner: addrs.addr("owner"),
            max_tokens: 1,
            unit_price: Uint128::new(1),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: addrs.addr(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };
        let owner = addrs.addr("owner");
        let info = message_info(&owner, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

        instantiate(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg {
                msg: WasmMsg::Instantiate {
                    code_id: msg.token_code_id,
                    msg: to_json_binary(&Cw721InstantiateMsg {
                        name: msg.name.clone(),
                        symbol: msg.symbol.clone(),
                        collection_info_extension: msg.collection_info_extension,
                        minter: None,
                        creator: None,
                        withdraw_address: None,
                    })
                    .unwrap(),
                    funds: vec![],
                    admin: None,
                    label: String::from("Instantiate fixed price NFT contract"),
                }
                .into(),
                id: INSTANTIATE_TOKEN_REPLY_ID,
                gas_limit: None,
                reply_on: ReplyOn::Success,
                payload: Binary::new(vec![]),
            }]
        );

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: addrs.addr("nftcontract").to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            payload: Binary::default(),
            gas_used: 1000,
            #[allow(deprecated)]
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.clone().into()),
                msg_responses: vec![MsgResponse {
                    type_url: "/cosmwasm.wasm.v1.MsgInstantiateContractResponse".to_string(),
                    value: encoded_instantiate_reply.clone().into(),
                }],
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let query_msg = QueryMsg::GetConfig {};
        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let config: Config = from_json(res).unwrap();
        assert_eq!(
            config,
            Config {
                owner: addrs.addr("owner"),
                cw20_address: msg.cw20_address,
                cw721_address: Some(addrs.addr(NFT_CONTRACT_ADDR)),
                max_tokens: msg.max_tokens,
                unit_price: msg.unit_price,
                name: msg.name,
                symbol: msg.symbol,
                token_uri: msg.token_uri,
                extension: None,
                unused_token_id: 0
            }
        );
    }

    #[test]
    fn invalid_unit_price() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            max_tokens: 1,
            unit_price: Uint128::new(0),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let owner = deps.api.addr_make("owner");
        let info = message_info(&owner, &[]);
        let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::InvalidUnitPrice {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn invalid_max_tokens() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            max_tokens: 0,
            unit_price: Uint128::new(1),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let owner = deps.api.addr_make("owner");
        let info = message_info(&owner, &[]);
        let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::InvalidMaxTokens {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn mint() {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let msg = InstantiateMsg {
            owner: addrs.addr("owner"),
            max_tokens: 1,
            unit_price: Uint128::new(1),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: addrs.addr(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let owner = addrs.addr("owner");
        let info = message_info(&owner, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: addrs.addr(NFT_CONTRACT_ADDR).to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            payload: Binary::default(),
            gas_used: 1000,
            #[allow(deprecated)]
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.clone().into()),
                msg_responses: vec![MsgResponse {
                    type_url: "/cosmwasm.wasm.v1.MsgInstantiateContractResponse".to_string(),
                    value: encoded_instantiate_reply.clone().into(),
                }],
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: addrs.addr("minter").to_string(),
            amount: Uint128::new(1),
            msg: [].into(),
        });
        let contract = addrs.addr(MOCK_CONTRACT_ADDR);
        let info = message_info(&contract, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let mint_msg = Cw721ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::Mint {
            token_id: String::from("0"),
            owner: addrs.addr("minter").to_string(),
            token_uri: Some(String::from("https://ipfs.io/ipfs/Q")),
            extension: None,
        };

        assert_eq!(
            res.messages[0],
            SubMsg {
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: addrs.addr(NFT_CONTRACT_ADDR).to_string(),
                    msg: to_json_binary(&mint_msg).unwrap(),
                    funds: vec![],
                }),
                id: 0,
                gas_limit: None,
                reply_on: ReplyOn::Never,
                payload: Binary::new(vec![])
            }
        );
    }

    #[test]
    fn invalid_reply_id() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            max_tokens: 1,
            unit_price: Uint128::new(1),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let owner = deps.api.addr_make("owner");
        let info = message_info(&owner, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: 10,
            payload: Binary::default(),
            gas_used: 1000,
            #[allow(deprecated)]
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.clone().into()),
                msg_responses: vec![MsgResponse {
                    type_url: "/cosmwasm.wasm.v1.MsgInstantiateContractResponse".to_string(),
                    value: encoded_instantiate_reply.clone().into(),
                }],
            }),
        };
        let err = reply(deps.as_mut(), mock_env(), reply_msg).unwrap_err();
        match err {
            ContractError::InvalidTokenReplyId {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn cw721_already_linked() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            max_tokens: 1,
            unit_price: Uint128::new(1),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let owner = deps.api.addr_make("owner");
        let info = message_info(&owner, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: 1,
            payload: Binary::default(),
            gas_used: 1000,
            #[allow(deprecated)]
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.clone().into()),
                msg_responses: vec![MsgResponse {
                    type_url: "/cosmwasm.wasm.v1.MsgInstantiateContractResponse".to_string(),
                    value: encoded_instantiate_reply.clone().into(),
                }],
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg.clone()).unwrap();

        let err = reply(deps.as_mut(), mock_env(), reply_msg).unwrap_err();
        match err {
            ContractError::Cw721AlreadyLinked {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn sold_out() {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let msg = InstantiateMsg {
            owner: addrs.addr("owner"),
            max_tokens: 1,
            unit_price: Uint128::new(1),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: addrs.addr(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let owner = addrs.addr("owner");
        let info = message_info(&owner, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            payload: Binary::default(),
            gas_used: 1000,
            #[allow(deprecated)]
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.clone().into()),
                msg_responses: vec![MsgResponse {
                    type_url: "/cosmwasm.wasm.v1.MsgInstantiateContractResponse".to_string(),
                    value: encoded_instantiate_reply.clone().into(),
                }],
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: addrs.addr("minter").to_string(),
            amount: Uint128::new(1),
            msg: [].into(),
        });
        let contract = deps.api.addr_make(MOCK_CONTRACT_ADDR);
        let info = message_info(&contract, &[]);

        // Max mint is 1, so second mint request should fail
        execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::SoldOut {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn uninitialized() {
        // Config has not been fully initialized with nft contract address via instantiation reply
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let msg = InstantiateMsg {
            owner: addrs.addr("owner"),
            max_tokens: 1,
            unit_price: Uint128::new(1),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: addrs.addr(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let contract = addrs.addr(MOCK_CONTRACT_ADDR);
        let info = message_info(&contract, &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Test token transfer when nft contract has not been linked

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: addrs.addr("minter").to_string(),
            amount: Uint128::new(1),
            msg: [].into(),
        });

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ContractError::Uninitialized {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn unauthorized_token() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
            max_tokens: 1,
            unit_price: Uint128::new(1),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let owner = deps.api.addr_make("owner");
        let info = message_info(&owner, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Link nft token contract using reply

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            payload: Binary::default(),
            gas_used: 1000,
            #[allow(deprecated)]
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.clone().into()),
                msg_responses: vec![MsgResponse {
                    type_url: "/cosmwasm.wasm.v1.MsgInstantiateContractResponse".to_string(),
                    value: encoded_instantiate_reply.clone().into(),
                }],
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        // Test token transfer from invalid token contract
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: String::from("minter"),
            amount: Uint128::new(1),
            msg: [].into(),
        });
        let info = message_info(&deps.api.addr_make("unauthorized-token"), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::UnauthorizedTokenContract {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn wrong_amount() {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let msg = InstantiateMsg {
            owner: addrs.addr("owner"),
            max_tokens: 1,
            unit_price: Uint128::new(1),
            name: String::from("SYNTH"),
            symbol: String::from("SYNTH"),
            collection_info_extension: None,
            token_code_id: 10u64,
            cw20_address: addrs.addr(MOCK_CONTRACT_ADDR),
            token_uri: String::from("https://ipfs.io/ipfs/Q"),
            extension: None,
            withdraw_address: None,
        };

        let owner = addrs.addr("owner");
        let info = message_info(&owner, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Link nft token contract using reply

        let instantiate_reply = MsgInstantiateContractResponse {
            contract_address: NFT_CONTRACT_ADDR.to_string(),
            data: vec![2u8; 32769],
        };
        let mut encoded_instantiate_reply =
            Vec::<u8>::with_capacity(instantiate_reply.encoded_len());
        instantiate_reply
            .encode(&mut encoded_instantiate_reply)
            .unwrap();

        let reply_msg = Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            payload: Binary::default(),
            gas_used: 1000,
            #[allow(deprecated)]
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(encoded_instantiate_reply.clone().into()),
                msg_responses: vec![MsgResponse {
                    type_url: "/cosmwasm.wasm.v1.MsgInstantiateContractResponse".to_string(),
                    value: encoded_instantiate_reply.clone().into(),
                }],
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        // Test token transfer from invalid token contract
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: addrs.addr("minter").to_string(),
            amount: Uint128::new(100),
            msg: [].into(),
        });
        let contract = addrs.addr(MOCK_CONTRACT_ADDR);
        let info = message_info(&contract, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::WrongPaymentAmount {} => {}
            e => panic!("unexpected error: {e}"),
        }
    }
}
