use crate::{
    execute::Cw721Execute,
    msg::{Cw721ExecuteMsg, Cw721InstantiateMsg},
    query::{Cw721Query, },
    state::{DefaultOptionMetadataExtension, Metadata, MINTER},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Empty,
};
use cosmwasm_std::testing::message_info;
use cw2::ContractVersion;
use unit_tests::{contract::Cw721Contract, multi_tests::CREATOR_ADDR};

use super::*;

/// Make sure cw2 version info is properly initialized during instantiation.
#[test]
fn proper_cw2_initialization() {
    let mut deps = mock_dependencies();

    Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default()
        .instantiate(
            deps.as_mut(),
            mock_env(),
            message_info(&deps.api.addr_make("larry"), &[]),
            Cw721InstantiateMsg {
                name: "collection_name".into(),
                symbol: "collection_symbol".into(),
                minter: Some("minter".into()),
                withdraw_address: None,
            },
            "contract_name",
            "contract_version",
        )
        .unwrap();

    let minter = MINTER
        .get_ownership(deps.as_ref().storage)
        .unwrap()
        .owner
        .map(|a| a.into_string());
    assert_eq!(minter, Some("minter".to_string()));

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
fn proper_owner_initialization() {
    let mut deps = mock_dependencies();

    let info_owner = message_info(&deps.api.addr_make("owner"), &[]);
    Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default()
        .instantiate(
            deps.as_mut(),
            mock_env(),
            info_owner.clone(),
            Cw721InstantiateMsg {
                name: "collection_name".into(),
                symbol: "collection_symbol".into(),
                minter: None,
                withdraw_address: None,
            },
            "contract_name",
            "contract_version",
        )
        .unwrap();

    let minter = MINTER.item.load(deps.as_ref().storage).unwrap().owner;
    assert_eq!(minter, Some(info_owner.sender));
}

#[test]
fn use_metadata_extension() {
    let mut deps = mock_dependencies();
    let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();

    let info = message_info(&deps.api.addr_make(CREATOR_ADDR), &[]);
    let init_msg = Cw721InstantiateMsg {
        name: "collection_name".into(),
        symbol: "collection_symbol".into(),
        minter: None,
        withdraw_address: None,
    };
    let env = mock_env();
    contract
        .instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            init_msg,
            "contract_name",
            "contract_version",
        )
        .unwrap();

    let token_id = "Enterprise";
    let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
    let extension = Some(Metadata {
        description: Some("Spaceship with Warp Drive".into()),
        name: Some("Starship USS Enterprise".to_string()),
        ..Metadata::default()
    });
    let exec_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.to_string(),
        owner: "john".to_string(),
        token_uri: token_uri.clone(),
        extension: extension.clone(),
    };
    contract
        .execute(deps.as_mut(), env.clone(), info, exec_msg)
        .unwrap();

    let res = contract
        .query_nft_info(deps.as_ref(), env, token_id.into())
        .unwrap();
    assert_eq!(res.token_uri, token_uri);
    assert_eq!(res.extension, extension);
}

/* v16 Migrations aren't supported.
#[test]
fn test_migrate() {
    let mut deps = mock_dependencies();

    let env = mock_env();
    use cw721_base_016 as v16;
    v16::entry::instantiate(
        deps.as_mut(),
        env.clone(),
        message_info(&deps.api.addr_make("owner"), &[]),
        v16::InstantiateMsg {
            name: "legacy_name".into(),
            symbol: "legacy_symbol".into(),
            minter: "legacy_minter".into(),
        },
    )
    .unwrap();

    // mint 200 NFTs before migration
    for i in 0..200 {
        let info = message_info(&deps.api.addr_make("legacy_minter"), &[]);
        let msg = v16::ExecuteMsg::Mint(v16::msg::MintMsg {
            token_id: i.to_string(),
            owner: "owner".into(),
            token_uri: None,
            extension: None,
        });
        v16::entry::execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    }

    // assert new data before migration:
    // - ownership and collection info throws NotFound Error
    MINTER.item.load(deps.as_ref().storage).unwrap_err(); // cw_ownable in v16 is used for minter
    let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
    contract
        .query_collection_info(deps.as_ref(), env.clone())
        .unwrap_err();
    // - query in new minter and creator ownership store throws NotFound Error (in v16 it was stored outside cw_ownable, in dedicated "minter" store)
    MINTER.get_ownership(deps.as_ref().storage).unwrap_err();
    // assert legacy data before migration:
    // - version
    let version = cw2::get_contract_version(deps.as_ref().storage)
        .unwrap()
        .version;
    assert_eq!(version, "0.16.0");
    // - legacy minter is set
    let legacy_minter_store: Item<Addr> = Item::new("minter");
    let legacy_minter = legacy_minter_store.load(deps.as_ref().storage).unwrap();
    assert_eq!(legacy_minter, "legacy_minter");
    // - legacy collection info is set
    let legacy_collection_info_store: Item<cw721_016::ContractInfoResponse> = Item::new("nft_info");
    let all_tokens = contract
        .query_all_tokens(deps.as_ref(), env.clone(), None, Some(MAX_LIMIT))
        .unwrap();
    assert_eq!(all_tokens.tokens.len(), 200);
    for token_id in 0..200 {
        let token = contract
            .query_owner_of(deps.as_ref(), env.clone(), token_id.to_string(), false)
            .unwrap();
        assert_eq!(token.owner.as_str(), "owner");
    }

    Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default()
        .migrate(
            deps.as_mut(),
            env.clone(),
            crate::msg::Cw721MigrateMsg::WithUpdate {
                minter: None,
                creator: None,
            },
            "contract_name",
            "contract_version",
        )
        .unwrap();

    // version
    let version = cw2::get_contract_version(deps.as_ref().storage)
        .unwrap()
        .version;
    assert_eq!(version, "contract_version");
    assert_ne!(version, "0.16.0");

    // assert minter ownership
    let minter_ownership = MINTER
        .get_ownership(deps.as_ref().storage)
        .unwrap()
        .owner
        .map(|a| a.into_string());
    assert_eq!(minter_ownership, Some("legacy_minter".to_string()));

    // assert collection info
    let collection_info = contract
        .query_collection_info(deps.as_ref(), env.clone())
        .unwrap();
    let legacy_contract_info = CollectionInfo {
        name: "legacy_name".to_string(),
        symbol: "legacy_symbol".to_string(),
    };
    assert_eq!(collection_info, legacy_contract_info);

    // assert tokens
    let all_tokens = contract
        .query_all_tokens(deps.as_ref(), env.clone(), None, Some(MAX_LIMIT))
        .unwrap();
    assert_eq!(all_tokens.tokens.len(), 200);

    // assert legacy data is still there (allowing backward migration in case of issues)
    // - minter
    let legacy_minter = legacy_minter_store.load(deps.as_ref().storage).unwrap();
    assert_eq!(legacy_minter, "legacy_minter");
    // - collection info
    let legacy_collection_info = legacy_collection_info_store
        .load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(legacy_collection_info.name, "legacy_name");
    assert_eq!(legacy_collection_info.symbol, "legacy_symbol");
    // - tokens are unchanged/still exist
    let all_tokens = contract
        .query_all_tokens(deps.as_ref(), env.clone(), None, Some(MAX_LIMIT))
        .unwrap();
    assert_eq!(all_tokens.tokens.len(), 200);
    for token_id in 0..200 {
        let token = contract
            .query_owner_of(deps.as_ref(), env.clone(), token_id.to_string(), false)
            .unwrap();
        assert_eq!(token.owner.as_str(), "owner");
    }

}
 */
