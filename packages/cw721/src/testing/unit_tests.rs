use crate::{
    error::Cw721ContractError,
    extension::Cw721OnchainExtensions,
    msg::{
        CollectionExtensionMsg, CollectionInfoMsg, Cw721ExecuteMsg, Cw721InstantiateMsg,
        NftExtensionMsg, RoyaltyInfoResponse,
    },
    state::{
        NftExtension, Trait, CREATOR, MAX_COLLECTION_DESCRIPTION_LENGTH,
        MAX_ROYALTY_SHARE_DELTA_PCT, MAX_ROYALTY_SHARE_PCT, MINTER,
    },
    traits::{Cw721Execute, Cw721Query},
    CollectionExtension, RoyaltyInfo,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Addr, Api, Decimal, Timestamp,
};
use cw2::ContractVersion;
use cw_ownable::Action;
use unit_tests::contract_tests::MockAddrFactory;
use unit_tests::multi_tests::{CREATOR_ADDR, MINTER_ADDR, OTHER_ADDR};

use super::*;

#[test]
fn test_instantiation() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    // error on empty name
    let err = Cw721OnchainExtensions::default()
        .instantiate_with_version(
            deps.as_mut(),
            &mock_env(),
            &addrs.info("mr-t"),
            Cw721InstantiateMsg {
                name: "".into(),
                symbol: "collection_symbol".into(),
                collection_info_extension: None,
                creator: None,
                minter: None,
                withdraw_address: None,
            },
            "contract_name",
            "contract_version",
        )
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::CollectionNameEmpty {});

    // error on empty symbol
    let err = Cw721OnchainExtensions::default()
        .instantiate_with_version(
            deps.as_mut(),
            &mock_env(),
            &addrs.info("mr-t"),
            Cw721InstantiateMsg {
                name: "collection_name".into(),
                symbol: "".into(),
                collection_info_extension: None,
                creator: None,
                minter: None,
                withdraw_address: None,
            },
            "contract_name",
            "contract_version",
        )
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::CollectionSymbolEmpty {});

    Cw721OnchainExtensions::default()
        .instantiate_with_version(
            deps.as_mut(),
            &mock_env(),
            &addrs.info("larry"),
            Cw721InstantiateMsg {
                name: "collection_name".into(),
                symbol: "collection_symbol".into(),
                collection_info_extension: None,
                minter: Some("minter".into()),
                creator: Some("creator".into()),
                withdraw_address: None,
            },
            "contract_name",
            "contract_version",
        )
        .unwrap();

    // assert minter and creator
    let minter = MINTER
        .get_ownership(deps.as_ref().storage)
        .unwrap()
        .owner
        .map(|a| a.into_string());
    assert_eq!(minter, Some("minter".to_string()));

    let creator = CREATOR
        .get_ownership(deps.as_ref().storage)
        .unwrap()
        .owner
        .map(|a| a.into_string());
    assert_eq!(creator, Some("creator".to_string()));

    //
    let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
    assert_eq!(
        version,
        ContractVersion {
            contract: "contract_name".into(),
            version: "contract_version".into(),
        },
    );
}

#[test]
fn test_instantiation_with_proper_minter_and_creator() {
    // case 1: sender is used in case minter and creator is not set
    {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let info_minter_and_creator = addrs.info("minter_and_creator");
        Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info_minter_and_creator,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: None,
                    creator: None,
                    minter: None,
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap();

        let minter = MINTER.item.load(deps.as_ref().storage).unwrap().owner;
        assert_eq!(minter, Some(info_minter_and_creator.sender.clone()));
        let creator = CREATOR.item.load(deps.as_ref().storage).unwrap().owner;
        assert_eq!(creator, Some(info_minter_and_creator.sender));
    }
    // case 2: minter and creator are set
    {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let info = addrs.info(OTHER_ADDR);
        Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: None,
                    creator: Some(CREATOR_ADDR.into()),
                    minter: Some(MINTER_ADDR.into()),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap();

        let minter = MINTER.item.load(deps.as_ref().storage).unwrap().owner;
        assert_eq!(minter, Some(Addr::unchecked(MINTER_ADDR.to_string())));
        let creator = CREATOR.item.load(deps.as_ref().storage).unwrap().owner;
        assert_eq!(creator, Some(Addr::unchecked(CREATOR_ADDR.to_string())));
    }
    // case 3: sender is minter and creator is set
    {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let info = addrs.info(MINTER_ADDR);
        Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: None,
                    creator: Some(CREATOR_ADDR.into()),
                    minter: None,
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap();

        let minter = MINTER.item.load(deps.as_ref().storage).unwrap().owner;
        assert_eq!(minter, Some(info.sender));
        let creator = CREATOR.item.load(deps.as_ref().storage).unwrap().owner;
        assert_eq!(creator, Some(Addr::unchecked(CREATOR_ADDR.to_string())));
    }
    // case 4: sender is creator and minter is set
    {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let info = addrs.info(CREATOR_ADDR);
        Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: None,
                    creator: None,
                    minter: Some(MINTER_ADDR.into()),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap();

        let minter = MINTER.item.load(deps.as_ref().storage).unwrap().owner;
        assert_eq!(minter, Some(Addr::unchecked(MINTER_ADDR.to_string())));
        let creator = CREATOR.item.load(deps.as_ref().storage).unwrap().owner;
        assert_eq!(creator, Some(info.sender));
    }
}

#[test]
fn test_instantiation_with_collection_info() {
    // case 1: extension set with proper data
    {
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let info_creator = addrs.info(CREATOR_ADDR);
        let extension = Some(CollectionExtension {
            description: "description".into(),
            image: "https://moonphases.org".to_string(),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            // no minter owner assertion on start trading time, so even creator can change this here
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfo {
                payment_address: Addr::unchecked("payment_address"),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let extension_msg = Some(CollectionExtensionMsg {
            description: Some("description".into()),
            image: Some("https://moonphases.org".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info_creator,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: extension_msg,
                    creator: Some(CREATOR_ADDR.into()),
                    minter: Some(MINTER_ADDR.into()),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap();

        // validate data
        let collection_info = Cw721OnchainExtensions::default()
            .query_collection_info_and_extension(deps.as_ref())
            .unwrap();
        assert_eq!(collection_info.name, "collection_name");
        assert_eq!(collection_info.symbol, "collection_symbol");
        assert_eq!(collection_info.extension, extension);
    }
    // case 2: invalid data
    {
        // invalid image
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let info_creator = addrs.info(CREATOR_ADDR);
        let extension_msg = Some(CollectionExtensionMsg {
            description: Some("description".into()),
            image: Some("invalid_url".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let err = Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info_creator,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: extension_msg,
                    creator: Some(CREATOR_ADDR.into()),
                    minter: Some(MINTER_ADDR.into()),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::ParseError(url::ParseError::RelativeUrlWithoutBase)
        );

        // invalid external link
        let extension_msg = Some(CollectionExtensionMsg {
            description: Some("description".into()),
            image: Some("https://moonphases.org".to_string()),
            explicit_content: Some(true),
            external_link: Some("invalid_url".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let err = Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info_creator,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: extension_msg,
                    creator: Some(CREATOR_ADDR.into()),
                    minter: Some(MINTER_ADDR.into()),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::ParseError(url::ParseError::RelativeUrlWithoutBase)
        );

        // empty description
        let extension_msg = Some(CollectionExtensionMsg {
            description: Some("".into()),
            image: Some("https://moonphases.org".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let err = Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info_creator,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: extension_msg,
                    creator: Some(CREATOR_ADDR.into()),
                    minter: Some(MINTER_ADDR.into()),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::CollectionDescriptionEmpty {});

        // description too long
        let extension_msg = Some(CollectionExtensionMsg {
            description: Some("a".repeat(1001)),
            image: Some("https://moonphases.org".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let err = Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info_creator,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: extension_msg,
                    creator: Some(CREATOR_ADDR.into()),
                    minter: Some(MINTER_ADDR.into()),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::CollectionDescriptionTooLong {
                max_length: MAX_COLLECTION_DESCRIPTION_LENGTH
            }
        );

        // royalty share too high
        let extension_msg = Some(CollectionExtensionMsg {
            description: Some("description".into()),
            image: Some("https://moonphases.org".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: (MAX_ROYALTY_SHARE_PCT * 2).to_string().parse().unwrap(),
            }),
        });
        let err = Cw721OnchainExtensions::default()
            .instantiate_with_version(
                deps.as_mut(),
                &mock_env(),
                &info_creator,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: extension_msg,
                    creator: Some(CREATOR_ADDR.into()),
                    minter: Some(MINTER_ADDR.into()),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::InvalidRoyalties(format!(
                "Share cannot be greater than {MAX_ROYALTY_SHARE_PCT}%"
            ))
        );
    }
}

#[test]
fn test_collection_info_update() {
    // case 1: update with proper data
    {
        // initialize contract
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let env = mock_env();
        let info_creator = addrs.info(CREATOR_ADDR);
        let expected_instantiated_extension = Some(CollectionExtension {
            description: "description".into(),
            image: "https://moonphases.org".to_string(),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfo {
                payment_address: deps.api.addr_validate("payment_address").unwrap(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let instantiated_extension_msg = Some(CollectionExtensionMsg {
            description: Some("description".into()),
            image: Some("https://moonphases.org".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let contract = Cw721OnchainExtensions::default();
        contract
            .instantiate_with_version(
                deps.as_mut(),
                &env,
                &info_creator,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: instantiated_extension_msg,
                    creator: Some(CREATOR_ADDR.into()),
                    minter: Some(MINTER_ADDR.into()),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap();

        // update collection with no data
        let empty_extension_msg = CollectionExtensionMsg {
            description: None,
            image: None,
            explicit_content: None,
            external_link: None,
            start_trading_time: None,
            royalty_info: None,
        };
        let empty_collection_info_msg = CollectionInfoMsg {
            name: None,
            symbol: None,
            extension: Some(empty_extension_msg),
        };
        contract
            .execute(
                deps.as_mut(),
                &env,
                &info_creator,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: empty_collection_info_msg,
                },
            )
            .unwrap();
        // validate data
        let collection_info = contract
            .query_collection_info_and_extension(deps.as_ref())
            .unwrap();
        assert_eq!(collection_info.name, "collection_name");
        assert_eq!(collection_info.symbol, "collection_symbol");
        assert_eq!(collection_info.extension, expected_instantiated_extension);

        // update collection with proper data by creator
        let updated_extension_msg = CollectionExtensionMsg {
            description: Some("new_description".into()),
            image: Some("https://en.wikipedia.org/wiki/Non-fungible_token".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://github.com/CosmWasm/cw-nfts".to_string()),
            start_trading_time: None, // start trading time belongs to minter - not creator!
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: Some("new_collection_name".into()),
            symbol: Some("new_collection_symbol".into()),
            extension: Some(updated_extension_msg),
        };
        contract
            .execute(
                deps.as_mut(),
                &env,
                &info_creator,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap();

        // validate data
        let collection_info = Cw721OnchainExtensions::default()
            .query_collection_info_and_extension(deps.as_ref())
            .unwrap();
        assert_eq!(collection_info.name, "new_collection_name");
        assert_eq!(collection_info.symbol, "new_collection_symbol");
        assert_eq!(
            collection_info.extension,
            Some(CollectionExtension {
                description: "new_description".into(),
                image: "https://en.wikipedia.org/wiki/Non-fungible_token".to_string(),
                explicit_content: Some(true),
                external_link: Some("https://github.com/CosmWasm/cw-nfts".to_string()),
                start_trading_time: Some(Timestamp::from_seconds(0)),
                royalty_info: Some(RoyaltyInfo {
                    payment_address: Addr::unchecked("payment_address"),
                    share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                        .to_string()
                        .parse()
                        .unwrap(),
                }),
            })
        );

        // update start trading time by minter
        let updated_extension_msg = CollectionExtensionMsg {
            description: None,
            image: None,
            explicit_content: None,
            external_link: None,
            start_trading_time: Some(Timestamp::from_seconds(1)),
            royalty_info: None,
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: None,
            symbol: None,
            extension: Some(updated_extension_msg),
        };
        let info_minter = addrs.info(MINTER_ADDR);
        contract
            .execute(
                deps.as_mut(),
                &env,
                &info_minter,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap();
        // validate data
        let collection_info = Cw721OnchainExtensions::default()
            .query_collection_info_and_extension(deps.as_ref())
            .unwrap();
        assert_eq!(collection_info.name, "new_collection_name");
        assert_eq!(collection_info.symbol, "new_collection_symbol");
        assert_eq!(
            collection_info.extension,
            Some(CollectionExtension {
                description: "new_description".into(),
                image: "https://en.wikipedia.org/wiki/Non-fungible_token".to_string(),
                explicit_content: Some(true),
                external_link: Some("https://github.com/CosmWasm/cw-nfts".to_string()),
                start_trading_time: Some(Timestamp::from_seconds(1)),
                royalty_info: Some(RoyaltyInfo {
                    payment_address: Addr::unchecked("payment_address"),
                    share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                        .to_string()
                        .parse()
                        .unwrap(),
                }),
            })
        );
    }
    // case 2: udpate with invalid data
    {
        // initialize contract
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let env = mock_env();
        let info = addrs.info(CREATOR_ADDR);
        let instantiated_extension_msg = Some(CollectionExtensionMsg {
            description: Some("description".into()),
            image: Some("https://moonphases.org".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let contract = Cw721OnchainExtensions::default();
        contract
            .instantiate_with_version(
                deps.as_mut(),
                &env,
                &info,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: instantiated_extension_msg,
                    creator: None,
                    minter: None,
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap();

        // empty description
        let updated_extension_msg = CollectionExtensionMsg {
            description: Some("".into()),
            image: Some("https://en.wikipedia.org/wiki/Non-fungible_token".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://github.com/CosmWasm/cw-nfts".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: Some("new_collection_name".into()),
            symbol: Some("new_collection_symbol".into()),
            extension: Some(updated_extension_msg),
        };
        let err = contract
            .execute(
                deps.as_mut(),
                &env,
                &info,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::CollectionDescriptionEmpty {});

        // description too long
        let updated_extension_msg = CollectionExtensionMsg {
            description: Some("a".repeat(1001)),
            image: Some("https://en.wikipedia.org/wiki/Non-fungible_token".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://github.com/CosmWasm/cw-nfts".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: Some("new_collection_name".into()),
            symbol: Some("new_collection_symbol".into()),
            extension: Some(updated_extension_msg),
        };
        let err = contract
            .execute(
                deps.as_mut(),
                &env,
                &info,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::CollectionDescriptionTooLong {
                max_length: MAX_COLLECTION_DESCRIPTION_LENGTH
            }
        );

        // invalid image url
        let updated_extension_msg = CollectionExtensionMsg {
            description: Some("new_description".into()),
            image: Some("invalid_url".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://github.com/CosmWasm/cw-nfts".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: Some("new_collection_name".into()),
            symbol: Some("new_collection_symbol".into()),
            extension: Some(updated_extension_msg),
        };
        let err = contract
            .execute(
                deps.as_mut(),
                &env,
                &info,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::ParseError(url::ParseError::RelativeUrlWithoutBase)
        );

        // invalid external link url
        let updated_extension_msg = CollectionExtensionMsg {
            description: Some("new_description".into()),
            image: Some("https://en.wikipedia.org/wiki/Non-fungible_token".to_string()),
            explicit_content: Some(true),
            external_link: Some("invalid_url".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: Some("new_collection_name".into()),
            symbol: Some("new_collection_symbol".into()),
            extension: Some(updated_extension_msg),
        };
        let err = contract
            .execute(
                deps.as_mut(),
                &env,
                &info,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::ParseError(url::ParseError::RelativeUrlWithoutBase)
        );

        // royalty share too high
        let updated_extension_msg = CollectionExtensionMsg {
            description: Some("new_description".into()),
            image: Some("https://en.wikipedia.org/wiki/Non-fungible_token".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://github.com/CosmWasm/cw-nfts".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT + MAX_ROYALTY_SHARE_DELTA_PCT - 1)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: Some("new_collection_name".into()),
            symbol: Some("new_collection_symbol".into()),
            extension: Some(updated_extension_msg),
        };
        let err = contract
            .execute(
                deps.as_mut(),
                &env,
                &info,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::InvalidRoyalties(format!(
                "Share cannot be greater than {MAX_ROYALTY_SHARE_PCT}%"
            ))
        );

        // max share delta too high
        let updated_extension_msg = CollectionExtensionMsg {
            description: Some("new_description".into()),
            image: Some("https://en.wikipedia.org/wiki/Non-fungible_token".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://github.com/CosmWasm/cw-nfts".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT + MAX_ROYALTY_SHARE_DELTA_PCT + 1)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: Some("new_collection_name".into()),
            symbol: Some("new_collection_symbol".into()),
            extension: Some(updated_extension_msg),
        };
        let err = contract
            .execute(
                deps.as_mut(),
                &env,
                &info,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap_err();
        assert_eq!(
            err,
            Cw721ContractError::InvalidRoyalties(format!(
                "Share increase cannot be greater than {MAX_ROYALTY_SHARE_DELTA_PCT}%"
            ))
        );
    }
    // case 3: non-creator updating data
    {
        // initialize contract
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let env = mock_env();
        let info = addrs.info(CREATOR_ADDR);
        let instantiated_extension = Some(CollectionExtensionMsg {
            description: Some("description".into()),
            image: Some("https://moonphases.org".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let contract = Cw721OnchainExtensions::default();
        contract
            .instantiate_with_version(
                deps.as_mut(),
                &env,
                &info,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: instantiated_extension,
                    creator: None,
                    minter: None,
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap();

        // update collection by other user
        let updated_extension_msg = CollectionExtensionMsg {
            description: Some("new_description".into()),
            image: Some("https://en.wikipedia.org/wiki/Non-fungible_token".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://github.com/CosmWasm/cw-nfts".to_string()),
            start_trading_time: None,
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: Some("new_collection_name".into()),
            symbol: Some("new_collection_symbol".into()),
            extension: Some(updated_extension_msg),
        };
        let info_other = addrs.info(OTHER_ADDR);
        let err = contract
            .execute(
                deps.as_mut(),
                &env,
                &info_other,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg.clone(),
                },
            )
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::NotCreator {});
        // transfer creator to other user
        contract
            .execute(
                deps.as_mut(),
                &env,
                &info,
                Cw721ExecuteMsg::UpdateCreatorOwnership(Action::TransferOwnership {
                    new_owner: info_other.sender.to_string(),
                    expiry: None,
                }),
            )
            .unwrap();
        // other still cannot update collection, until ownership is accepted
        let err = contract
            .execute(
                deps.as_mut(),
                &env,
                &info_other,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg.clone(),
                },
            )
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::NotCreator {});
        // accept ownership
        contract
            .execute(
                deps.as_mut(),
                &env,
                &info_other,
                Cw721ExecuteMsg::UpdateCreatorOwnership(Action::AcceptOwnership {}),
            )
            .unwrap();
        // other can update collection now
        contract
            .execute(
                deps.as_mut(),
                &env,
                &info_other,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap();
    }
    // case 4: minter updating data
    {
        // initialize contract
        let mut deps = mock_dependencies();
        let mut addrs = MockAddrFactory::new(deps.api);
        let env = mock_env();
        let info_creator = addrs.info(CREATOR_ADDR);
        let info_minter = addrs.info(MINTER_ADDR);
        let instantiated_extension = Some(CollectionExtensionMsg {
            description: Some("description".into()),
            image: Some("https://moonphases.org".to_string()),
            explicit_content: Some(true),
            external_link: Some("https://moonphases.org".to_string()),
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".into(),
                share: Decimal::percent(MAX_ROYALTY_SHARE_PCT)
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
        });
        let contract = Cw721OnchainExtensions::default();
        contract
            .instantiate_with_version(
                deps.as_mut(),
                &env,
                &info_creator,
                Cw721InstantiateMsg {
                    name: "collection_name".into(),
                    symbol: "collection_symbol".into(),
                    collection_info_extension: instantiated_extension,
                    creator: None, // in case of none, sender is creator
                    minter: info_minter.sender.to_string().into(),
                    withdraw_address: None,
                },
                "contract_name",
                "contract_version",
            )
            .unwrap();

        // update start trading time by creator user not allowed
        let updated_extension_msg = CollectionExtensionMsg {
            description: None,
            image: None,
            explicit_content: None,
            external_link: None,
            start_trading_time: Some(Timestamp::from_seconds(1)),
            royalty_info: None,
        };
        let updated_collection_info_msg = CollectionInfoMsg {
            name: None,
            symbol: None,
            extension: Some(updated_extension_msg),
        };
        let err = contract
            .execute(
                deps.as_mut(),
                &env,
                &info_creator,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg.clone(),
                },
            )
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::NotMinter {});
        // update start trading time by minter
        contract
            .execute(
                deps.as_mut(),
                &env,
                &info_minter,
                Cw721ExecuteMsg::UpdateCollectionInfo {
                    collection_info: updated_collection_info_msg,
                },
            )
            .unwrap();
        // assert start trading has changed
        let collection_info = contract
            .query_collection_info_and_extension(deps.as_ref())
            .unwrap();
        assert_eq!(
            collection_info.extension.unwrap().start_trading_time,
            Some(Timestamp::from_seconds(1))
        );
    }
}

#[test]
fn test_nft_mint() {
    // case 1: mint without onchain metadata
    {
        let mut deps = mock_dependencies();
        let contract = Cw721OnchainExtensions::default();
        let mut addrs = MockAddrFactory::new(deps.api);
        let info = addrs.info(CREATOR_ADDR);
        let init_msg = Cw721InstantiateMsg {
            name: "collection_name".into(),
            symbol: "collection_symbol".into(),
            collection_info_extension: None,
            minter: None,
            creator: None,
            withdraw_address: None,
        };
        let env = mock_env();
        contract
            .instantiate_with_version(
                deps.as_mut(),
                &env,
                &info,
                init_msg,
                "contract_name",
                "contract_version",
            )
            .unwrap();

        let token_id = "Enterprise";
        let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let exec_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: token_uri.clone(),
            extension: None,
        };
        contract
            .execute(deps.as_mut(), &env, &info, exec_msg)
            .unwrap();

        let res = contract
            .query_nft_info(deps.as_ref().storage, token_id.into())
            .unwrap();
        assert_eq!(res.token_uri, token_uri);
        assert_eq!(res.extension, None);
        // mint with empty token_uri
        let exec_msg = Cw721ExecuteMsg::Mint {
            token_id: token_id.to_string(), // already minted/claimed
            owner: "john".to_string(),
            token_uri: "".to_string().into(), // empty token_uri
            extension: None,
        };
        let err = contract
            .execute(deps.as_mut(), &env, &info, exec_msg)
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::Claimed {});
        // non-minter cant mint
        let info = addrs.info("john");
        let exec_msg = Cw721ExecuteMsg::Mint {
            token_id: "Enterprise".to_string(),
            owner: "john".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: None,
        };
        let err = contract
            .execute(deps.as_mut(), &env, &info, exec_msg)
            .unwrap_err();
        assert_eq!(err, Cw721ContractError::NotMinter {});
    }
    // case 2: mint with onchain metadata
    {
        let mut deps = mock_dependencies();
        let contract = Cw721OnchainExtensions::default();
        let mut addrs = MockAddrFactory::new(deps.api);
        let info = addrs.info(CREATOR_ADDR);
        let init_msg = Cw721InstantiateMsg {
            name: "collection_name".into(),
            symbol: "collection_symbol".into(),
            collection_info_extension: None,
            minter: None,
            creator: None,
            withdraw_address: None,
        };
        let env = mock_env();
        contract
            .instantiate_with_version(
                deps.as_mut(),
                &env,
                &info,
                init_msg,
                "contract_name",
                "contract_version",
            )
            .unwrap();

        let nft_1 = "1";
        let uri_1 = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let extension_1_msg = Some(NftExtensionMsg {
            description: Some("description1".into()),
            name: Some("name1".to_string()),
            attributes: Some(vec![Trait {
                display_type: None,
                trait_type: "type1".to_string(),
                value: "value1".to_string(),
            }]),
            ..NftExtensionMsg::default()
        });
        let exec_msg = Cw721ExecuteMsg::Mint {
            token_id: nft_1.to_string(),
            owner: "john".to_string(),
            token_uri: uri_1.clone(),
            extension: extension_1_msg.clone(),
        };
        contract
            .execute(deps.as_mut(), &env, &info, exec_msg)
            .unwrap();

        let nft_info_1 = contract
            .query_nft_info(deps.as_ref().storage, nft_1.into())
            .unwrap();
        assert_eq!(nft_info_1.token_uri, uri_1);
        assert_eq!(
            nft_info_1.extension,
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

        // mint another
        let nft_2 = "2";
        let uri_2 = Some("ipfs://foo.bar".into());
        let extension_2_msg = Some(NftExtensionMsg {
            description: Some("other_description".into()),
            name: Some("name1".to_string()),
            attributes: Some(vec![Trait {
                display_type: None,
                trait_type: "type1".to_string(),
                value: "value1".to_string(),
            }]),
            ..NftExtensionMsg::default()
        });
        let exec_msg = Cw721ExecuteMsg::Mint {
            token_id: nft_2.to_string(),
            owner: "allen".to_string(),
            token_uri: uri_2.clone(),
            extension: extension_2_msg.clone(),
        };
        contract
            .execute(deps.as_mut(), &env, &info, exec_msg)
            .unwrap();

        let nft_info_2 = contract
            .query_nft_info(deps.as_ref().storage, nft_2.into())
            .unwrap();
        assert_eq!(nft_info_2.token_uri, uri_2);
        assert_eq!(
            nft_info_2.extension,
            Some(NftExtension {
                description: Some("other_description".into()),
                name: Some("name1".to_string()),
                attributes: Some(vec![Trait {
                    display_type: None,
                    trait_type: "type1".to_string(),
                    value: "value1".to_string(),
                }]),
                ..NftExtension::default()
            })
        );

        // query for token 2 with different description
        let res = contract
            .query_nft_by_extension(
                deps.as_ref().storage,
                Some(NftExtension {
                    description: Some("other_description".into()), // only description is different compared to nft 1
                    ..NftExtension::default()
                }),
                None,
                None,
            )
            .unwrap();
        assert!(res.is_some());
        let result = res.unwrap();
        assert_eq!(result.len(), 1);
        // get first element
        let nft = result.first().unwrap();
        assert_eq!(nft.token_uri, "ipfs://foo.bar".to_string().into());

        // query for both tokens
        let res = contract
            .query_nft_by_extension(
                deps.as_ref().storage,
                Some(NftExtension {
                    name: Some("name1".into()), // only description is different compared to nft 1
                    attributes: Some(vec![Trait {
                        display_type: None,
                        trait_type: "type1".to_string(),
                        value: "value1".to_string(),
                    }]),
                    ..NftExtension::default()
                }),
                None,
                None,
            )
            .unwrap();
        assert!(res.is_some());
        let result = res.unwrap();
        assert_eq!(result.len(), 2);
    }
}
