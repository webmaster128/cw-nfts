use crate::{
    error::Cw721ContractError,
    execute::Cw721Execute,
    msg::{
        Cw721ExecuteMsg, Cw721InstantiateMsg, Cw721MigrateMsg, Cw721QueryMsg, MinterResponse,
        OwnerOfResponse,
    },
    query::Cw721Query,
    state::DefaultOptionMetadataExtension,
};
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, QuerierWrapper, Response,
    StdResult, WasmMsg,
};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_ownable::{Ownership, OwnershipError};
use cw_utils::Expiration;

use super::contract::Cw721Contract;

pub const CREATOR_ADDR: &str = "creator";
pub const MINTER_ADDR: &str = "minter";
pub const OTHER_ADDR: &str = "other";
pub const NFT_OWNER_ADDR: &str = "nft_owner";

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721InstantiateMsg,
) -> Result<Response, Cw721ContractError> {
    let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
    contract.instantiate(deps, env, info, msg, "contract_name", "contract_version")
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ExecuteMsg<DefaultOptionMetadataExtension, Empty>,
) -> Result<Response, Cw721ContractError> {
    let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
    contract.execute(deps, env, info, msg)
}

pub fn query(
    deps: Deps,
    env: Env,
    msg: Cw721QueryMsg<DefaultOptionMetadataExtension>,
) -> StdResult<Binary> {
    let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
    contract.query(deps, env, msg)
}

pub fn migrate(
    deps: DepsMut,
    env: Env,
    msg: Cw721MigrateMsg,
) -> Result<Response, Cw721ContractError> {
    let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
    contract.migrate(deps, env, msg, "contract_name", "contract_version")
}

fn cw721_base_latest_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
    Box::new(contract)
}
/* cw16 not supported
fn cw721_base_016_contract() -> Box<dyn Contract<Empty>> {
    use cw721_base_016 as v16;
    let contract = ContractWrapper::new(
        v16::entry::execute,
        v16::entry::instantiate,
        v16::entry::query,
    );
    Box::new(contract)
}

 */

fn cw721_base_017_contract() -> Box<dyn Contract<Empty>> {
    use cw721_base_017 as v17;
    let contract = ContractWrapper::new(
        v17::entry::execute,
        v17::entry::instantiate,
        v17::entry::query,
    );
    Box::new(contract)
}

fn cw721_base_018_contract() -> Box<dyn Contract<Empty>> {
    use cw721_base_018 as v18;
    let contract = ContractWrapper::new(
        v18::entry::execute,
        v18::entry::instantiate,
        v18::entry::query,
    );
    Box::new(contract)
}

fn query_owner(querier: QuerierWrapper, cw721: &Addr, token_id: String) -> Addr {
    let resp: OwnerOfResponse = querier
        .query_wasm_smart(
            cw721,
            &Cw721QueryMsg::<Empty>::OwnerOf {
                token_id,
                include_expired: None,
            },
        )
        .unwrap();
    Addr::unchecked(resp.owner)
}

fn mint_transfer_and_burn(app: &mut App, cw721: Addr, sender: Addr, token_id: String) {
    app.execute_contract(
        sender.clone(),
        cw721.clone(),
        &Cw721ExecuteMsg::<Empty, Empty>::Mint {
            token_id: token_id.clone(),
            owner: sender.to_string(),
            token_uri: None,
            extension: Empty::default(),
        },
        &[],
    )
    .unwrap();

    let owner = query_owner(app.wrap(), &cw721, token_id.clone());
    assert_eq!(owner.to_string(), sender.to_string());

    app.execute_contract(
        sender,
        cw721.clone(),
        &Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
            recipient: "burner".to_string(),
            token_id: token_id.clone(),
        },
        &[],
    )
    .unwrap();

    let owner = query_owner(app.wrap(), &cw721, token_id.clone());
    assert_eq!(owner.to_string(), "burner".to_string());

    app.execute_contract(
        Addr::unchecked("burner"),
        cw721,
        &Cw721ExecuteMsg::<Empty, Empty>::Burn { token_id },
        &[],
    )
    .unwrap();
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
            &Cw721InstantiateMsg {
                name: "collection".to_string(),
                symbol: "symbol".to_string(),
                minter: Some(MINTER_ADDR.to_string()),
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
        &Cw721ExecuteMsg::<Empty, Empty>::Mint {
            token_id: "1".to_string(),
            owner: nft_owner.to_string(),
            token_uri: None,
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
        &Cw721ExecuteMsg::<Empty, Empty>::ApproveAll {
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
        &Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
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
            &Cw721QueryMsg::<Empty>::OwnerOf {
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
            &Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
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
        &Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
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
            &Cw721QueryMsg::<Empty>::OwnerOf {
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
        &Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
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
            &Cw721QueryMsg::<Empty>::OwnerOf {
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
        &Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
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
        &Cw721ExecuteMsg::<Empty, Empty>::RevokeAll {
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
            &Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
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

/// Instantiates a 0.16 version of this contract and tests that tokens
/// can be minted, transferred, and burnred after migration.
#[test]
fn test_migration_legacy_to_latest() {
    // case 1: migrate from v0.16 to latest by using existing minter addr
    {
        use cw721_base_017 as v17;
        let mut app = App::default();
        let admin = Addr::unchecked("admin");

        let code_id_017 = app.store_code(cw721_base_017_contract());
        let code_id_latest = app.store_code(cw721_base_latest_contract());

        let legacy_creator_and_minter = Addr::unchecked("legacy_creator_and_minter");

        let cw721 = app
            .instantiate_contract(
                code_id_017,
                legacy_creator_and_minter.clone(),
                &v17::InstantiateMsg {
                    name: "collection".to_string(),
                    symbol: "symbol".to_string(),
                    minter: legacy_creator_and_minter.to_string(),
                },
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap();

        mint_transfer_and_burn(
            &mut app,
            cw721.clone(),
            legacy_creator_and_minter.clone(),
            "1".to_string(),
        );

        // migrate
        app.execute(
            admin,
            WasmMsg::Migrate {
                contract_addr: cw721.to_string(),
                new_code_id: code_id_latest,
                msg: to_json_binary(&Cw721MigrateMsg::WithUpdate {
                    minter: None,
                    creator: None,
                })
                .unwrap(),
            }
            .into(),
        )
        .unwrap();

        // non-minter user cant mint
        let other = Addr::unchecked(OTHER_ADDR);
        let err: Cw721ContractError = app
            .execute_contract(
                other.clone(),
                cw721.clone(),
                &Cw721ExecuteMsg::<Empty, Empty>::Mint {
                    token_id: "1".to_string(),
                    owner: other.to_string(),
                    token_uri: None,
                    extension: Empty::default(),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

        // legacy minter can still mint
        mint_transfer_and_burn(
            &mut app,
            cw721.clone(),
            legacy_creator_and_minter.clone(),
            "1".to_string(),
        );

        // check new mint query response works.
        let m: MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(legacy_creator_and_minter.to_string()));

        // check that the new response is backwards compatable when minter
        // is not None.
        let m: v17::MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(legacy_creator_and_minter.to_string()));

        // check minter ownership query works
        let minter_ownership: Ownership<Addr> = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Ownership {})
            .unwrap();
        assert_eq!(minter_ownership.owner, Some(legacy_creator_and_minter));
    }
    // case 2: migrate from v0.16 to latest by providing new creator and minter addr
    {
        use cw721_base_017 as v17;
        let mut app = App::default();
        let admin = Addr::unchecked("admin");

        let code_id_017 = app.store_code(cw721_base_017_contract());
        let code_id_latest = app.store_code(cw721_base_latest_contract());

        let legacy_creator_and_minter = Addr::unchecked("legacy_creator_and_minter");

        let cw721 = app
            .instantiate_contract(
                code_id_017,
                legacy_creator_and_minter.clone(),
                &v17::InstantiateMsg {
                    name: "collection".to_string(),
                    symbol: "symbol".to_string(),
                    minter: legacy_creator_and_minter.to_string(),
                },
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap();

        mint_transfer_and_burn(
            &mut app,
            cw721.clone(),
            legacy_creator_and_minter.clone(),
            "1".to_string(),
        );

        // migrate
        app.execute(
            admin,
            WasmMsg::Migrate {
                contract_addr: cw721.to_string(),
                new_code_id: code_id_latest,
                msg: to_json_binary(&Cw721MigrateMsg::WithUpdate {
                    minter: Some(MINTER_ADDR.to_string()),
                    creator: Some(CREATOR_ADDR.to_string()),
                })
                .unwrap(),
            }
            .into(),
        )
        .unwrap();

        // legacy minter user cant mint
        let err: Cw721ContractError = app
            .execute_contract(
                legacy_creator_and_minter.clone(),
                cw721.clone(),
                &Cw721ExecuteMsg::<Empty, Empty>::Mint {
                    token_id: "1".to_string(),
                    owner: legacy_creator_and_minter.to_string(),
                    token_uri: None,
                    extension: Empty::default(),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

        // new minter can mint
        let minter = Addr::unchecked(MINTER_ADDR);
        mint_transfer_and_burn(&mut app, cw721.clone(), minter.clone(), "1".to_string());

        // check new mint query response works.
        let m: MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(minter.to_string()));

        // check that the new response is backwards compatable when minter
        // is not None.
        let m: v17::MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter,Some(minter.to_string()));

        // check minter ownership query works
        let minter_ownership: Ownership<Addr> = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Ownership {})
            .unwrap();
        assert_eq!(minter_ownership.owner, Some(minter));
    }
    // case 3: migrate from v0.17 to latest by using existing minter addr
    {
        use cw721_base_017 as v17;
        let mut app = App::default();
        let admin = Addr::unchecked("admin");

        let code_id_017 = app.store_code(cw721_base_017_contract());
        let code_id_latest = app.store_code(cw721_base_latest_contract());

        let legacy_creator_and_minter = Addr::unchecked("legacy_creator_and_minter");

        let cw721 = app
            .instantiate_contract(
                code_id_017,
                legacy_creator_and_minter.clone(),
                &v17::InstantiateMsg {
                    name: "collection".to_string(),
                    symbol: "symbol".to_string(),
                    minter: legacy_creator_and_minter.to_string(),
                },
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap();

        mint_transfer_and_burn(
            &mut app,
            cw721.clone(),
            legacy_creator_and_minter.clone(),
            "1".to_string(),
        );

        // migrate
        app.execute(
            admin,
            WasmMsg::Migrate {
                contract_addr: cw721.to_string(),
                new_code_id: code_id_latest,
                msg: to_json_binary(&Cw721MigrateMsg::WithUpdate {
                    minter: None,
                    creator: None,
                })
                .unwrap(),
            }
            .into(),
        )
        .unwrap();

        // non-minter user cant mint
        let other = Addr::unchecked(OTHER_ADDR);
        let err: Cw721ContractError = app
            .execute_contract(
                other.clone(),
                cw721.clone(),
                &Cw721ExecuteMsg::<Empty, Empty>::Mint {
                    token_id: "1".to_string(),
                    owner: other.to_string(),
                    token_uri: None,
                    extension: Empty::default(),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

        // legacy minter can still mint
        mint_transfer_and_burn(
            &mut app,
            cw721.clone(),
            legacy_creator_and_minter.clone(),
            "1".to_string(),
        );

        // check new mint query response works.
        let m: MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(legacy_creator_and_minter.to_string()));

        // check that the new response is backwards compatable when minter
        // is not None.
        let m: v17::MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(legacy_creator_and_minter.to_string()));

        // check minter ownership query works
        let minter_ownership: Ownership<Addr> = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Ownership {})
            .unwrap();
        assert_eq!(minter_ownership.owner, Some(legacy_creator_and_minter));
    }
    // case 4: migrate from v0.17 to latest by providing new creator and minter addr
    {
        use cw721_base_017 as v17;
        let mut app = App::default();
        let admin = Addr::unchecked("admin");

        let code_id_017 = app.store_code(cw721_base_017_contract());
        let code_id_latest = app.store_code(cw721_base_latest_contract());

        let legacy_creator_and_minter = Addr::unchecked("legacy_creator_and_minter");

        let cw721 = app
            .instantiate_contract(
                code_id_017,
                legacy_creator_and_minter.clone(),
                &v17::InstantiateMsg {
                    name: "collection".to_string(),
                    symbol: "symbol".to_string(),
                    minter: legacy_creator_and_minter.to_string(),
                },
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap();

        mint_transfer_and_burn(
            &mut app,
            cw721.clone(),
            legacy_creator_and_minter.clone(),
            "1".to_string(),
        );

        // migrate
        app.execute(
            admin,
            WasmMsg::Migrate {
                contract_addr: cw721.to_string(),
                new_code_id: code_id_latest,
                msg: to_json_binary(&Cw721MigrateMsg::WithUpdate {
                    minter: Some(MINTER_ADDR.to_string()),
                    creator: Some(CREATOR_ADDR.to_string()),
                })
                .unwrap(),
            }
            .into(),
        )
        .unwrap();

        // legacy minter user cant mint
        let err: Cw721ContractError = app
            .execute_contract(
                legacy_creator_and_minter.clone(),
                cw721.clone(),
                &Cw721ExecuteMsg::<Empty, Empty>::Mint {
                    token_id: "1".to_string(),
                    owner: legacy_creator_and_minter.to_string(),
                    token_uri: None,
                    extension: Empty::default(),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

        // new minter can mint
        let minter = Addr::unchecked(MINTER_ADDR);
        mint_transfer_and_burn(&mut app, cw721.clone(), minter.clone(), "1".to_string());

        // check new mint query response works.
        let m: MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(minter.to_string()));

        // check that the new response is backwards compatable when minter
        // is not None.
        let m: v17::MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(minter.to_string()));

        // check minter ownership query works
        let minter_ownership: Ownership<Addr> = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Ownership {})
            .unwrap();
        assert_eq!(minter_ownership.owner, Some(minter));
    }
    // case 5: migrate from v0.18 to latest by using existing minter addr
    {
        use cw721_base_018 as v18;
        let mut app = App::default();
        let admin = Addr::unchecked("admin");

        let code_id_018 = app.store_code(cw721_base_018_contract());
        let code_id_latest = app.store_code(cw721_base_latest_contract());

        let legacy_creator_and_minter = Addr::unchecked("legacy_creator_and_minter");

        let cw721 = app
            .instantiate_contract(
                code_id_018,
                legacy_creator_and_minter.clone(),
                &v18::InstantiateMsg {
                    name: "collection".to_string(),
                    symbol: "symbol".to_string(),
                    minter: legacy_creator_and_minter.to_string(),
                },
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap();

        mint_transfer_and_burn(
            &mut app,
            cw721.clone(),
            legacy_creator_and_minter.clone(),
            "1".to_string(),
        );

        // migrate
        app.execute(
            admin,
            WasmMsg::Migrate {
                contract_addr: cw721.to_string(),
                new_code_id: code_id_latest,
                msg: to_json_binary(&Cw721MigrateMsg::WithUpdate {
                    minter: None,
                    creator: None,
                })
                .unwrap(),
            }
            .into(),
        )
        .unwrap();

        // non-minter user cant mint
        let other = Addr::unchecked(OTHER_ADDR);
        let err: Cw721ContractError = app
            .execute_contract(
                other.clone(),
                cw721.clone(),
                &Cw721ExecuteMsg::<Empty, Empty>::Mint {
                    token_id: "1".to_string(),
                    owner: other.to_string(),
                    token_uri: None,
                    extension: Empty::default(),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

        // legacy minter can still mint
        mint_transfer_and_burn(
            &mut app,
            cw721.clone(),
            legacy_creator_and_minter.clone(),
            "1".to_string(),
        );

        // check new mint query response works.
        let m: MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(legacy_creator_and_minter.to_string()));

        // check that the new response is backwards compatable when minter
        // is not None.
        let m: v18::MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(legacy_creator_and_minter.to_string()));

        // check minter ownership query works
        let minter_ownership: Ownership<Addr> = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Ownership {})
            .unwrap();
        assert_eq!(minter_ownership.owner, Some(legacy_creator_and_minter));
    }
    // case 6: migrate from v0.18 to latest by providing new creator and minter addr
    {
        use cw721_base_018 as v18;
        let mut app = App::default();
        let admin = Addr::unchecked("admin");

        let code_id_018 = app.store_code(cw721_base_018_contract());
        let code_id_latest = app.store_code(cw721_base_latest_contract());

        let legacy_creator_and_minter = Addr::unchecked("legacy_creator_and_minter");

        let cw721 = app
            .instantiate_contract(
                code_id_018,
                legacy_creator_and_minter.clone(),
                &v18::InstantiateMsg {
                    name: "collection".to_string(),
                    symbol: "symbol".to_string(),
                    minter: legacy_creator_and_minter.to_string(),
                },
                &[],
                "cw721-base",
                Some(admin.to_string()),
            )
            .unwrap();

        mint_transfer_and_burn(
            &mut app,
            cw721.clone(),
            legacy_creator_and_minter.clone(),
            "1".to_string(),
        );

        // migrate
        app.execute(
            admin,
            WasmMsg::Migrate {
                contract_addr: cw721.to_string(),
                new_code_id: code_id_latest,
                msg: to_json_binary(&Cw721MigrateMsg::WithUpdate {
                    minter: Some(MINTER_ADDR.to_string()),
                    creator: Some(CREATOR_ADDR.to_string()),
                })
                .unwrap(),
            }
            .into(),
        )
        .unwrap();

        // legacy minter user cant mint
        let err: Cw721ContractError = app
            .execute_contract(
                legacy_creator_and_minter.clone(),
                cw721.clone(),
                &Cw721ExecuteMsg::<Empty, Empty>::Mint {
                    token_id: "1".to_string(),
                    owner: legacy_creator_and_minter.to_string(),
                    token_uri: None,
                    extension: Empty::default(),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

        // new minter can mint
        let minter = Addr::unchecked(MINTER_ADDR);
        mint_transfer_and_burn(&mut app, cw721.clone(), minter.clone(), "1".to_string());

        // check new mint query response works.
        let m: MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(minter.to_string()));

        // check that the new response is backwards compatable when minter
        // is not None.
        let m: v18::MinterResponse = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Minter {})
            .unwrap();
        assert_eq!(m.minter, Some(minter.to_string()));

        // check minter ownership query works
        let minter_ownership: Ownership<Addr> = app
            .wrap()
            .query_wasm_smart(&cw721, &Cw721QueryMsg::<Empty>::Ownership {})
            .unwrap();
        assert_eq!(minter_ownership.owner, Some(minter));
    }
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
        .query_wasm_smart(cw721, &Cw721QueryMsg::<Empty>::GetWithdrawAddress {})
        .unwrap();
    assert!(withdraw_addr.is_none());
}
