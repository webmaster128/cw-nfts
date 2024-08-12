use crate::{
    error::Cw721ContractError,
    extension::Cw721OnchainExtensions,
    msg::{
        Cw721ExecuteMsg, Cw721InstantiateMsg, Cw721MigrateMsg, Cw721QueryMsg, NumTokensResponse,
        OwnerOfResponse,
    },
    state::{NftExtension, Trait},
    traits::{Cw721Execute, Cw721Query},
    DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg, NftExtensionMsg,
};
use cosmwasm_std::{
    Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, QuerierWrapper, Response,
};
use cw721_016::NftInfoResponse;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_ownable::OwnershipError;
use cw_utils::Expiration;
use url::ParseError;

pub const CREATOR_ADDR: &str = "creator";
pub const MINTER_ADDR: &str = "minter";
pub const OTHER_ADDR: &str = "other";
pub const NFT_OWNER_ADDR: &str = "nft_owner";

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

#[test]
fn test_operator() {
    // --- setup ---
    let mut app = App::default();
    let admin = Addr::unchecked("admin");
    let code_id = app.store_code(cw721_base_latest_contract());
    let other = Addr::unchecked(OTHER_ADDR);
    let cw721 = app
        .instantiate_contract(
            code_id,
            other.clone(),
            &Cw721InstantiateMsg::<DefaultOptionalCollectionExtension> {
                name: "collection".to_string(),
                symbol: "symbol".to_string(),
                minter: Some(MINTER_ADDR.to_string()),
                creator: Some(CREATOR_ADDR.to_string()),
                collection_info_extension: None,
                withdraw_address: None,
            },
            &[],
            "cw721-base",
            Some(admin.to_string()),
        )
        .unwrap();
    // mint
    let minter = Addr::unchecked(MINTER_ADDR);
    let nft_owner = Addr::unchecked(NFT_OWNER_ADDR);
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

/// Test backward compatibility using instantiate msg from a 0.16 version on latest contract.
/// This ensures existing 3rd party contracts doesnt need to update as well.
#[test]
fn test_instantiate_016_msg() {
    use cw721_base_016 as v16;
    let mut app = App::default();
    let admin = || Addr::unchecked("admin");

    let code_id_latest = app.store_code(cw721_base_latest_contract());

    let cw721 = app
        .instantiate_contract(
            code_id_latest,
            admin(),
            &v16::InstantiateMsg {
                name: "collection".to_string(),
                symbol: "symbol".to_string(),
                minter: admin().into_string(),
            },
            &[],
            "cw721-base",
            Some(admin().into_string()),
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

#[test]
fn test_update_nft_metadata() {
    // --- setup ---
    let mut app = App::default();
    let admin = Addr::unchecked("admin");
    let code_id = app.store_code(cw721_base_latest_contract());
    let creator = Addr::unchecked(CREATOR_ADDR);
    let cw721 = app
        .instantiate_contract(
            code_id,
            creator.clone(),
            &Cw721InstantiateMsg::<DefaultOptionalCollectionExtension> {
                name: "collection".to_string(),
                symbol: "symbol".to_string(),
                minter: Some(MINTER_ADDR.to_string()),
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
    let minter = Addr::unchecked(MINTER_ADDR);
    let nft_owner = Addr::unchecked(NFT_OWNER_ADDR);
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
        minter,
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
