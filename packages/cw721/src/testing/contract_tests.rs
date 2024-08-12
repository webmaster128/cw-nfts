use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};

use cosmwasm_std::{
    from_json, to_json_binary, Addr, Coin, CosmosMsg, DepsMut, Empty, MessageInfo, Response,
    StdError, Timestamp, WasmMsg,
};

use crate::error::Cw721ContractError;
use crate::extension::Cw721OnchainExtensions;
use crate::msg::{
    ApprovalResponse, CollectionExtensionMsg, NftExtensionMsg, NftInfoResponse, OperatorResponse,
    OperatorsResponse, OwnerOfResponse, RoyaltyInfoResponse,
};
use crate::msg::{CollectionInfoMsg, Cw721ExecuteMsg, Cw721InstantiateMsg, Cw721QueryMsg};
use crate::receiver::Cw721ReceiveMsg;
use crate::state::{NftExtension, Trait, CREATOR, MINTER};
use crate::{
    traits::{Cw721Execute, Cw721Query},
    Approval, DefaultOptionalCollectionExtensionMsg, DefaultOptionalNftExtension,
    DefaultOptionalNftExtensionMsg, Expiration,
};
use crate::{CollectionExtension, CollectionInfoAndExtensionResponse, RoyaltyInfo};
use cw_ownable::{Action, Ownership, OwnershipError};

const CONTRACT_NAME: &str = "Magic Power";
const SYMBOL: &str = "MGK";

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

    pub fn info(&mut self, name: &'a str) -> MessageInfo {
        message_info(&self.addr(name), &[])
    }
}

fn setup_contract(
    deps: DepsMut<'_>,
    creator: &Addr,
    minter: &Addr,
) -> Cw721OnchainExtensions<'static> {
    let contract = Cw721OnchainExtensions::default();
    let msg = Cw721InstantiateMsg::<DefaultOptionalCollectionExtensionMsg> {
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        collection_info_extension: None,
        minter: Some(minter.to_string()),
        creator: Some(creator.to_string()),
        withdraw_address: None,
    };
    let info_creator = message_info(creator, &[]);
    let res = contract
        .instantiate_with_version(
            deps,
            &mock_env(),
            &info_creator,
            msg,
            "contract_name",
            "contract_version",
        )
        .unwrap();
    assert_eq!(0, res.messages.len());
    contract
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies();
    let contract = Cw721OnchainExtensions::default();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let msg = Cw721InstantiateMsg {
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        collection_info_extension: None,
        minter: Some(minter.to_string()),
        creator: Some(creator.to_string()),
        withdraw_address: Some(creator.to_string()),
    };
    let info = addrs.info("creator");
    let env = mock_env();

    // we can just call .unwrap() to assert this was a success
    let res = contract
        .instantiate_with_version(
            deps.as_mut(),
            &env,
            &info,
            msg,
            "contract_name",
            "contract_version",
        )
        .unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let minter_ownership = MINTER.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(minter), minter_ownership.owner);
    let creator_ownership = CREATOR.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(creator.clone()), creator_ownership.owner);
    let collection_info = contract
        .query_collection_info_and_extension(deps.as_ref())
        .unwrap();
    assert_eq!(
        collection_info,
        CollectionInfoAndExtensionResponse {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            extension: None,
            updated_at: env.block.time
        }
    );

    let withdraw_address = contract
        .config
        .withdraw_address
        .may_load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(Some(creator.to_string()), withdraw_address);

    let count = contract.query_num_tokens(deps.as_ref().storage).unwrap();
    assert_eq!(0, count.count);

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, None)
        .unwrap();
    assert_eq!(0, tokens.tokens.len());
}

#[test]
fn test_instantiate_with_collection_info_and_extension() {
    let mut deps = mock_dependencies();
    let contract = Cw721OnchainExtensions::default();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let payment = addrs.addr("payment");
    let collection_info_extension_msg = Some(CollectionExtensionMsg {
        description: Some("description".to_string()),
        image: Some("https://moonphases.org".to_string()),
        explicit_content: Some(true),
        external_link: Some("https://moonphases.org/".to_string()),
        start_trading_time: Some(Timestamp::from_seconds(0)),
        royalty_info: Some(RoyaltyInfoResponse {
            payment_address: payment.to_string(),
            share: "0.1".parse().unwrap(),
        }),
    });
    let msg = Cw721InstantiateMsg::<DefaultOptionalCollectionExtensionMsg> {
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        collection_info_extension: collection_info_extension_msg,
        minter: Some(minter.to_string()),
        creator: Some(creator.to_string()),
        withdraw_address: Some(creator.to_string()),
    };
    let info = addrs.info("creator");
    let env = mock_env();

    // we can just call .unwrap() to assert this was a success
    let res = contract
        .instantiate_with_version(
            deps.as_mut(),
            &env,
            &info,
            msg,
            "contract_name",
            "contract_version",
        )
        .unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let minter_ownership = MINTER.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(minter), minter_ownership.owner);
    let creator_ownership = CREATOR.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(creator.clone()), creator_ownership.owner);
    let info = contract
        .query_collection_info_and_extension(deps.as_ref())
        .unwrap();
    let collection_info_extension_expected = Some(CollectionExtension {
        description: "description".to_string(),
        image: "https://moonphases.org".to_string(),
        explicit_content: Some(true),
        external_link: Some("https://moonphases.org/".to_string()),
        start_trading_time: Some(Timestamp::from_seconds(0)),
        royalty_info: Some(RoyaltyInfo {
            payment_address: payment,
            share: "0.1".parse().unwrap(),
        }),
    });
    assert_eq!(
        info,
        CollectionInfoAndExtensionResponse {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            extension: collection_info_extension_expected,
            updated_at: env.block.time
        }
    );

    let withdraw_address = contract
        .config
        .withdraw_address
        .may_load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(Some(creator.to_string()), withdraw_address);

    let count = contract.query_num_tokens(deps.as_ref().storage).unwrap();
    assert_eq!(0, count.count);

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, None)
        .unwrap();
    assert_eq!(0, tokens.tokens.len());
}

#[test]
fn test_instantiate_with_minimal_collection_info_and_extension() {
    let mut deps = mock_dependencies();
    let contract = Cw721OnchainExtensions::default();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let collection_info_extension_msg = Some(CollectionExtensionMsg {
        description: Some("description".to_string()),
        image: Some("https://moonphases.org".to_string()),
        explicit_content: None,
        external_link: None,
        start_trading_time: None,
        royalty_info: None,
    });
    let msg = Cw721InstantiateMsg::<DefaultOptionalCollectionExtensionMsg> {
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        collection_info_extension: collection_info_extension_msg,
        minter: Some(minter.to_string()),
        creator: Some(creator.to_string()),
        withdraw_address: Some(creator.to_string()),
    };
    let info = addrs.info("creator");
    let env = mock_env();

    // we can just call .unwrap() to assert this was a success
    let res = contract
        .instantiate_with_version(
            deps.as_mut(),
            &env,
            &info,
            msg,
            "contract_name",
            "contract_version",
        )
        .unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let minter_ownership = MINTER.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(minter), minter_ownership.owner);
    let creator_ownership = CREATOR.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(creator), creator_ownership.owner);
    let info = contract
        .query_collection_info_and_extension(deps.as_ref())
        .unwrap();
    let collection_info_extension_expected = Some(CollectionExtension {
        description: "description".to_string(),
        image: "https://moonphases.org".to_string(),
        explicit_content: None,
        external_link: None,
        start_trading_time: None,
        royalty_info: None,
    });
    assert_eq!(
        info,
        CollectionInfoAndExtensionResponse {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            extension: collection_info_extension_expected,
            updated_at: env.block.time
        }
    );
}

#[test]
fn test_mint() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    let token_id1 = "petrify".to_string();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id1.clone(),
        owner: String::from("medusa"),
        token_uri: Some("invalid_uri".to_string()),
        extension: None,
    };

    // invalid token uri
    let env = mock_env();
    let info_minter = addrs.info("minter");
    let err = contract
        .execute(deps.as_mut(), &env, &info_minter, mint_msg)
        .unwrap_err();
    assert_eq!(
        err,
        Cw721ContractError::ParseError(url::ParseError::RelativeUrlWithoutBase)
    );

    // random cannot mint
    let info_random = addrs.info("random");
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id1.clone(),
        owner: String::from("medusa"),
        token_uri: Some(token_uri.clone()),
        extension: None,
    };
    let err = contract
        .execute(deps.as_mut(), &env, &info_random, mint_msg.clone())
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::NotMinter {});

    // minter can mint
    let info_minter = addrs.info("minter");
    let _ = contract
        .execute(deps.as_mut(), &env, &info_minter, mint_msg)
        .unwrap();

    // ensure num tokens increases
    let count = contract.query_num_tokens(deps.as_ref().storage).unwrap();
    assert_eq!(1, count.count);

    // unknown nft returns error
    let _ = contract
        .query_nft_info(deps.as_ref().storage, "unknown".to_string())
        .unwrap_err();

    // this nft info is correct
    let info = contract
        .query_nft_info(deps.as_ref().storage, token_id1.clone())
        .unwrap();
    assert_eq!(
        info,
        NftInfoResponse::<DefaultOptionalNftExtension> {
            token_uri: Some(token_uri),
            extension: None,
        }
    );

    // owner info is correct
    let owner = contract
        .query_owner_of(deps.as_ref(), &mock_env(), token_id1.clone(), true)
        .unwrap();
    assert_eq!(
        owner,
        OwnerOfResponse {
            owner: String::from("medusa"),
            approvals: vec![],
        }
    );

    // Cannot mint same token_id again
    let mint_msg2 = Cw721ExecuteMsg::Mint {
        token_id: token_id1.clone(),
        owner: String::from("hercules"),
        token_uri: None,
        extension: None,
    };

    let err = contract
        .execute(deps.as_mut(), &mock_env(), &info_minter, mint_msg2)
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Claimed {});

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, None)
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id1.clone()], tokens.tokens);

    // minter mints another one
    let token_id2 = "id2".to_string();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id2.clone(),
        owner: String::from("medusa"),
        token_uri: Some("".to_string()), // empty token uri
        extension: None,
    };
    let _ = contract
        .execute(deps.as_mut(), &env, &info_minter, mint_msg)
        .unwrap();

    // ensure num tokens increases
    let count = contract.query_num_tokens(deps.as_ref().storage).unwrap();
    assert_eq!(2, count.count);

    // unknown nft returns error
    let _ = contract
        .query_nft_info(deps.as_ref().storage, "unknown".to_string())
        .unwrap_err();

    // this nft info is correct
    let info = contract
        .query_nft_info(deps.as_ref().storage, token_id2.clone())
        .unwrap();
    assert_eq!(
        info,
        NftInfoResponse::<DefaultOptionalNftExtension> {
            token_uri: None,
            extension: None,
        }
    );

    // owner info is correct
    let owner = contract
        .query_owner_of(deps.as_ref(), &mock_env(), token_id2.clone(), true)
        .unwrap();
    assert_eq!(
        owner,
        OwnerOfResponse {
            owner: String::from("medusa"),
            approvals: vec![],
        }
    );

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, None)
        .unwrap();
    assert_eq!(2, tokens.tokens.len());
    assert_eq!(vec![token_id2.clone(), token_id1.clone()], tokens.tokens);

    // minter mints another one
    let token_id3 = "id3".to_string();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id3.clone(),
        owner: String::from("medusa"),
        token_uri: None, // empty token uri
        extension: None,
    };
    let _ = contract
        .execute(deps.as_mut(), &env, &info_minter, mint_msg)
        .unwrap();

    // ensure num tokens increases
    let count = contract.query_num_tokens(deps.as_ref().storage).unwrap();
    assert_eq!(3, count.count);

    // unknown nft returns error
    let _ = contract
        .query_nft_info(deps.as_ref().storage, "unknown".to_string())
        .unwrap_err();

    // this nft info is correct
    let info = contract
        .query_nft_info(deps.as_ref().storage, token_id3.clone())
        .unwrap();
    assert_eq!(
        info,
        NftInfoResponse::<DefaultOptionalNftExtension> {
            token_uri: None,
            extension: None,
        }
    );

    // owner info is correct
    let owner = contract
        .query_owner_of(deps.as_ref(), &mock_env(), token_id3.clone(), true)
        .unwrap();
    assert_eq!(
        owner,
        OwnerOfResponse {
            owner: String::from("medusa"),
            approvals: vec![],
        }
    );

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, None)
        .unwrap();
    assert_eq!(3, tokens.tokens.len());
    assert_eq!(vec![token_id2, token_id3, token_id1], tokens.tokens);
}

#[test]
fn test_update_nft_info() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    let token_id = "1".to_string();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: String::from("owner"),
        token_uri: Some("ipfs://foo.bar".to_string()),
        extension: None,
    };

    // mint nft
    let info_minter = addrs.info("minter");
    let env = mock_env();
    contract
        .execute(deps.as_mut(), &env, &info_minter, mint_msg)
        .unwrap();

    // minter update unknown nft info
    let update_msg = Cw721ExecuteMsg::<
        DefaultOptionalNftExtensionMsg,
        DefaultOptionalCollectionExtensionMsg,
        Empty,
    >::UpdateNftInfo {
        token_id: "unknown".to_string(),
        token_uri: Some("ipfs://to.the.moon".to_string()),
        extension: None,
    };
    // throws NotFound error
    contract
        .execute(deps.as_mut(), &env, &info_minter, update_msg)
        .unwrap_err();

    // minter udpate nft info
    let update_msg_without_extension = Cw721ExecuteMsg::<
        DefaultOptionalNftExtensionMsg,
        DefaultOptionalCollectionExtensionMsg,
        Empty,
    >::UpdateNftInfo {
        token_id: token_id.clone(),
        token_uri: Some("".to_string()), // sets token uri to none
        extension: None,
    };
    let err = contract
        .execute(
            deps.as_mut(),
            &env,
            &info_minter,
            update_msg_without_extension.clone(),
        )
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::NotCreator {});

    // other udpate nft metadata extension
    let update_msg_only_extension = Cw721ExecuteMsg::<
        DefaultOptionalNftExtensionMsg,
        DefaultOptionalCollectionExtensionMsg,
        Empty,
    >::UpdateNftInfo {
        token_id: token_id.clone(),
        token_uri: None,
        extension: Some(NftExtensionMsg {
            image: Some("ipfs://foo.bar/image.png".to_string()),
            image_data: None,
            external_url: None,
            description: None,
            name: None,
            attributes: None,
            background_color: None,
            animation_url: None,
            youtube_url: None,
        }),
    };
    let info_other = addrs.info("other");
    let err = contract
        .execute(
            deps.as_mut(),
            &env,
            &info_other,
            update_msg_only_extension.clone(),
        )
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::NotCreator {});

    // creator updates nft info
    let creator_info = addrs.info("creator");
    contract
        .execute(
            deps.as_mut(),
            &env,
            &creator_info,
            update_msg_without_extension,
        )
        .unwrap();
    assert_eq!(
        contract
            .query_nft_info(deps.as_ref().storage, token_id.clone())
            .unwrap(),
        NftInfoResponse {
            token_uri: None,
            extension: None,
        }
    );

    // creator updates nft metadata extension
    contract
        .execute(
            deps.as_mut(),
            &env,
            &creator_info,
            update_msg_only_extension,
        )
        .unwrap();
    assert_eq!(
        contract
            .query_nft_info(deps.as_ref().storage, token_id)
            .unwrap(),
        NftInfoResponse {
            token_uri: None,
            extension: Some(NftExtension {
                image: Some("ipfs://foo.bar/image.png".to_string()),
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: None,
                background_color: None,
                animation_url: None,
                youtube_url: None,
            }),
        }
    );
}

#[test]
fn test_mint_with_metadata() {
    // case 1: mint with valid metadata
    {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let creator = addrs.addr("creator");
        let minter = addrs.addr("minter");
        let contract = setup_contract(deps.as_mut(), &creator, &minter);

        let token_id = "1".to_string();
        let token_uri = "ipfs://foo.bar".to_string();
        let valid_extension_msg = NftExtensionMsg {
            image: Some("ipfs://foo.bar/image.png".to_string()),
            image_data: Some("image data".to_string()),
            external_url: Some("https://github.com".to_string()),
            description: Some("description".to_string()),
            name: Some("name".to_string()),
            attributes: Some(vec![Trait {
                trait_type: "trait_type".to_string(),
                value: "value".to_string(),
                display_type: Some("display_type".to_string()),
            }]),
            background_color: Some("background_color".to_string()),
            animation_url: Some("ssl://animation_url".to_string()),
            youtube_url: Some("file://youtube_url".to_string()),
        };
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: Some(token_uri),
            extension: Some(valid_extension_msg.clone()),
        };

        let info_minter = addrs.info("minter");
        let env = mock_env();
        contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap();
        assert_eq!(
            contract
                .query_nft_info(deps.as_ref().storage, token_id)
                .unwrap(),
            NftInfoResponse {
                token_uri: Some("ipfs://foo.bar".to_string()),
                extension: Some(valid_extension_msg.clone().into()),
            }
        );

        // mint with empty token uri and empty extension
        let mint_msg = Cw721ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::Mint {
            token_id: "2".to_string(),
            owner: String::from("medusa"),
            token_uri: None,
            extension: Some(NftExtensionMsg {
                image: None,
                image_data: None,
                external_url: None,
                description: None,
                name: None,
                attributes: None,
                background_color: None,
                animation_url: None,
                youtube_url: None,
            }),
        };
        contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap();
        assert_eq!(
            contract
                .query_nft_info(deps.as_ref().storage, "2".to_string())
                .unwrap(),
            NftInfoResponse {
                token_uri: None,
                extension: Some(NftExtension {
                    image: None,
                    image_data: None,
                    external_url: None,
                    description: None,
                    name: None,
                    attributes: None,
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                }),
            }
        );
        // empty description
        let token_id = "3".to_string();
        let mut metadata = valid_extension_msg.clone();
        metadata.description = Some("".to_string());
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: None,
            extension: Some(metadata),
        };
        contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap();
        // empty name
        let token_id = "4".to_string();
        let mut metadata = valid_extension_msg.clone();
        metadata.name = Some("".to_string());
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: None,
            extension: Some(metadata),
        };
        contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap();
        // empty background color
        let token_id = "5".to_string();
        let mut metadata = valid_extension_msg.clone();
        metadata.background_color = Some("".to_string());
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: None,
            extension: Some(metadata),
        };
        contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap();
    }
    // case 2: mint with invalid metadata
    {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let creator = addrs.addr("creator");
        let minter = addrs.addr("minter");
        let contract = setup_contract(deps.as_mut(), &creator, &minter);

        let token_id = "1".to_string();
        let token_uri = "ipfs://foo.bar".to_string();
        let info_minter = addrs.info("minter");
        let env = mock_env();

        let valid_extension_msg = NftExtensionMsg {
            image: Some("ipfs://foo.bar/image.png".to_string()),
            image_data: Some("image data".to_string()),
            external_url: Some("https://github.com".to_string()),
            description: Some("description".to_string()),
            name: Some("name".to_string()),
            attributes: Some(vec![Trait {
                trait_type: "trait_type".to_string(),
                value: "value".to_string(),
                display_type: Some("display_type".to_string()),
            }]),
            background_color: Some("background_color".to_string()),
            animation_url: Some("ssl://animation_url".to_string()),
            youtube_url: Some("file://youtube_url".to_string()),
        };

        // invalid image
        let mut metadata = valid_extension_msg.clone();
        metadata.image = Some("invalid".to_string());
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: Some(token_uri.clone()),
            extension: Some(metadata),
        };
        let err = contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::ParseError(url::ParseError::RelativeUrlWithoutBase)
        );
        // invalid external url
        let mut metadata = valid_extension_msg.clone();
        metadata.external_url = Some("invalid".to_string());
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: Some(token_uri.clone()),
            extension: Some(metadata),
        };
        let err = contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::ParseError(url::ParseError::RelativeUrlWithoutBase)
        );
        // invalid animation url
        let mut metadata = valid_extension_msg.clone();
        metadata.animation_url = Some("invalid".to_string());
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: Some(token_uri.clone()),
            extension: Some(metadata),
        };
        let err = contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::ParseError(url::ParseError::RelativeUrlWithoutBase)
        );
        // invalid youtube url
        let mut metadata = valid_extension_msg.clone();
        metadata.youtube_url = Some("invalid".to_string());
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: Some(token_uri.clone()),
            extension: Some(metadata),
        };
        let err = contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::ParseError(url::ParseError::RelativeUrlWithoutBase)
        );

        // empty image data
        let mut metadata = valid_extension_msg.clone();
        metadata.image_data = Some("".to_string());
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: Some(token_uri.clone()),
            extension: Some(metadata),
        };
        contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap();
        // trait type empty
        let mut metadata = valid_extension_msg.clone();
        metadata.attributes = Some(vec![Trait {
            trait_type: "".to_string(),
            value: "value".to_string(),
            display_type: Some("display_type".to_string()),
        }]);
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: Some(token_uri.clone()),
            extension: Some(metadata),
        };
        let err = contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::TraitTypeEmpty {});
        // trait value empty
        let mut metadata = valid_extension_msg.clone();
        metadata.attributes = Some(vec![Trait {
            trait_type: "trait_type".to_string(),
            value: "".to_string(),
            display_type: Some("display_type".to_string()),
        }]);
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.clone(),
            owner: String::from("medusa"),
            token_uri: Some(token_uri.clone()),
            extension: Some(metadata),
        };
        let err = contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::TraitValueEmpty {});
        // display type empty
        let mut metadata = valid_extension_msg;
        metadata.attributes = Some(vec![Trait {
            trait_type: "trait_type".to_string(),
            value: "value".to_string(),
            display_type: Some("".to_string()),
        }]);
        let mint_msg = Cw721ExecuteMsg::Mint {
            token_id,
            owner: String::from("medusa"),
            token_uri: Some(token_uri),
            extension: Some(metadata),
        };
        let err = contract
            .execute(deps.as_mut(), &env, &info_minter, mint_msg)
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::TraitDisplayTypeEmpty {});
    }
}

#[test]
fn test_update_collection_info() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    let update_collection_info_msg = Cw721ExecuteMsg::UpdateCollectionInfo {
        collection_info: CollectionInfoMsg {
            name: Some("new name".to_string()),
            symbol: Some("NEW".to_string()),
            extension: None,
        },
    };

    // Creator can update collection info
    let creator_info = addrs.info("creator");
    let _ = contract
        .execute(
            deps.as_mut(),
            &mock_env(),
            &creator_info,
            update_collection_info_msg,
        )
        .unwrap();

    // Update the owner to "random". The new owner should be able to
    // mint new tokens, the old one should not.
    let random = addrs.addr("random");
    contract
        .execute(
            deps.as_mut(),
            &mock_env(),
            &creator_info,
            Cw721ExecuteMsg::UpdateCreatorOwnership(Action::TransferOwnership {
                new_owner: random.to_string(),
                expiry: None,
            }),
        )
        .unwrap();

    // Creator does not change until ownership transfer completes.
    // Pending ownership transfer should be discoverable via query.
    let ownership: Ownership<Addr> = from_json(
        contract
            .query(
                deps.as_ref(),
                &mock_env(),
                Cw721QueryMsg::GetCreatorOwnership {},
            )
            .unwrap(),
    )
    .unwrap();

    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(creator),
            pending_owner: Some(random),
            pending_expiry: None,
        }
    );

    // Accept the ownership transfer.
    let random_info = addrs.info("random");
    contract
        .execute(
            deps.as_mut(),
            &mock_env(),
            &random_info,
            Cw721ExecuteMsg::UpdateCreatorOwnership(Action::AcceptOwnership),
        )
        .unwrap();

    // Creator changes after ownership transfer is accepted.
    let creator_ownership: Ownership<Addr> = from_json(
        contract
            .query(
                deps.as_ref(),
                &mock_env(),
                Cw721QueryMsg::GetCreatorOwnership {},
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(creator_ownership.owner, Some(random_info.sender.clone()));

    let update_collection_info_msg = Cw721ExecuteMsg::UpdateCollectionInfo {
        collection_info: CollectionInfoMsg {
            name: Some("new name".to_string()),
            symbol: Some("NEW".to_string()),
            extension: None,
        },
    };

    // Old owner can not update.
    let err: Cw721ContractError = contract
        .execute(
            deps.as_mut(),
            &mock_env(),
            &creator_info,
            update_collection_info_msg.clone(),
        )
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::NotCreator {});

    // New owner can update.
    let _ = contract
        .execute(
            deps.as_mut(),
            &mock_env(),
            &random_info,
            update_collection_info_msg,
        )
        .unwrap();
}

#[test]
fn test_update_minter() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    let token_id = "petrify".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id,
        owner: String::from("medusa"),
        token_uri: Some(token_uri.clone()),
        extension: None,
    };

    // Minter can mint
    let current_minter_info = addrs.info("minter");
    let _ = contract
        .execute(deps.as_mut(), &mock_env(), &current_minter_info, mint_msg)
        .unwrap();

    // Update the owner to "random". The new owner should be able to
    // mint new tokens, the old one should not.
    let random = addrs.addr("random");
    contract
        .execute(
            deps.as_mut(),
            &mock_env(),
            &current_minter_info,
            Cw721ExecuteMsg::UpdateMinterOwnership(Action::TransferOwnership {
                new_owner: random.to_string(),
                expiry: None,
            }),
        )
        .unwrap();

    // Minter does not change until ownership transfer completes.
    // Pending ownership transfer should be discoverable via query.
    let ownership: Ownership<Addr> = from_json(
        contract
            .query(
                deps.as_ref(),
                &mock_env(),
                Cw721QueryMsg::GetMinterOwnership {},
            )
            .unwrap(),
    )
    .unwrap();

    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(minter),
            pending_owner: Some(random),
            pending_expiry: None,
        }
    );

    // Accept the ownership transfer.
    let new_minter_info = addrs.info("random");
    contract
        .execute(
            deps.as_mut(),
            &mock_env(),
            &new_minter_info,
            Cw721ExecuteMsg::UpdateMinterOwnership(Action::AcceptOwnership),
        )
        .unwrap();

    // Minter changes after ownership transfer is accepted.
    let minter_ownership: Ownership<Addr> = from_json(
        contract
            .query(
                deps.as_ref(),
                &mock_env(),
                Cw721QueryMsg::GetMinterOwnership {},
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(minter_ownership.owner, Some(new_minter_info.sender.clone()));

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: "randoms_token".to_string(),
        owner: String::from("medusa"),
        token_uri: Some(token_uri),
        extension: None,
    };

    // Old owner can not mint.
    let err: Cw721ContractError = contract
        .execute(
            deps.as_mut(),
            &mock_env(),
            &current_minter_info,
            mint_msg.clone(),
        )
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::NotMinter {});

    // New owner can mint.
    let _ = contract
        .execute(deps.as_mut(), &mock_env(), &new_minter_info, mint_msg)
        .unwrap();
}

#[test]
fn test_burn() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    let token_id = "petrify".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: minter.to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    let burn_msg = Cw721ExecuteMsg::Burn { token_id };

    // mint some NFT
    let allowed = message_info(&minter, &[]);
    let _ = contract
        .execute(deps.as_mut(), &mock_env(), &allowed, mint_msg)
        .unwrap();

    // random not allowed to burn
    let random = message_info(&deps.api.addr_make("random"), &[]);
    let env = mock_env();
    let err = contract
        .execute(deps.as_mut(), &env, &random, burn_msg.clone())
        .unwrap_err();

    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    let _ = contract
        .execute(deps.as_mut(), &env, &allowed, burn_msg)
        .unwrap();

    // ensure num tokens decreases
    let count = contract.query_num_tokens(deps.as_ref().storage).unwrap();
    assert_eq!(0, count.count);

    // trying to get nft returns error
    let _ = contract
        .query_nft_info(deps.as_ref().storage, "petrify".to_string())
        .unwrap_err();

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, None)
        .unwrap();
    assert!(tokens.tokens.is_empty());
}

#[test]
fn test_transfer_nft() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    // Mint a token
    let token_id = "melt".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/melt".to_string();
    let venus = addrs.addr("venus");
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: venus.to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    let minter_info = message_info(&minter, &[]);
    contract
        .execute(deps.as_mut(), &mock_env(), &minter_info, mint_msg)
        .unwrap();

    // random cannot transfer
    let random = addrs.addr("random");
    let random_info = addrs.info("random");
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: random.to_string(),
        token_id: token_id.clone(),
    };

    let err = contract
        .execute(deps.as_mut(), &mock_env(), &random_info, transfer_msg)
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // owner can
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: random.to_string(),
        token_id: token_id.clone(),
    };

    let res = contract
        .execute(deps.as_mut(), &mock_env(), &random_info, transfer_msg)
        .unwrap();

    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", venus.to_string())
            .add_attribute("recipient", "random")
            .add_attribute("token_id", token_id)
    );
}

#[test]
fn test_send_nft() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    // Mint a token
    let token_id = "melt".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/melt".to_string();
    let venus = addrs.addr("venus");
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: venus.to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    contract
        .execute(deps.as_mut(), &mock_env(), &addrs.info("minter"), mint_msg)
        .unwrap();

    let msg = to_json_binary("You now have the melting power").unwrap();
    let target = String::from("another_contract");
    let send_msg = Cw721ExecuteMsg::SendNft {
        contract: target.clone(),
        token_id: token_id.clone(),
        msg: msg.clone(),
    };

    let err = contract
        .execute(
            deps.as_mut(),
            &mock_env(),
            &addrs.info("random"),
            send_msg.clone(),
        )
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // but owner can
    let res = contract
        .execute(deps.as_mut(), &mock_env(), &addrs.info("venus"), send_msg)
        .unwrap();

    let payload = Cw721ReceiveMsg {
        sender: venus.to_string(),
        token_id: token_id.clone(),
        msg,
    };
    let expected = payload.into_cosmos_msg(target.clone()).unwrap();
    // ensure expected serializes as we think it should
    match &expected {
        CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) => {
            assert_eq!(contract_addr, &target)
        }
        m => panic!("Unexpected message type: {m:?}"),
    }
    // and make sure this is the request sent by the contract
    assert_eq!(
        res,
        Response::new()
            .add_message(expected)
            .add_attribute("action", "send_nft")
            .add_attribute("sender", venus.to_string())
            .add_attribute("recipient", "another_contract")
            .add_attribute("token_id", token_id)
    );
}

#[test]
fn test_approve_revoke() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    // Mint a token
    let token_id = "grow".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/grow".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: String::from("demeter"),
        token_uri: Some(token_uri),
        extension: None,
    };

    let minter_info = addrs.info("minter");
    contract
        .execute(deps.as_mut(), &mock_env(), &minter_info, mint_msg)
        .unwrap();

    // token owner shows in approval query
    let res = contract
        .query_approval(
            deps.as_ref(),
            &mock_env(),
            token_id.clone(),
            String::from("demeter"),
            false,
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovalResponse {
            approval: Approval {
                spender: Addr::unchecked("demeter"),
                expires: Expiration::Never {}
            }
        }
    );

    // Give random transferring power
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: String::from("random"),
        token_id: token_id.clone(),
        expires: None,
    };
    let owner_info = addrs.info("demeter");
    let res = contract
        .execute(deps.as_mut(), &mock_env(), &owner_info, approve_msg)
        .unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "approve")
            .add_attribute("sender", addrs.addr("demeter").to_string())
            .add_attribute("spender", "random")
            .add_attribute("token_id", token_id.clone())
    );

    // test approval query
    let res = contract
        .query_approval(
            deps.as_ref(),
            &mock_env(),
            token_id.clone(),
            String::from("random"),
            true,
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovalResponse {
            approval: Approval {
                spender: Addr::unchecked("random"),
                expires: Expiration::Never {}
            }
        }
    );

    // random can now transfer
    let random = addrs.addr("random");
    let random_info = addrs.info("random");
    let person = addrs.addr("person");
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: person.to_string(),
        token_id: token_id.clone(),
    };
    contract
        .execute(deps.as_mut(), &mock_env(), &random_info, transfer_msg)
        .unwrap();

    // Approvals are removed / cleared
    let query_msg = Cw721QueryMsg::OwnerOf {
        token_id: token_id.clone(),
        include_expired: None,
    };
    let res: OwnerOfResponse = from_json(
        contract
            .query(deps.as_ref(), &mock_env(), query_msg.clone())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        res,
        OwnerOfResponse {
            owner: person.to_string(),
            approvals: vec![],
        }
    );

    // Approve, revoke, and check for empty, to test revoke
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: random.to_string(),
        token_id: token_id.clone(),
        expires: None,
    };
    let owner_info = addrs.info("person");
    contract
        .execute(deps.as_mut(), &mock_env(), &owner_info, approve_msg)
        .unwrap();

    let revoke_msg = Cw721ExecuteMsg::Revoke {
        spender: String::from("random"),
        token_id,
    };
    contract
        .execute(deps.as_mut(), &mock_env(), &owner_info, revoke_msg)
        .unwrap();

    // Approvals are now removed / cleared
    let res: OwnerOfResponse = from_json(
        contract
            .query(deps.as_ref(), &mock_env(), query_msg)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        res,
        OwnerOfResponse {
            owner: person.to_string(),
            approvals: vec![],
        }
    );
}

#[test]
fn test_approve_all_revoke_all() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    // Mint a couple tokens (from the same owner)
    let token_id1 = "grow1".to_string();
    let token_uri1 = "https://www.merriam-webster.com/dictionary/grow1".to_string();

    let token_id2 = "grow2".to_string();
    let token_uri2 = "https://www.merriam-webster.com/dictionary/grow2".to_string();

    let mint_msg1 = Cw721ExecuteMsg::Mint {
        token_id: token_id1.clone(),
        owner: String::from("demeter"),
        token_uri: Some(token_uri1),
        extension: None,
    };

    let minter_info = addrs.info("minter");
    contract
        .execute(deps.as_mut(), &mock_env(), &minter_info, mint_msg1)
        .unwrap();

    let mint_msg2 = Cw721ExecuteMsg::Mint {
        token_id: token_id2.clone(),
        owner: String::from("demeter"),
        token_uri: Some(token_uri2),
        extension: None,
    };

    let env = mock_env();
    contract
        .execute(deps.as_mut(), &env, &minter_info, mint_msg2)
        .unwrap();

    // paginate the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, Some(1))
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id1.clone()], tokens.tokens);
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, Some(token_id1.clone()), Some(3))
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id2.clone()], tokens.tokens);

    // demeter gives random full (operator) power over her tokens
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: String::from("random"),
        expires: None,
    };
    let owner_info = addrs.info("demeter");
    let res = contract
        .execute(deps.as_mut(), &mock_env(), &owner_info, approve_all_msg)
        .unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "approve_all")
            .add_attribute("sender", addrs.addr("demeter").to_string())
            .add_attribute("operator", "random")
    );

    // random can now transfer
    let random_info = addrs.info("random");
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: String::from("person"),
        token_id: token_id1,
    };
    contract
        .execute(deps.as_mut(), &mock_env(), &random_info, transfer_msg)
        .unwrap();

    // random can now send
    let inner_msg = WasmMsg::Execute {
        contract_addr: "another_contract".into(),
        msg: to_json_binary("You now also have the growing power").unwrap(),
        funds: vec![],
    };
    let msg: CosmosMsg = CosmosMsg::Wasm(inner_msg);

    let send_msg = Cw721ExecuteMsg::SendNft {
        contract: String::from("another_contract"),
        token_id: token_id2,
        msg: to_json_binary(&msg).unwrap(),
    };
    contract
        .execute(deps.as_mut(), &mock_env(), &random_info, send_msg)
        .unwrap();

    // Approve_all, revoke_all, and check for empty, to test revoke_all
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: String::from("operator"),
        expires: None,
    };
    // person is now the owner of the tokens
    let owner_info = addrs.info("person");
    contract
        .execute(deps.as_mut(), &mock_env(), &owner_info, approve_all_msg)
        .unwrap();

    // query for operator should return approval
    let res = contract
        .query_operator(
            deps.as_ref(),
            &mock_env(),
            addrs.addr("person").to_string(),
            addrs.addr("operator").to_string(),
            true,
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorResponse {
            approval: Approval {
                spender: addrs.addr("operator"),
                expires: Expiration::Never {}
            }
        }
    );

    // query for other should throw error
    let res = contract.query_operator(
        deps.as_ref(),
        &mock_env(),
        String::from("person"),
        String::from("other"),
        true,
    );
    match res {
        Err(StdError::NotFound { kind, .. }) => assert_eq!(kind, "Approval not found"),
        _ => panic!("Unexpected error"),
    }

    let res = contract
        .query_operators(
            deps.as_ref(),
            &mock_env(),
            String::from("person"),
            true,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![Approval {
                spender: Addr::unchecked("operator"),
                expires: Expiration::Never {}
            }]
        }
    );

    // second approval
    let buddy_expires = Expiration::AtHeight(1234567);
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: String::from("buddy"),
        expires: Some(buddy_expires),
    };
    let owner_info = addrs.info("person");
    contract
        .execute(deps.as_mut(), &mock_env(), &owner_info, approve_all_msg)
        .unwrap();

    // and paginate queries
    let res = contract
        .query_operators(
            deps.as_ref(),
            &mock_env(),
            String::from("person"),
            true,
            None,
            Some(1),
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![Approval {
                spender: Addr::unchecked("buddy"),
                expires: buddy_expires,
            }]
        }
    );
    let res = contract
        .query_operators(
            deps.as_ref(),
            &mock_env(),
            String::from("person"),
            true,
            Some(String::from("buddy")),
            Some(2),
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![Approval {
                spender: Addr::unchecked("operator"),
                expires: Expiration::Never {}
            }]
        }
    );

    let revoke_all_msg = Cw721ExecuteMsg::RevokeAll {
        operator: String::from("operator"),
    };
    contract
        .execute(deps.as_mut(), &mock_env(), &owner_info, revoke_all_msg)
        .unwrap();

    // query for operator should return error
    let res = contract.query_operator(
        deps.as_ref(),
        &mock_env(),
        String::from("person"),
        String::from("operator"),
        true,
    );
    match res {
        Err(StdError::NotFound { kind, .. }) => assert_eq!(kind, "Approval not found"),
        _ => panic!("Unexpected error"),
    }

    // Approvals are removed / cleared without affecting others
    let res = contract
        .query_operators(
            deps.as_ref(),
            &mock_env(),
            String::from("person"),
            false,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![Approval {
                spender: Addr::unchecked("buddy"),
                expires: buddy_expires,
            }]
        }
    );

    // ensure the filter works (nothing should be here
    let mut late_env = mock_env();
    late_env.block.height = 1234568; //expired
    let res = contract
        .query_operators(
            deps.as_ref(),
            &late_env,
            String::from("person"),
            false,
            None,
            None,
        )
        .unwrap();
    assert_eq!(0, res.operators.len());

    // query operator should also return error
    let res = contract.query_operator(
        deps.as_ref(),
        &late_env,
        String::from("person"),
        String::from("buddy"),
        false,
    );

    match res {
        Err(StdError::NotFound { kind, .. }) => assert_eq!(kind, "Approval not found"),
        _ => panic!("Unexpected error"),
    }
}

#[test]
fn test_set_withdraw_address() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    // other than creator cant set
    let err = contract
        .set_withdraw_address(deps.as_mut(), &minter, "foo".to_string())
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // creator can set
    contract
        .set_withdraw_address(deps.as_mut(), &creator, "foo".to_string())
        .unwrap();

    let withdraw_address = contract
        .config
        .withdraw_address
        .load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(withdraw_address, "foo".to_string())
}

#[test]
fn test_remove_withdraw_address() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    // other than creator cant remove
    let err = contract
        .remove_withdraw_address(deps.as_mut().storage, &minter)
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // no withdraw address set yet
    let err = contract
        .remove_withdraw_address(deps.as_mut().storage, &creator)
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::NoWithdrawAddress {});

    // set and remove
    contract
        .set_withdraw_address(deps.as_mut(), &creator, "foo".to_string())
        .unwrap();
    contract
        .remove_withdraw_address(deps.as_mut().storage, &creator)
        .unwrap();
    assert!(!contract
        .config
        .withdraw_address
        .exists(deps.as_ref().storage));

    // test that we can set again
    contract
        .set_withdraw_address(deps.as_mut(), &creator, "foo".to_string())
        .unwrap();
    let withdraw_address = contract
        .config
        .withdraw_address
        .load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(withdraw_address, "foo".to_string())
}

#[test]
fn test_withdraw_funds() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);

    // no withdraw address set
    let err = contract
        .withdraw_funds(deps.as_mut().storage, &Coin::new(100u32, "uark"))
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::NoWithdrawAddress {});

    // set and withdraw by non-creator
    contract
        .set_withdraw_address(deps.as_mut(), &creator, "foo".to_string())
        .unwrap();
    contract
        .withdraw_funds(deps.as_mut().storage, &Coin::new(100u32, "uark"))
        .unwrap();
}

#[test]
fn query_tokens_by_owner() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), &creator, &minter);
    let minter_info = addrs.info("minter");

    // Mint a couple tokens (from the same owner)
    let token_id1 = "grow1".to_string();
    let demeter = String::from("demeter");
    let token_id2 = "grow2".to_string();
    let ceres = String::from("ceres");
    let token_id3 = "sing".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id1.clone(),
        owner: demeter.clone(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), &mock_env(), &minter_info, mint_msg)
        .unwrap();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id2.clone(),
        owner: ceres.clone(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), &mock_env(), &minter_info, mint_msg)
        .unwrap();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id3.clone(),
        owner: demeter.clone(),
        token_uri: None,
        extension: None,
    };
    let env = mock_env();
    contract
        .execute(deps.as_mut(), &env, &minter_info, mint_msg)
        .unwrap();

    // get all tokens in order:
    let expected = vec![token_id1.clone(), token_id2.clone(), token_id3.clone()];
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, None)
        .unwrap();
    assert_eq!(&expected, &tokens.tokens);
    // paginate
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, Some(2))
        .unwrap();
    assert_eq!(&expected[..2], &tokens.tokens[..]);
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, Some(expected[1].clone()), None)
        .unwrap();
    assert_eq!(&expected[2..], &tokens.tokens[..]);

    // get by owner
    let by_ceres = vec![token_id2];
    let by_demeter = vec![token_id1, token_id3];
    // all tokens by owner
    let tokens = contract
        .query_tokens(deps.as_ref(), &env, demeter.clone(), None, None)
        .unwrap();
    assert_eq!(&by_demeter, &tokens.tokens);
    let tokens = contract
        .query_tokens(deps.as_ref(), &env, ceres, None, None)
        .unwrap();
    assert_eq!(&by_ceres, &tokens.tokens);

    // paginate for demeter
    let tokens = contract
        .query_tokens(deps.as_ref(), &env, demeter.clone(), None, Some(1))
        .unwrap();
    assert_eq!(&by_demeter[..1], &tokens.tokens[..]);
    let tokens = contract
        .query_tokens(
            deps.as_ref(),
            &env,
            demeter,
            Some(by_demeter[0].clone()),
            Some(3),
        )
        .unwrap();
    assert_eq!(&by_demeter[1..], &tokens.tokens[..]);
}
