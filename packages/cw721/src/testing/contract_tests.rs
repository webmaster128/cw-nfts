#![cfg(test)]

use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

use cosmwasm_std::{
    from_json, to_json_binary, Addr, Coin, CosmosMsg, DepsMut, Empty, Response, StdError, WasmMsg,
};

use crate::error::Cw721ContractError;
use crate::msg::{ApprovalResponse, MinterResponse, NftInfoResponse, OperatorResponse, OperatorsResponse, OwnerOfResponse};
use crate::msg::{Cw721ExecuteMsg, Cw721InstantiateMsg, Cw721QueryMsg};
use crate::receiver::Cw721ReceiveMsg;
use crate::state::{CollectionInfo, DefaultOptionMetadataExtension, MINTER};
use crate::{execute::Cw721Execute, query::Cw721Query, Approval, Expiration};
use cw_ownable::{Action, Ownership, OwnershipError};

use super::contract::Cw721Contract;

const MINTER_ADDR: &str = "minter";
const CREATOR: &str = "creator";
const CONTRACT_NAME: &str = "Magic Power";
const SYMBOL: &str = "MGK";

fn setup_contract(
    deps: DepsMut<'_>,
    creator: Addr,
    minter: Addr,
) -> Cw721Contract<'static, DefaultOptionMetadataExtension, Empty, Empty> {
    let contract = Cw721Contract::default();
    let msg = Cw721InstantiateMsg {
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: Some(minter.to_string()),
        withdraw_address: None,
    };
    let info = message_info(&creator, &[]);
    let res = contract
        .instantiate(
            deps,
            mock_env(),
            info,
            msg,
            "contract_name",
            "contract_version",
        )
        .unwrap();
    assert_eq!(0, res.messages.len());
    contract
}

#[test]
fn proper_instantiation() {
    let mut deps = mock_dependencies();
    let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
    let minter = deps.api.addr_make(MINTER_ADDR);
    let creator = deps.api.addr_make(CREATOR);
    let msg = Cw721InstantiateMsg {
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: Some(minter.to_string()),
        withdraw_address: Some(creator.to_string()),
    };
    let info = message_info(&creator, &[]);
    let env = mock_env();

    // we can just call .unwrap() to assert this was a success
    let res = contract
        .instantiate(
            deps.as_mut(),
            env.clone(),
            info,
            msg,
            "contract_name",
            "contract_version",
        )
        .unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = contract.minter(deps.as_ref()).unwrap();
    assert_eq!(Some(minter.to_string()), res.minter);
    let collection_info = contract
        .query_collection_info(deps.as_ref(), env.clone())
        .unwrap();
    assert_eq!(
        collection_info,
        CollectionInfo {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
        }
    );

    let withdraw_address = contract
        .config
        .withdraw_address
        .may_load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(Some(creator.to_string()), withdraw_address);

    let count = contract
        .query_num_tokens(deps.as_ref(), env.clone())
        .unwrap();
    assert_eq!(0, count.count);

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), env, None, None)
        .unwrap();
    assert_eq!(0, tokens.tokens.len());
}

#[test]
fn proper_instantiation_with_collection_info() {
    let mut deps = mock_dependencies();
    let contract = Cw721Contract::<DefaultOptionMetadataExtension, Empty, Empty>::default();
    let creator = deps.api.addr_make(CREATOR);
    let minter = deps.api.addr_make(MINTER_ADDR);
    let msg = Cw721InstantiateMsg {
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: Some(minter.to_string()),
        withdraw_address: Some(creator.to_string()),
    };
    let collection_info = message_info(&creator, &[]);
    let env = mock_env();

    // we can just call .unwrap() to assert this was a success
    let res = contract
        .instantiate(
            deps.as_mut(),
            env.clone(),
            collection_info,
            msg,
            "contract_name",
            "contract_version",
        )
        .unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let minter_ownership = MINTER.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(minter), minter_ownership.owner);
    let info = contract
        .query_collection_info(deps.as_ref(), env.clone())
        .unwrap();
    assert_eq!(
        info,
        CollectionInfo {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
        }
    );

    let withdraw_address = contract
        .config
        .withdraw_address
        .may_load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(Some(creator.to_string()), withdraw_address);

    let count = contract
        .query_num_tokens(deps.as_ref(), env.clone())
        .unwrap();
    assert_eq!(0, count.count);

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), env, None, None)
        .unwrap();
    assert_eq!(0, tokens.tokens.len());
}

#[test]
fn minting() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make(CREATOR);
    let minter = deps.api.addr_make(MINTER_ADDR);
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());

    let token_id = "petrify".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: deps.api.addr_make("medusa").to_string(),
        token_uri: Some(token_uri.clone()),
        extension: None,
    };

    // random cannot mint
    let random = message_info(&deps.api.addr_make("random"), &[]);
    let env = mock_env();
    let err = contract
        .execute(deps.as_mut(), env.clone(), random, mint_msg.clone())
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // minter can mint
    let allowed = message_info(&minter, &[]);
    let _ = contract
        .execute(deps.as_mut(), env.clone(), allowed, mint_msg)
        .unwrap();

    // ensure num tokens increases
    let count = contract
        .query_num_tokens(deps.as_ref(), env.clone())
        .unwrap();
    assert_eq!(1, count.count);

    // unknown nft returns error
    let _ = contract
        .query_nft_info(deps.as_ref(), env.clone(), "unknown".to_string())
        .unwrap_err();

    // this nft info is correct
    let info = contract
        .query_nft_info(deps.as_ref(), env.clone(), token_id.clone())
        .unwrap();
    assert_eq!(
        info,
        NftInfoResponse::<DefaultOptionMetadataExtension> {
            token_uri: Some(token_uri),
            extension: None,
        }
    );

    // owner info is correct
    let owner = contract
        .query_owner_of(deps.as_ref(), mock_env(), token_id.clone(), true)
        .unwrap();
    assert_eq!(
        owner,
        OwnerOfResponse {
            owner: deps.api.addr_make("medusa").to_string(),
            approvals: vec![],
        }
    );

    // Cannot mint same token_id again
    let mint_msg2 = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: deps.api.addr_make("hercules").to_string(),
        token_uri: None,
        extension: None,
    };

    let allowed = message_info(&minter, &[]);
    let err = contract
        .execute(deps.as_mut(), mock_env(), allowed, mint_msg2)
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Claimed {});

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), env, None, None)
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id], tokens.tokens);
}

#[test]
fn test_update_minter() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make("creator");
    let minter = deps.api.addr_make(MINTER_ADDR);
    let random = deps.api.addr_make("random");
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());

    let token_id = "petrify".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id,
        owner: deps.api.addr_make("medusa").to_string(),
        token_uri: Some(token_uri.clone()),
        extension: None,
    };

    // Minter can mint
    let minter_info = message_info(&minter, &[]);
    let _ = contract
        .execute(deps.as_mut(), mock_env(), minter_info.clone(), mint_msg)
        .unwrap();

    // Update the owner to "random". The new owner should be able to
    // mint new tokens, the old one should not.
    contract
        .execute(
            deps.as_mut(),
            mock_env(),
            minter_info.clone(),
            Cw721ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
                new_owner: random.to_string(),
                expiry: None,
            }),
        )
        .unwrap();

    // Minter does not change until ownership transfer completes.
    let minter_response: MinterResponse = from_json(
        contract
            .query(deps.as_ref(), mock_env(), Cw721QueryMsg::Minter {})
            .unwrap(),
    )
    .unwrap();
    assert_eq!(minter_response.minter, Some(minter.to_string()));

    // Pending ownership transfer should be discoverable via query.
    let ownership: Ownership<Addr> = from_json(
        contract
            .query(deps.as_ref(), mock_env(), Cw721QueryMsg::Ownership {})
            .unwrap(),
    )
    .unwrap();

    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(minter),
            pending_owner: Some(random.clone()),
            pending_expiry: None,
        }
    );

    // Accept the ownership transfer.
    let random_info = message_info(&random, &[]);
    contract
        .execute(
            deps.as_mut(),
            mock_env(),
            random_info.clone(),
            Cw721ExecuteMsg::UpdateOwnership(Action::AcceptOwnership),
        )
        .unwrap();

    // Minter changes after ownership transfer is accepted.
    let minter_ownership: Ownership<Addr> = from_json(
        contract
            .query(deps.as_ref(), mock_env(), Cw721QueryMsg::Ownership {})
            .unwrap(),
    )
    .unwrap();
    assert_eq!(MINTER.minter, Some(random.to_string()));

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: "randoms_token".to_string(),
        owner: deps.api.addr_make("medusa").to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    // Old owner can not mint.
    let err: Cw721ContractError = contract
        .execute(deps.as_mut(), mock_env(), minter_info, mint_msg.clone())
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // New owner can mint.
    let _ = contract
        .execute(deps.as_mut(), mock_env(), random_info, mint_msg)
        .unwrap();
}

#[test]
fn burning() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make(CREATOR);
    let minter = deps.api.addr_make(MINTER_ADDR);
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());

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
        .execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg)
        .unwrap();

    // random not allowed to burn
    let random = message_info(&deps.api.addr_make("random"), &[]);
    let env = mock_env();

    let err = contract
        .execute(deps.as_mut(), env.clone(), random, burn_msg.clone())
        .unwrap_err();

    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    let _ = contract
        .execute(deps.as_mut(), env.clone(), allowed, burn_msg)
        .unwrap();

    // ensure num tokens decreases
    let count = contract
        .query_num_tokens(deps.as_ref(), env.clone())
        .unwrap();
    assert_eq!(0, count.count);

    // trying to get nft returns error
    let _ = contract
        .query_nft_info(deps.as_ref(), env.clone(), "petrify".to_string())
        .unwrap_err();

    // list the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), env, None, None)
        .unwrap();
    assert!(tokens.tokens.is_empty());
}

#[test]
fn transferring_nft() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make(CREATOR);
    let minter = deps.api.addr_make(MINTER_ADDR);
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());

    // Mint a token
    let token_id = "melt".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/melt".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: deps.api.addr_make("venus").to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    let minter = message_info(&minter, &[]);
    contract
        .execute(deps.as_mut(), mock_env(), minter, mint_msg)
        .unwrap();

    // random cannot transfer
    let random = message_info(&deps.api.addr_make("random"), &[]);
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: deps.api.addr_make("random").to_string(),
        token_id: token_id.clone(),
    };

    let err = contract
        .execute(deps.as_mut(), mock_env(), random, transfer_msg)
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // owner can
    let random = message_info(&deps.api.addr_make("venus"), &[]);
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: deps.api.addr_make("random").to_string(),
        token_id: token_id.clone(),
    };

    let res = contract
        .execute(deps.as_mut(), mock_env(), random, transfer_msg)
        .unwrap();

    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", deps.api.addr_make("venus").as_ref())
            .add_attribute("recipient", deps.api.addr_make("random").as_ref())
            .add_attribute("token_id", token_id)
    );
}

#[test]
fn sending_nft() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make(CREATOR);
    let minter = deps.api.addr_make(MINTER_ADDR);
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());

    // Mint a token
    let token_id = "melt".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/melt".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: deps.api.addr_make("venus").to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    let minter = message_info(&minter, &[]);
    contract
        .execute(deps.as_mut(), mock_env(), minter, mint_msg)
        .unwrap();

    let msg = to_json_binary("You now have the melting power").unwrap();
    let target = deps.api.addr_make("another_contract");
    let send_msg = Cw721ExecuteMsg::SendNft {
        contract: target.to_string(),
        token_id: token_id.clone(),
        msg: msg.clone(),
    };

    let random = message_info(&deps.api.addr_make("random"), &[]);
    let err = contract
        .execute(deps.as_mut(), mock_env(), random, send_msg.clone())
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // but owner can
    let random = message_info(&deps.api.addr_make("venus"), &[]);
    let res = contract
        .execute(deps.as_mut(), mock_env(), random, send_msg)
        .unwrap();

    let payload = Cw721ReceiveMsg {
        sender: deps.api.addr_make("venus").to_string(),
        token_id: token_id.clone(),
        msg,
    };
    let expected = payload.into_cosmos_msg(target.clone()).unwrap();
    // ensure expected serializes as we think it should
    match &expected {
        CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) => {
            assert_eq!(contract_addr, target.as_ref())
        }
        _m => panic!("Unexpected message type: {_m:?}"),
    }
    // and make sure this is the request sent by the contract
    assert_eq!(
        res,
        Response::new()
            .add_message(expected)
            .add_attribute("action", "send_nft")
            .add_attribute("sender", deps.api.addr_make("venus").as_ref())
            .add_attribute("recipient", deps.api.addr_make("another_contract").as_ref())
            .add_attribute("token_id", token_id)
    );
}

#[test]
fn approving_revoking() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make(CREATOR);
    let minter = deps.api.addr_make(MINTER_ADDR);
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());

    // Mint a token
    let token_id = "grow".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/grow".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: deps.api.addr_make("demeter").to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    let minter = message_info(&minter, &[]);
    contract
        .execute(deps.as_mut(), mock_env(), minter, mint_msg)
        .unwrap();

    // token owner shows in approval query
    let res = contract
        .query_approval(
            deps.as_ref(),
            mock_env(),
            token_id.clone(),
            deps.api.addr_make("demeter").to_string(),
            false,
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovalResponse {
            approval: Approval {
                spender: deps.api.addr_make("demeter"),
                expires: Expiration::Never {}
            }
        }
    );

    // Give random transferring power
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: deps.api.addr_make("random").to_string(),
        token_id: token_id.clone(),
        expires: None,
    };
    let owner = message_info(&deps.api.addr_make("demeter"), &[]);
    let res = contract
        .execute(deps.as_mut(), mock_env(), owner, approve_msg)
        .unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "approve")
            .add_attribute("sender", deps.api.addr_make("demeter").as_ref())
            .add_attribute("spender", deps.api.addr_make("random").as_ref())
            .add_attribute("token_id", token_id.clone())
    );

    // test approval query
    let res = contract
        .query_approval(
            deps.as_ref(),
            mock_env(),
            token_id.clone(),
            deps.api.addr_make("random").to_string(),
            true,
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovalResponse {
            approval: Approval {
                spender: deps.api.addr_make("random"),
                expires: Expiration::Never {}
            }
        }
    );

    // random can now transfer
    let random = message_info(&deps.api.addr_make("random"), &[]);
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: deps.api.addr_make("person").to_string(),
        token_id: token_id.clone(),
    };
    contract
        .execute(deps.as_mut(), mock_env(), random, transfer_msg)
        .unwrap();

    // Approvals are removed / cleared
    let query_msg = Cw721QueryMsg::OwnerOf {
        token_id: token_id.clone(),
        include_expired: None,
    };
    let res: OwnerOfResponse = from_json(
        contract
            .query(deps.as_ref(), mock_env(), query_msg.clone())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        res,
        OwnerOfResponse {
            owner: deps.api.addr_make("person").to_string(),
            approvals: vec![],
        }
    );

    // Approve, revoke, and check for empty, to test revoke
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: deps.api.addr_make("random").to_string(),
        token_id: token_id.clone(),
        expires: None,
    };
    let owner = message_info(&deps.api.addr_make("person"), &[]);
    contract
        .execute(deps.as_mut(), mock_env(), owner.clone(), approve_msg)
        .unwrap();

    let revoke_msg = Cw721ExecuteMsg::Revoke {
        spender: deps.api.addr_make("random").to_string(),
        token_id,
    };
    contract
        .execute(deps.as_mut(), mock_env(), owner, revoke_msg)
        .unwrap();

    // Approvals are now removed / cleared
    let res: OwnerOfResponse = from_json(
        contract
            .query(deps.as_ref(), mock_env(), query_msg)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        res,
        OwnerOfResponse {
            owner: deps.api.addr_make("person").to_string(),
            approvals: vec![],
        }
    );
}

#[test]
fn approving_all_revoking_all() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make(CREATOR);
    let minter = deps.api.addr_make(MINTER_ADDR);
    let contract = setup_contract(deps.as_mut(), creator, minter);

    // Mint a couple tokens (from the same owner)
    let token_id1 = "grow1".to_string();
    let token_uri1 = "https://www.merriam-webster.com/dictionary/grow1".to_string();

    let token_id2 = "grow2".to_string();
    let token_uri2 = "https://www.merriam-webster.com/dictionary/grow2".to_string();

    let mint_msg1 = Cw721ExecuteMsg::Mint {
        token_id: token_id1.clone(),
        owner: deps.api.addr_make("demeter").to_string(),
        token_uri: Some(token_uri1),
        extension: None,
    };

    let minter = message_info(&deps.api.addr_make(MINTER_ADDR), &[]);
    contract
        .execute(deps.as_mut(), mock_env(), minter.clone(), mint_msg1)
        .unwrap();

    let mint_msg2 = Cw721ExecuteMsg::Mint {
        token_id: token_id2.clone(),
        owner: deps.api.addr_make("demeter").to_string(),
        token_uri: Some(token_uri2),
        extension: None,
    };

    let env = mock_env();
    contract
        .execute(deps.as_mut(), env.clone(), minter, mint_msg2)
        .unwrap();

    // paginate the token_ids
    let tokens = contract
        .query_all_tokens(deps.as_ref(), env.clone(), None, Some(1))
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id1.clone()], tokens.tokens);
    let tokens = contract
        .query_all_tokens(deps.as_ref(), env, Some(token_id1.clone()), Some(3))
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id2.clone()], tokens.tokens);

    // demeter gives random full (operator) power over her tokens
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: deps.api.addr_make("random").to_string(),
        expires: None,
    };
    let owner = message_info(&deps.api.addr_make("demeter"), &[]);
    let res = contract
        .execute(deps.as_mut(), mock_env(), owner, approve_all_msg)
        .unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "approve_all")
            .add_attribute("sender", deps.api.addr_make("demeter").as_ref())
            .add_attribute("operator", deps.api.addr_make("random").as_ref())
    );

    // random can now transfer
    let random = message_info(&deps.api.addr_make("random"), &[]);
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: deps.api.addr_make("person").to_string(),
        token_id: token_id1,
    };
    contract
        .execute(deps.as_mut(), mock_env(), random.clone(), transfer_msg)
        .unwrap();

    // random can now send
    let inner_msg = WasmMsg::Execute {
        contract_addr: "another_contract".into(),
        msg: to_json_binary("You now also have the growing power").unwrap(),
        funds: vec![],
    };
    let msg: CosmosMsg = CosmosMsg::Wasm(inner_msg);

    let send_msg = Cw721ExecuteMsg::SendNft {
        contract: deps.api.addr_make("another_contract").to_string(),
        token_id: token_id2,
        msg: to_json_binary(&msg).unwrap(),
    };
    contract
        .execute(deps.as_mut(), mock_env(), random, send_msg)
        .unwrap();

    // Approve_all, revoke_all, and check for empty, to test revoke_all
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: deps.api.addr_make("operator").to_string(),
        expires: None,
    };
    // person is now the owner of the tokens
    let owner = message_info(&deps.api.addr_make("person"), &[]);
    contract
        .execute(deps.as_mut(), mock_env(), owner, approve_all_msg)
        .unwrap();

    // query for operator should return approval
    let res = contract
        .query_operator(
            deps.as_ref(),
            mock_env(),
            deps.api.addr_make("person").to_string(),
            deps.api.addr_make("operator").to_string(),
            true,
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorResponse {
            approval: Approval {
                spender: deps.api.addr_make("operator"),
                expires: Expiration::Never {}
            }
        }
    );

    // query for other should throw error
    let res = contract.query_operator(
        deps.as_ref(),
        mock_env(),
        deps.api.addr_make("person").to_string(),
        deps.api.addr_make("other").to_string(),
        true,
    );
    match res {
        Err(StdError::NotFound { kind, .. }) => assert_eq!(kind, "Approval not found"),
        _ => panic!("Unexpected error"),
    }

    let res = contract
        .query_operators(
            deps.as_ref(),
            mock_env(),
            deps.api.addr_make("person").to_string(),
            true,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![Approval {
                spender: deps.api.addr_make("operator"),
                expires: Expiration::Never {}
            }]
        }
    );

    // second approval
    let buddy_expires = Expiration::AtHeight(1234567);
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: deps.api.addr_make("buddy").to_string(),
        expires: Some(buddy_expires),
    };
    let owner = message_info(&deps.api.addr_make("person"), &[]);
    contract
        .execute(deps.as_mut(), mock_env(), owner.clone(), approve_all_msg)
        .unwrap();

    // and paginate queries
    let res = contract
        .query_operators(
            deps.as_ref(),
            mock_env(),
            deps.api.addr_make("person").to_string(),
            true,
            None,
            Some(1),
        )
        .unwrap();
    // addr_make actually makes the buddy address *after* the operator one

    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![Approval {
                spender: deps.api.addr_make("buddy"),
                expires: buddy_expires,
            }]
        }
    );
    let res = contract
        .query_operators(
            deps.as_ref(),
            mock_env(),
            deps.api.addr_make("person").to_string(),
            true,
            Some(deps.api.addr_make("buddy").to_string()),
            Some(2),
        )
        .unwrap();

    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![Approval {
                spender: deps.api.addr_make("operator"),
                expires: Expiration::Never {}
            }]
        }
    );

    let revoke_all_msg = Cw721ExecuteMsg::RevokeAll {
        operator: deps.api.addr_make("operator").to_string(),
    };
    contract
        .execute(deps.as_mut(), mock_env(), owner, revoke_all_msg)
        .unwrap();

    // query for operator should return error
    let res = contract.query_operator(
        deps.as_ref(),
        mock_env(),
        deps.api.addr_make("person").to_string(),
        deps.api.addr_make("operator").to_string(),
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
            mock_env(),
            deps.api.addr_make("person").to_string(),
            false,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![Approval {
                spender: deps.api.addr_make("buddy"),
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
            late_env.clone(),
            deps.api.addr_make("person").to_string(),
            false,
            None,
            None,
        )
        .unwrap();
    assert_eq!(0, res.operators.len());

    // query operator should also return error
    let res = contract.query_operator(
        deps.as_ref(),
        late_env,
        deps.api.addr_make("person").to_string(),
        deps.api.addr_make("buddy").to_string(),
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
    // other than minter cant set
    let creator = deps.api.addr_make("creator");
    let minter = deps.api.addr_make(MINTER_ADDR);
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());
    let other = deps.api.addr_make("other");
    // other cant set
    let foo = deps.api.addr_make("foo");
    let err = contract
        .set_withdraw_address(deps.as_mut(), &other, foo.to_string())
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // minter can set
    contract
        .set_withdraw_address(
            deps.as_mut(),
            &minter,
            "foo".to_string(),
        )
        .unwrap();

    let withdraw_address = contract
        .config
        .withdraw_address
        .load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(withdraw_address, foo.to_string())
}

#[test]
fn test_remove_withdraw_address() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make(CREATOR);
    let minter = deps.api.addr_make(MINTER_ADDR);
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());
    let other = deps.api.addr_make("other");
    let foo = deps.api.addr_make("foo");

    // other than creator cant remove
    let err = contract
        .remove_withdraw_address(deps.as_mut().storage, &other)
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::Ownership(OwnershipError::NotOwner));

    // no withdraw address set yet
    let err = contract
        .remove_withdraw_address(deps.as_mut().storage, &minter)
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::NoWithdrawAddress {});

    // set and remove
    contract
        .set_withdraw_address(
            deps.as_mut(),
            &minter,
            foo.to_string(),
        )
        .unwrap();
    contract
        .remove_withdraw_address(deps.as_mut().storage, &minter)
        .unwrap();
    assert!(!contract
        .config
        .withdraw_address
        .exists(deps.as_ref().storage));

    // test that we can set again
    contract
        .set_withdraw_address(deps.as_mut(), &minter, foo.to_string())
        .unwrap();
    let withdraw_address = contract
        .config
        .withdraw_address
        .load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(withdraw_address, foo.to_string())
}

#[test]
fn test_withdraw_funds() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make("creator");
    let minter = deps.api.addr_make(MINTER_ADDR);
    let foo = deps.api.addr_make("foo");
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());

    // no withdraw address set
    let err = contract
        .withdraw_funds(deps.as_mut().storage, &Coin::new(100u32, "uark"))
        .unwrap_err();
    assert_eq!(err, Cw721ContractError::NoWithdrawAddress {});

    // set and withdraw by non-owner
    contract
        .set_withdraw_address(deps.as_mut(), &minter, foo.to_string())
        .unwrap();
    contract
        .withdraw_funds(deps.as_mut().storage, &Coin::new(100u32, "uark"))
        .unwrap();
}

#[test]
fn query_tokens_by_owner() {
    let mut deps = mock_dependencies();
    let creator = deps.api.addr_make(CREATOR);
    let minter = deps.api.addr_make(MINTER_ADDR);
    let contract = setup_contract(deps.as_mut(), creator, minter.clone());

    let minter = message_info(&minter, &[]);

    // Mint a couple tokens (from the same owner)
    let token_id1 = "grow1".to_string();
    let demeter = deps.api.addr_make("demeter");
    let token_id2 = "grow2".to_string();
    let ceres = deps.api.addr_make("ceres");
    let token_id3 = "sing".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id1.clone(),
        owner: demeter.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), mock_env(), minter.clone(), mint_msg)
        .unwrap();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id2.clone(),
        owner: ceres.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), mock_env(), minter.clone(), mint_msg)
        .unwrap();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id3.clone(),
        owner: demeter.to_string(),
        token_uri: None,
        extension: None,
    };
    let env = mock_env();
    contract
        .execute(deps.as_mut(), env.clone(), minter, mint_msg)
        .unwrap();

    // get all tokens in order:
    let expected = vec![token_id1.clone(), token_id2.clone(), token_id3.clone()];
    let tokens = contract
        .query_all_tokens(deps.as_ref(), env.clone(), None, None)
        .unwrap();
    assert_eq!(&expected, &tokens.tokens);
    // paginate
    let tokens = contract
        .query_all_tokens(deps.as_ref(), env.clone(), None, Some(2))
        .unwrap();
    assert_eq!(&expected[..2], &tokens.tokens[..]);
    let tokens = contract
        .query_all_tokens(deps.as_ref(), env.clone(), Some(expected[1].clone()), None)
        .unwrap();
    assert_eq!(&expected[2..], &tokens.tokens[..]);

    // get by owner
    let by_ceres = vec![token_id2];
    let by_demeter = vec![token_id1, token_id3];
    // all tokens by owner
    let tokens = contract
        .query_tokens(deps.as_ref(), env.clone(), demeter.to_string(), None, None)
        .unwrap();
    assert_eq!(&by_demeter, &tokens.tokens);
    let tokens = contract
        .query_tokens(deps.as_ref(), env.clone(), ceres.to_string(), None, None)
        .unwrap();
    assert_eq!(&by_ceres, &tokens.tokens);

    // paginate for demeter
    let tokens = contract
        .query_tokens(deps.as_ref(), env.clone(), demeter.to_string(), None, Some(1))
        .unwrap();
    assert_eq!(&by_demeter[..1], &tokens.tokens[..]);
    let tokens = contract
        .query_tokens(
            deps.as_ref(),
            env,
            demeter.to_string(),
            Some(by_demeter[0].clone()),
            Some(3),
        )
        .unwrap();
    assert_eq!(&by_demeter[1..], &tokens.tokens[..]);
}
