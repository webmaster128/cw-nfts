use crate::{
    error::Cw721ContractError,
    extension::Cw721OnchainExtensions,
    msg::{
        CollectionExtensionMsg, ConfigResponse, Cw721ExecuteMsg, Cw721InstantiateMsg,
        Cw721MigrateMsg, Cw721QueryMsg, NumTokensResponse, OwnerOfResponse, RoyaltyInfoResponse,
    },
    state::{CollectionInfo, NftExtension, Trait},
    traits::{Cw721Execute, Cw721Query},
    DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg, NftExtensionMsg,
};
use anyhow::Result;
use cosmwasm_std::testing::{mock_dependencies, MockApi};
use cosmwasm_std::{
    Addr, Binary, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, QuerierWrapper, Response,
    StdError, Timestamp,
};
use cw721_016::NftInfoResponse;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_ownable::{Ownership, OwnershipError};
use cw_utils::Expiration;
use url::ParseError;
pub const ADMIN_ADDR: &str = "admin";
pub const CREATOR_ADDR: &str = "creator";
pub const MINTER_ADDR: &str = "minter";
pub const OTHER_ADDR: &str = "other";
pub const WITHDRAW_ADDR: &str = "other";
pub const NFT_OWNER_ADDR: &str = "nft_owner";

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

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721InstantiateMsg<DefaultOptionalCollectionExtensionMsg>,
) -> Result<Response, Cw721ContractError> {
    let contract = Cw721OnchainExtensions::default();
    contract.instantiate_with_version(deps, &env, &info, msg, "contract_name", "contract_version")
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ExecuteMsg<
        DefaultOptionalNftExtensionMsg,
        DefaultOptionalCollectionExtensionMsg,
        Empty,
    >,
) -> Result<Response, Cw721ContractError> {
    let contract = Cw721OnchainExtensions::default();
    contract.execute(deps, &env, &info, msg)
}

pub fn query(
    deps: Deps,
    env: Env,
    msg: Cw721QueryMsg<DefaultOptionalNftExtension, DefaultOptionalCollectionExtension, Empty>,
) -> Result<Binary, Cw721ContractError> {
    let contract = Cw721OnchainExtensions::default();
    contract.query(deps, &env, msg)
}

pub fn migrate(
    deps: DepsMut,
    env: Env,
    msg: Cw721MigrateMsg,
) -> Result<Response, Cw721ContractError> {
    let contract = Cw721OnchainExtensions::default();
    contract.migrate(deps, env, msg, "contract_name", "contract_version")
}

fn cw721_base_latest_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
    Box::new(contract)
}

fn query_nft_info(
    querier: QuerierWrapper,
    cw721: &Addr,
    token_id: String,
) -> NftInfoResponse<Option<NftExtension>> {
    querier
        .query_wasm_smart(
            cw721,
            &Cw721QueryMsg::<
                DefaultOptionalNftExtension,
                DefaultOptionalCollectionExtension,
                Empty,
            >::NftInfo {
                token_id,
            },
        )
        .unwrap()
}

fn query_all_collection_info(
    querier: QuerierWrapper,
    cw721: &Addr,
) -> ConfigResponse<DefaultOptionalCollectionExtension> {
    querier
        .query_wasm_smart(
            cw721,
            &Cw721QueryMsg::<
                DefaultOptionalNftExtension,
                DefaultOptionalCollectionExtension,
                Empty,
            >::GetConfig {},
        )
        .unwrap()
}

#[test]
fn test_operator() {
    // --- setup ---
    let mut app = App::default();
    let deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let admin = addrs.addr(ADMIN_ADDR);
    let creator = addrs.addr(CREATOR_ADDR);
    let minter = addrs.addr(MINTER_ADDR);
    let code_id = app.store_code(cw721_base_latest_contract());
    let other = addrs.addr(OTHER_ADDR);
    let cw721 = app
        .instantiate_contract(
            code_id,
            other.clone(),
            &Cw721InstantiateMsg::<DefaultOptionalCollectionExtension> {
                name: "collection".to_string(),
                symbol: "symbol".to_string(),
                minter: Some(minter.to_string()),
                creator: Some(creator.to_string()),
                collection_info_extension: None,
                withdraw_address: None,
            },
            &[],
            "cw721-base",
            Some(admin.to_string()),
        )
        .unwrap();
    // mint
    let nft_owner = addrs.addr(NFT_OWNER_ADDR);
    app.execute_contract(
        minter,
        cw721.clone(),
        &Cw721ExecuteMsg::<Empty, Empty, Empty>::Mint {
            token_id: "1".to_string(),
            owner: nft_owner.to_string(),
            token_uri: Some("".to_string()), // empty uri, response contains attribute with value "empty"
            extension: Empty::default(),
        },
        &[],
    )
    .unwrap();

    // --- test operator/approve all ---
    // owner adds other user as operator using approve all
    app.execute_contract(
        nft_owner.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<Empty, Empty, Empty>::ApproveAll {
            operator: other.to_string(),
            expires: Some(Expiration::Never {}),
        },
        &[],
    )
    .unwrap();

    // transfer by operator
    app.execute_contract(
        other.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<Empty, Empty, Empty>::TransferNft {
            recipient: other.to_string(),
            token_id: "1".to_string(),
        },
        &[],
    )
    .unwrap();
    // check other is new owner
    let owner_response: OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(
            &cw721,
            &Cw721QueryMsg::<
                DefaultOptionalNftExtension,
                DefaultOptionalCollectionExtension,
                Empty,
            >::OwnerOf {
                token_id: "1".to_string(),
                include_expired: None,
            },
        )
        .unwrap();
    assert_eq!(owner_response.owner, other.to_string());
    // check previous owner cant transfer
    let err: Cw721ContractError = app
        .execute_contract(
            nft_owner.clone(),
            cw721.clone(),
            &Cw721ExecuteMsg::<Empty, Empty, Empty>::TransferNft {
                recipient: other.to_string(),
                token_id: "1".to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // transfer back to previous owner
    app.execute_contract(
        other.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<Empty, Empty, Empty>::TransferNft {
            recipient: nft_owner.to_string(),
            token_id: "1".to_string(),
        },
        &[],
    )
    .unwrap();
    // check owner
    let owner_response: OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(
            &cw721,
            &Cw721QueryMsg::<
                DefaultOptionalNftExtension,
                DefaultOptionalCollectionExtension,
                Empty,
            >::OwnerOf {
                token_id: "1".to_string(),
                include_expired: None,
            },
        )
        .unwrap();
    assert_eq!(owner_response.owner, nft_owner.to_string());

    // other user is still operator and can transfer!
    app.execute_contract(
        other.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<Empty, Empty, Empty>::TransferNft {
            recipient: other.to_string(),
            token_id: "1".to_string(),
        },
        &[],
    )
    .unwrap();
    // check other is new owner
    let owner_response: OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(
            &cw721,
            &Cw721QueryMsg::<
                DefaultOptionalNftExtension,
                DefaultOptionalCollectionExtension,
                Empty,
            >::OwnerOf {
                token_id: "1".to_string(),
                include_expired: None,
            },
        )
        .unwrap();
    assert_eq!(owner_response.owner, other.to_string());

    // -- test revoke
    // transfer to previous owner
    app.execute_contract(
        other.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<Empty, Empty, Empty>::TransferNft {
            recipient: nft_owner.to_string(),
            token_id: "1".to_string(),
        },
        &[],
    )
    .unwrap();

    // revoke operator
    app.execute_contract(
        nft_owner,
        cw721.clone(),
        &Cw721ExecuteMsg::<Empty, Empty, Empty>::RevokeAll {
            operator: other.to_string(),
        },
        &[],
    )
    .unwrap();

    // other not operator anymore and cant send
    let err: Cw721ContractError = app
        .execute_contract(
            other.clone(),
            cw721,
            &Cw721ExecuteMsg::<Empty, Empty, Empty>::TransferNft {
                recipient: other.to_string(),
                token_id: "1".to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));
}

#[test]
fn test_instantiate() {
    let mut app = App::default();
    let deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);

    let admin = addrs.addr(ADMIN_ADDR);
    let minter = addrs.addr(MINTER_ADDR);
    let creator = addrs.addr(CREATOR_ADDR);
    let payment_address = addrs.addr(OTHER_ADDR);
    let withdraw_addr = addrs.addr(WITHDRAW_ADDR);

    let init_msg = Cw721InstantiateMsg {
        name: "collection".to_string(),
        symbol: "symbol".to_string(),
        minter: Some(minter.to_string()),
        creator: Some(creator.to_string()),
        withdraw_address: Some(withdraw_addr.to_string()),
        collection_info_extension: Some(CollectionExtensionMsg {
            description: Some("description".to_string()),
            image: Some("ipfs://ark.pass".to_string()),
            explicit_content: Some(false),
            external_link: Some("https://interchain.arkprotocol.io".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(42)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: payment_address.to_string(),
                share: Decimal::bps(1000),
            }),
        }),
    };
    // test case: happy path
    {
        let code_id_latest = app.store_code(cw721_base_latest_contract());
        let cw721 = app
            .instantiate_contract(
                code_id_latest,
                admin.clone(),
                &init_msg.clone(),
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap();

        // assert withdraw address
        let withdraw_addr_result: Option<String> = app
            .wrap()
            .query_wasm_smart(
                cw721,
                &Cw721QueryMsg::<
                    DefaultOptionalNftExtension,
                    DefaultOptionalCollectionExtension,
                    Empty,
                >::GetWithdrawAddress {},
            )
            .unwrap();
        assert_eq!(withdraw_addr_result, Some(withdraw_addr.to_string()));
    }
    // test case: invalid addresses
    {
        // invalid creator
        let code_id_latest = app.store_code(cw721_base_latest_contract());
        let mut invalid_init_msg = init_msg.clone();
        invalid_init_msg.creator = Some("invalid".to_string());
        let error: Cw721ContractError = app
            .instantiate_contract(
                code_id_latest,
                admin.clone(),
                &invalid_init_msg.clone(),
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            error,
            Cw721ContractError::Std(StdError::generic_err("Error decoding bech32"))
        );
        // invalid minter
        let mut invalid_init_msg = init_msg.clone();
        invalid_init_msg.minter = Some("invalid".to_string());
        let error: Cw721ContractError = app
            .instantiate_contract(
                code_id_latest,
                admin.clone(),
                &invalid_init_msg.clone(),
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            error,
            Cw721ContractError::Std(StdError::generic_err("Error decoding bech32"))
        );
        // invalid withdraw addr
        let mut invalid_init_msg = init_msg.clone();
        invalid_init_msg.withdraw_address = Some("invalid".to_string());
        let error: Cw721ContractError = app
            .instantiate_contract(
                code_id_latest,
                admin.clone(),
                &invalid_init_msg.clone(),
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            error,
            Cw721ContractError::Std(StdError::generic_err("Error decoding bech32"))
        );
        // invalid payment addr
        let mut invalid_init_msg = init_msg.clone();
        invalid_init_msg.collection_info_extension = Some(CollectionExtensionMsg {
            description: Some("description".to_string()),
            image: Some("ipfs://ark.pass".to_string()),
            explicit_content: Some(false),
            external_link: Some("https://interchain.arkprotocol.io".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(42)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "invalid".to_string(),
                share: Decimal::bps(1000),
            }),
        });
        let error: Cw721ContractError = app
            .instantiate_contract(
                code_id_latest,
                admin.clone(),
                &invalid_init_msg.clone(),
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            error,
            Cw721ContractError::Std(StdError::generic_err("Error decoding bech32"))
        );
    }
    // test case: backward compatibility using instantiate msg from a 0.16 version on latest contract.
    // This ensures existing 3rd party contracts doesnt need to update as well.
    {
        use cw721_base_016 as v16;
        let code_id_latest = app.store_code(cw721_base_latest_contract());

        let cw721 = app
            .instantiate_contract(
                code_id_latest,
                admin.clone(),
                &v16::InstantiateMsg {
                    name: "collection".to_string(),
                    symbol: "symbol".to_string(),
                    minter: admin.to_string(),
                },
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap();

        // assert withdraw address is None
        let withdraw_addr: Option<String> = app
            .wrap()
            .query_wasm_smart(
                cw721,
                &Cw721QueryMsg::<
                    DefaultOptionalNftExtension,
                    DefaultOptionalCollectionExtension,
                    Empty,
                >::GetWithdrawAddress {},
            )
            .unwrap();
        assert!(withdraw_addr.is_none());
    }
}

#[test]
fn test_update_nft_metadata() {
    // --- setup ---
    let mut app = App::default();
    let deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);

    let admin = addrs.addr(ADMIN_ADDR);
    let code_id = app.store_code(cw721_base_latest_contract());
    let creator = addrs.addr(CREATOR_ADDR);
    let minter_addr = addrs.addr(MINTER_ADDR);
    let cw721 = app
        .instantiate_contract(
            code_id,
            creator.clone(),
            &Cw721InstantiateMsg::<DefaultOptionalCollectionExtension> {
                name: "collection".to_string(),
                symbol: "symbol".to_string(),
                minter: Some(minter_addr.to_string()),
                creator: None, // in case of none, sender is creator
                collection_info_extension: None,
                withdraw_address: None,
            },
            &[],
            "cw721-base",
            Some(admin.to_string()),
        )
        .unwrap();
    // mint
    let nft_owner = addrs.addr(NFT_OWNER_ADDR);
    let nft_metadata_msg = NftExtensionMsg {
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
    app.execute_contract(
        minter_addr,
        cw721.clone(),
        &Cw721ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::Mint {
            token_id: "1".to_string(),
            owner: nft_owner.to_string(),
            token_uri: Some("ipfs://foo.bar/metadata.json".to_string()),
            extension: Some(nft_metadata_msg.clone()),
        },
        &[],
    )
    .unwrap();

    // check nft info
    let nft_info = query_nft_info(app.wrap(), &cw721, "1".to_string());
    assert_eq!(
        nft_info.token_uri,
        Some("ipfs://foo.bar/metadata.json".to_string())
    );
    assert_eq!(
        nft_info.extension,
        Some(NftExtension {
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
        })
    );

    // nft owner cant update - only creator is allowed
    let err: Cw721ContractError = app
        .execute_contract(
            nft_owner,
            cw721.clone(),
            &Cw721ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::UpdateNftInfo {
                token_id: "1".to_string(),
                token_uri: None,
                extension: Some(NftExtensionMsg {
                    name: Some("new name".to_string()),
                    description: Some("new description".to_string()),
                    image: None,
                    image_data: None,
                    external_url: None,
                    attributes: None,
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                }),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, Cw721ContractError::NotCreator {});

    // update invalid token uri
    let err: Cw721ContractError = app
        .execute_contract(
            creator.clone(),
            cw721.clone(),
            &Cw721ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::UpdateNftInfo {
                token_id: "1".to_string(),
                token_uri: Some("invalid".to_string()),
                extension: Some(NftExtensionMsg {
                    name: None,
                    description: None,
                    image: None,
                    image_data: None,
                    external_url: None,
                    attributes: None,
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                }),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        Cw721ContractError::ParseError(ParseError::RelativeUrlWithoutBase)
    );

    // invalid image URL
    let err: Cw721ContractError = app
        .execute_contract(
            creator.clone(),
            cw721.clone(),
            &Cw721ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::UpdateNftInfo {
                token_id: "1".to_string(),
                token_uri: None,
                extension: Some(NftExtensionMsg {
                    name: None,
                    description: None,
                    image: Some("invalid".to_string()),
                    image_data: None,
                    external_url: None,
                    attributes: None,
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                }),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        Cw721ContractError::ParseError(ParseError::RelativeUrlWithoutBase)
    );

    // invalid external url
    let err: Cw721ContractError = app
        .execute_contract(
            creator.clone(),
            cw721.clone(),
            &Cw721ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::UpdateNftInfo {
                token_id: "1".to_string(),
                token_uri: None,
                extension: Some(NftExtensionMsg {
                    name: None,
                    description: None,
                    image: None,
                    image_data: None,
                    external_url: Some("invalid".to_string()),
                    attributes: None,
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                }),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        Cw721ContractError::ParseError(ParseError::RelativeUrlWithoutBase)
    );

    // invalid animation url
    let err: Cw721ContractError = app
        .execute_contract(
            creator.clone(),
            cw721.clone(),
            &Cw721ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::UpdateNftInfo {
                token_id: "1".to_string(),
                token_uri: None,
                extension: Some(NftExtensionMsg {
                    name: None,
                    description: None,
                    image: None,
                    image_data: None,
                    external_url: None,
                    attributes: None,
                    background_color: None,
                    animation_url: Some("invalid".to_string()),
                    youtube_url: None,
                }),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        Cw721ContractError::ParseError(ParseError::RelativeUrlWithoutBase)
    );

    // invalid youtube url
    let err: Cw721ContractError = app
        .execute_contract(
            creator.clone(),
            cw721.clone(),
            &Cw721ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::UpdateNftInfo {
                token_id: "1".to_string(),
                token_uri: None,
                extension: Some(NftExtensionMsg {
                    name: None,
                    description: None,
                    image: None,
                    image_data: None,
                    external_url: None,
                    attributes: None,
                    background_color: None,
                    animation_url: None,
                    youtube_url: Some("invalid".to_string()),
                }),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        Cw721ContractError::ParseError(ParseError::RelativeUrlWithoutBase)
    );

    // no image data (empty)
    app.execute_contract(
        creator.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::UpdateNftInfo {
            token_id: "1".to_string(),
            token_uri: None,
            extension: Some(NftExtensionMsg {
                name: None,
                description: None,
                image: None,
                image_data: Some("".to_string()),
                external_url: None,
                attributes: None,
                background_color: None,
                animation_url: None,
                youtube_url: None,
            }),
        },
        &[],
    )
    .unwrap();
    // check nft info
    let nft_info = query_nft_info(app.wrap(), &cw721, "1".to_string());
    assert_eq!(
        nft_info.token_uri,
        Some("ipfs://foo.bar/metadata.json".to_string())
    );
    assert_eq!(
        nft_info.extension,
        Some(NftExtension {
            image: Some("ipfs://foo.bar/image.png".to_string()),
            image_data: None,
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
        })
    );

    // no description (empty)
    app.execute_contract(
        creator.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::UpdateNftInfo {
            token_id: "1".to_string(),
            token_uri: None,
            extension: Some(NftExtensionMsg {
                name: None,
                description: Some("".to_string()),
                image: None,
                image_data: None,
                external_url: None,
                attributes: None,
                background_color: None,
                animation_url: None,
                youtube_url: None,
            }),
        },
        &[],
    )
    .unwrap();
    // check nft info
    let nft_info = query_nft_info(app.wrap(), &cw721, "1".to_string());
    assert_eq!(
        nft_info.token_uri,
        Some("ipfs://foo.bar/metadata.json".to_string())
    );
    assert_eq!(
        nft_info.extension,
        Some(NftExtension {
            image: Some("ipfs://foo.bar/image.png".to_string()),
            image_data: None,
            external_url: Some("https://github.com".to_string()),
            description: None,
            name: Some("name".to_string()),
            attributes: Some(vec![Trait {
                trait_type: "trait_type".to_string(),
                value: "value".to_string(),
                display_type: Some("display_type".to_string()),
            }]),
            background_color: Some("background_color".to_string()),
            animation_url: Some("ssl://animation_url".to_string()),
            youtube_url: Some("file://youtube_url".to_string()),
        })
    );

    // no metadata name (empty)
    app.execute_contract(
        creator.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::UpdateNftInfo {
            token_id: "1".to_string(),
            token_uri: None,
            extension: Some(NftExtensionMsg {
                name: Some("".to_string()),
                description: None,
                image: None,
                image_data: None,
                external_url: None,
                attributes: None,
                background_color: None,
                animation_url: None,
                youtube_url: None,
            }),
        },
        &[],
    )
    .unwrap();
    // check nft info
    let nft_info = query_nft_info(app.wrap(), &cw721, "1".to_string());
    assert_eq!(
        nft_info.token_uri,
        Some("ipfs://foo.bar/metadata.json".to_string())
    );
    assert_eq!(
        nft_info.extension,
        Some(NftExtension {
            image: Some("ipfs://foo.bar/image.png".to_string()),
            image_data: None,
            external_url: Some("https://github.com".to_string()),
            description: None,
            name: None,
            attributes: Some(vec![Trait {
                trait_type: "trait_type".to_string(),
                value: "value".to_string(),
                display_type: Some("display_type".to_string()),
            }]),
            background_color: Some("background_color".to_string()),
            animation_url: Some("ssl://animation_url".to_string()),
            youtube_url: Some("file://youtube_url".to_string()),
        })
    );

    // no background color (empty)
    app.execute_contract(
        creator.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::UpdateNftInfo {
            token_id: "1".to_string(),
            token_uri: None,
            extension: Some(NftExtensionMsg {
                name: None,
                description: None,
                image: None,
                image_data: None,
                external_url: None,
                attributes: None,
                background_color: Some("".to_string()),
                animation_url: None,
                youtube_url: None,
            }),
        },
        &[],
    )
    .unwrap();
    // check nft info
    let nft_info = query_nft_info(app.wrap(), &cw721, "1".to_string());
    assert_eq!(
        nft_info.token_uri,
        Some("ipfs://foo.bar/metadata.json".to_string())
    );
    assert_eq!(
        nft_info.extension,
        Some(NftExtension {
            image: Some("ipfs://foo.bar/image.png".to_string()),
            image_data: None,
            external_url: Some("https://github.com".to_string()),
            description: None,
            name: None,
            attributes: Some(vec![Trait {
                trait_type: "trait_type".to_string(),
                value: "value".to_string(),
                display_type: Some("display_type".to_string()),
            }]),
            background_color: None,
            animation_url: Some("ssl://animation_url".to_string()),
            youtube_url: Some("file://youtube_url".to_string()),
        })
    );

    // invalid trait type (empty)
    let err: Cw721ContractError = app
        .execute_contract(
            creator.clone(),
            cw721.clone(),
            &Cw721ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::UpdateNftInfo {
                token_id: "1".to_string(),
                token_uri: None,
                extension: Some(NftExtensionMsg {
                    name: None,
                    description: None,
                    image: None,
                    image_data: None,
                    external_url: None,
                    attributes: Some(vec![Trait {
                        trait_type: "".to_string(),
                        value: "value".to_string(),
                        display_type: Some("display_type".to_string()),
                    }]),
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                }),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, Cw721ContractError::TraitTypeEmpty {});

    // invalid trait value (empty)
    let err: Cw721ContractError = app
        .execute_contract(
            creator.clone(),
            cw721.clone(),
            &Cw721ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::UpdateNftInfo {
                token_id: "1".to_string(),
                token_uri: None,
                extension: Some(NftExtensionMsg {
                    name: None,
                    description: None,
                    image: None,
                    image_data: None,
                    external_url: None,
                    attributes: Some(vec![Trait {
                        trait_type: "trait_type".to_string(),
                        value: "".to_string(),
                        display_type: Some("display_type".to_string()),
                    }]),
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                }),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, Cw721ContractError::TraitValueEmpty {});

    // invalid trait display type (empty)
    let err: Cw721ContractError = app
        .execute_contract(
            creator.clone(),
            cw721.clone(),
            &Cw721ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::UpdateNftInfo {
                token_id: "1".to_string(),
                token_uri: None,
                extension: Some(NftExtensionMsg {
                    name: None,
                    description: None,
                    image: None,
                    image_data: None,
                    external_url: None,
                    attributes: Some(vec![Trait {
                        trait_type: "trait_type".to_string(),
                        value: "value".to_string(),
                        display_type: Some("".to_string()),
                    }]),
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                }),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, Cw721ContractError::TraitDisplayTypeEmpty {});

    // proper update
    let new_nft_metadata_msg = NftExtensionMsg {
        image: None, // set to none to ensure it is unchanged
        image_data: Some("image data2".to_string()),
        external_url: Some("https://github.com2".to_string()),
        description: Some("description2".to_string()),
        name: Some("name2".to_string()),
        attributes: Some(vec![Trait {
            trait_type: "trait_type2".to_string(),
            value: "value2".to_string(),
            display_type: Some("display_type2".to_string()),
        }]),
        background_color: Some("background_color2".to_string()),
        animation_url: Some("ssl://animation_url2".to_string()),
        youtube_url: Some("file://youtube_url2".to_string()),
    };
    app.execute_contract(
        creator,
        cw721.clone(),
        &Cw721ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::UpdateNftInfo {
            token_id: "1".to_string(),
            token_uri: Some("ipfs://foo.bar/metadata2.json".to_string()),
            extension: Some(new_nft_metadata_msg.clone()),
        },
        &[],
    )
    .unwrap();
    // check token uri and extension
    let nft_info = query_nft_info(app.wrap(), &cw721, "1".to_string());
    assert_eq!(
        nft_info.token_uri,
        Some("ipfs://foo.bar/metadata2.json".to_string())
    );
    assert_eq!(
        nft_info.extension,
        Some(NftExtension {
            image: Some("ipfs://foo.bar/image.png".to_string()), // this is unchanged
            image_data: Some("image data2".to_string()),
            external_url: Some("https://github.com2".to_string()),
            description: Some("description2".to_string()),
            name: Some("name2".to_string()),
            attributes: Some(vec![Trait {
                trait_type: "trait_type2".to_string(),
                value: "value2".to_string(),
                display_type: Some("display_type2".to_string()),
            }]),
            background_color: Some("background_color2".to_string()),
            animation_url: Some("ssl://animation_url2".to_string()),
            youtube_url: Some("file://youtube_url2".to_string()),
        })
    );
    // check num tokens
    let num_tokens: NumTokensResponse = app
        .wrap()
        .query_wasm_smart(
            &cw721,
            &Cw721QueryMsg::<
                DefaultOptionalNftExtension,
                DefaultOptionalCollectionExtension,
                Empty,
            >::NumTokens {},
        )
        .unwrap();
    assert_eq!(num_tokens.count, 1);
}

#[test]
fn test_queries() {
    // --- setup ---
    let mut app = App::default();
    let deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);

    let admin = addrs.addr(ADMIN_ADDR);
    let code_id = app.store_code(cw721_base_latest_contract());
    let creator = addrs.addr(CREATOR_ADDR);
    let minter_addr = addrs.addr(MINTER_ADDR);
    let withdraw_addr = addrs.addr(OTHER_ADDR);

    let cw721 = app
        .instantiate_contract(
            code_id,
            creator.clone(),
            &Cw721InstantiateMsg::<DefaultOptionalCollectionExtension> {
                name: "collection".to_string(),
                symbol: "symbol".to_string(),
                minter: Some(minter_addr.to_string()),
                creator: None, // in case of none, sender is creator
                collection_info_extension: None,
                withdraw_address: Some(withdraw_addr.to_string()),
            },
            &[],
            "cw721-base",
            Some(admin.to_string()),
        )
        .unwrap();
    // mint
    let nft_owner = app.api().addr_make(NFT_OWNER_ADDR);
    let nft_metadata_msg = NftExtensionMsg {
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
    app.execute_contract(
        minter_addr.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::Mint {
            token_id: "1".to_string(),
            owner: nft_owner.to_string(),
            token_uri: Some("ipfs://foo.bar/metadata.json".to_string()),
            extension: Some(nft_metadata_msg.clone()),
        },
        &[],
    )
    .unwrap();

    // check nft info
    let nft_info = query_nft_info(app.wrap(), &cw721, "1".to_string());
    assert_eq!(
        nft_info.token_uri,
        Some("ipfs://foo.bar/metadata.json".to_string())
    );
    assert_eq!(
        nft_info.extension,
        Some(NftExtension {
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
        })
    );
    // check num tokens
    let num_tokens: NumTokensResponse = app
        .wrap()
        .query_wasm_smart(
            &cw721,
            &Cw721QueryMsg::<
                DefaultOptionalNftExtension,
                DefaultOptionalCollectionExtension,
                Empty,
            >::NumTokens {},
        )
        .unwrap();
    assert_eq!(num_tokens.count, 1);
    // check withdraw address
    let withdraw_addr_result: Option<String> = app
        .wrap()
        .query_wasm_smart(
            &cw721,
            &Cw721QueryMsg::<
                DefaultOptionalNftExtension,
                DefaultOptionalCollectionExtension,
                Empty,
            >::GetWithdrawAddress {},
        )
        .unwrap();
    assert_eq!(withdraw_addr_result, Some(withdraw_addr.to_string()));
    // check all collection info
    let all_collection_info = query_all_collection_info(app.wrap(), &cw721);
    let contract_info = app.wrap().query_wasm_contract_info(&cw721).unwrap();
    assert_eq!(
        all_collection_info,
        ConfigResponse {
            minter_ownership: Ownership {
                owner: Some(minter_addr),
                pending_expiry: None,
                pending_owner: None,
            },
            creator_ownership: Ownership {
                owner: Some(creator),
                pending_expiry: None,
                pending_owner: None,
            },
            collection_info: CollectionInfo {
                name: "collection".to_string(),
                symbol: "symbol".to_string(),
                updated_at: all_collection_info.collection_info.updated_at,
            },
            collection_extension: None,
            num_tokens: 1,
            withdraw_address: Some(withdraw_addr.into_string()),
            contract_info
        }
    );
}
