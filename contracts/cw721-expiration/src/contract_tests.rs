use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};

use cosmwasm_std::{
    from_json, to_json_binary, Addr, CosmosMsg, DepsMut, MessageInfo, Response, StdError, WasmMsg,
};

use cw721::error::Cw721ContractError;
use cw721::msg::CollectionInfoAndExtensionResponse;
use cw721::msg::{
    ApprovalResponse, Cw721ExecuteMsg, NftInfoResponse, OperatorResponse, OperatorsResponse,
    OwnerOfResponse, TokensResponse,
};
use cw721::receiver::Cw721ReceiveMsg;
use cw721::state::{CREATOR, MINTER};
use cw721::{traits::Cw721Query, Approval, Expiration};
use cw_ownable::{Action, Ownership, OwnershipError};

use crate::state::DefaultCw721ExpirationContract;
use crate::{
    error::ContractError, msg::InstantiateMsg, msg::QueryMsg, DefaultOptionalNftExtension,
};
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

const CONTRACT_NAME: &str = "Magic Power";
const SYMBOL: &str = "MGK";

fn setup_contract(
    deps: DepsMut<'_>,
    expiration_days: u16,
    creator: &Addr,
    minter: &Addr,
) -> DefaultCw721ExpirationContract<'static> {
    let contract = DefaultCw721ExpirationContract::default();
    let msg = InstantiateMsg {
        expiration_days,
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        collection_info_extension: None,
        minter: Some(minter.to_string()),
        creator: Some(creator.to_string()),
        withdraw_address: None,
    };
    let info = message_info(creator, &[]);
    let res = contract.instantiate(deps, mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    contract
}

#[test]
fn proper_instantiation() {
    let mut deps = mock_dependencies();
    let contract = DefaultCw721ExpirationContract::default();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let msg = InstantiateMsg {
        expiration_days: 1,
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
        .instantiate(deps.as_mut(), env.clone(), info, msg)
        .unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let minter_ownership = MINTER.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(minter), minter_ownership.owner);
    let creator_ownership = CREATOR.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(creator.clone()), creator_ownership.owner);
    let collection_info = contract
        .base_contract
        .query_collection_info_and_extension(deps.as_ref())
        .unwrap();
    assert_eq!(
        collection_info,
        CollectionInfoAndExtensionResponse {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            extension: None,
            updated_at: env.block.time,
        }
    );

    let withdraw_address = contract
        .base_contract
        .config
        .withdraw_address
        .may_load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(Some(creator.to_string()), withdraw_address);

    let count = contract
        .base_contract
        .query_num_tokens(deps.as_ref().storage)
        .unwrap();
    assert_eq!(0, count.count);

    // list the token_ids
    let tokens = contract
        .query_all_tokens_include_expired_nft(deps.as_ref(), mock_env(), None, None, false)
        .unwrap();
    assert_eq!(0, tokens.tokens.len());
}

#[test]
fn proper_instantiation_with_collection_info() {
    let mut deps = mock_dependencies();
    let contract = DefaultCw721ExpirationContract::default();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let msg = InstantiateMsg {
        expiration_days: 1,
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
        .instantiate(deps.as_mut(), env.clone(), info, msg)
        .unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let minter_ownership = MINTER.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(minter), minter_ownership.owner);
    let creator_ownership = CREATOR.get_ownership(deps.as_ref().storage).unwrap();
    assert_eq!(Some(creator.clone()), creator_ownership.owner);
    let collection_info = contract
        .base_contract
        .query_collection_info_and_extension(deps.as_ref())
        .unwrap();
    assert_eq!(
        collection_info,
        CollectionInfoAndExtensionResponse {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            extension: None,
            updated_at: env.block.time,
        }
    );

    let withdraw_address = contract
        .base_contract
        .config
        .withdraw_address
        .may_load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(Some(creator.to_string()), withdraw_address);

    let count = contract
        .base_contract
        .query_num_tokens(deps.as_ref().storage)
        .unwrap();
    assert_eq!(0, count.count);

    // list the token_ids
    let tokens = contract
        .query_all_tokens_include_expired_nft(deps.as_ref(), mock_env(), None, None, false)
        .unwrap();
    assert_eq!(0, tokens.tokens.len());
}

#[test]
fn test_mint() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);

    let token_id = "atomize".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/atomize".to_string();
    let owner = addrs.addr("medusa");
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: Some(token_uri.clone()),
        extension: None,
    };

    // random cannot mint
    let random = addrs.info("random");
    let env = mock_env();
    let err = contract
        .execute(deps.as_mut(), env.clone(), random, mint_msg.clone())
        .unwrap_err();
    assert_eq!(err, ContractError::Cw721(Cw721ContractError::NotMinter {}));

    // minter can mint
    let allowed = addrs.info("minter");
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed, mint_msg)
        .unwrap();

    // ensure num tokens increases
    let count = contract
        .base_contract
        .query_num_tokens(deps.as_ref().storage)
        .unwrap();
    assert_eq!(1, count.count);

    // unknown nft returns error
    let _ = contract
        .query_nft_info_include_expired_nft(deps.as_ref(), mock_env(), "unknown".to_string(), false)
        .unwrap_err();

    // this nft info is correct
    let info = contract
        .query_nft_info_include_expired_nft(deps.as_ref(), mock_env(), token_id.clone(), false)
        .unwrap();
    assert_eq!(
        info,
        NftInfoResponse::<DefaultOptionalNftExtension> {
            token_uri: Some(token_uri),
            extension: None,
        }
    );

    // owner info is correct
    let owner_res = contract
        .query_owner_of_include_expired_nft(
            deps.as_ref(),
            mock_env(),
            token_id.clone(),
            true,
            false,
        )
        .unwrap();
    assert_eq!(
        owner_res,
        OwnerOfResponse {
            owner: owner.to_string(),
            approvals: vec![],
        }
    );

    // assert mint timestamp is set
    let mint_timestamp = contract
        .mint_timestamps
        .load(deps.as_ref().storage, token_id.as_str())
        .unwrap();
    assert_eq!(mint_timestamp, mock_env().block.time);

    // Cannot mint same token_id again
    let mint_msg2 = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: addrs.addr("hercules").to_string(),
        token_uri: None,
        extension: None,
    };

    let allowed = addrs.info("minter");
    let err = contract
        .execute(deps.as_mut(), mock_env(), allowed, mint_msg2)
        .unwrap_err();
    assert_eq!(err, ContractError::Cw721(Cw721ContractError::Claimed {}));

    // list the token_ids
    let tokens = contract
        .query_all_tokens_include_expired_nft(deps.as_ref(), mock_env(), None, None, false)
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id], tokens.tokens);
}

#[test]
fn test_update_minter() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);

    let token_id = "petrify".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();
    let medusa = addrs.addr("medusa");
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id,
        owner: medusa.to_string(),
        token_uri: Some(token_uri.clone()),
        extension: None,
    };

    // Minter can mint
    let minter_info = addrs.info("minter");
    let _ = contract
        .execute(deps.as_mut(), mock_env(), minter_info.clone(), mint_msg)
        .unwrap();

    // Update the owner to "random". The new owner should be able to
    // mint new tokens, the old one should not.
    let random = addrs.addr("random");
    contract
        .execute(
            deps.as_mut(),
            mock_env(),
            minter_info.clone(),
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
            .query(deps.as_ref(), mock_env(), QueryMsg::GetMinterOwnership {})
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
    let random_info = addrs.info("random");
    contract
        .execute(
            deps.as_mut(),
            mock_env(),
            random_info.clone(),
            Cw721ExecuteMsg::UpdateMinterOwnership(Action::AcceptOwnership),
        )
        .unwrap();

    // Minter changes after ownership transfer is accepted.
    let minter_ownership: Ownership<Addr> = from_json(
        contract
            .query(deps.as_ref(), mock_env(), QueryMsg::GetMinterOwnership {})
            .unwrap(),
    )
    .unwrap();
    assert_eq!(minter_ownership.owner, Some(random));

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: "randoms_token".to_string(),
        owner: addrs.addr("medusa").to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    // Old owner can not mint.
    let err: ContractError = contract
        .execute(deps.as_mut(), mock_env(), minter_info, mint_msg.clone())
        .unwrap_err();
    assert_eq!(err, ContractError::Cw721(Cw721ContractError::NotMinter {}));

    // New owner can mint.
    let _ = contract
        .execute(deps.as_mut(), mock_env(), random_info, mint_msg)
        .unwrap();
}

#[test]
fn test_burn() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);

    let token_id = "petrify".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: minter.to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    let burn_msg = Cw721ExecuteMsg::Burn {
        token_id: token_id.clone(),
    };

    // mint some NFT
    let mut env = mock_env();
    let minter_info = addrs.info("minter");
    let _ = contract
        .execute(
            deps.as_mut(),
            env.clone(),
            minter_info.clone(),
            mint_msg.clone(),
        )
        .unwrap();

    // random not allowed to burn
    let random = addrs.info("random");
    let err = contract
        .execute(deps.as_mut(), env.clone(), random, burn_msg.clone())
        .unwrap_err();

    assert_eq!(
        err,
        ContractError::Cw721(Cw721ContractError::Ownership(OwnershipError::NotOwner))
    );

    let _ = contract
        .execute(
            deps.as_mut(),
            env.clone(),
            minter_info.clone(),
            burn_msg.clone(),
        )
        .unwrap();

    // ensure num tokens decreases
    let count = contract
        .base_contract
        .query_num_tokens(deps.as_ref().storage)
        .unwrap();
    assert_eq!(0, count.count);

    // trying to get nft returns error
    let _ = contract
        .query_nft_info_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            "petrify".to_string(),
            false,
        )
        .unwrap_err();

    // list the token_ids
    let tokens = contract
        .query_all_tokens_include_expired_nft(deps.as_ref(), env.clone(), None, None, false)
        .unwrap();
    assert!(tokens.tokens.is_empty());

    // assert invalid nft throws error
    // - mint again
    contract
        .execute(deps.as_mut(), env.clone(), minter_info.clone(), mint_msg)
        .unwrap();
    // - burn
    let mint_date = env.block.time;
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let error = contract
        .execute(deps.as_mut(), env, minter_info, burn_msg)
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id,
            mint_date,
            expiration
        }
    );
}

#[test]
fn test_transfer_nft() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);

    // Mint a token
    let token_id = "melt".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/melt".to_string();

    let owner = addrs.addr("owner");
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    let mut env = mock_env();
    let minter_info = addrs.info("minter");
    contract
        .execute(deps.as_mut(), env.clone(), minter_info, mint_msg)
        .unwrap();

    // random cannot transfer
    let random = addrs.addr("random");
    let random_info = addrs.info("random");
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: random.to_string(),
        token_id: token_id.clone(),
    };

    let err = contract
        .execute(deps.as_mut(), env.clone(), random_info, transfer_msg)
        .unwrap_err();
    assert_eq!(
        err,
        ContractError::Cw721(Cw721ContractError::Ownership(OwnershipError::NotOwner))
    );

    // owner can
    let owner_info = addrs.info("owner");
    let new_owner = addrs.addr("random");
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: new_owner.to_string(),
        token_id: token_id.clone(),
    };

    let res = contract
        .execute(
            deps.as_mut(),
            env.clone(),
            owner_info.clone(),
            transfer_msg.clone(),
        )
        .unwrap();

    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", owner.to_string())
            .add_attribute("recipient", new_owner.to_string())
            .add_attribute("token_id", token_id.clone())
    );

    // assert invalid nft throws error
    let mint_date = env.block.time;
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let error = contract
        .execute(deps.as_mut(), env, owner_info, transfer_msg)
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id,
            mint_date,
            expiration
        }
    );
}

#[test]
fn test_send_nft() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);

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

    let mut env = mock_env();
    let minter = addrs.info("minter");
    contract
        .execute(deps.as_mut(), env.clone(), minter, mint_msg)
        .unwrap();

    let msg = to_json_binary("You now have the melting power").unwrap();
    let target = addrs.addr("another_contract");
    let send_msg = Cw721ExecuteMsg::SendNft {
        contract: target.to_string(),
        token_id: token_id.clone(),
        msg: msg.clone(),
    };

    let random_info = addrs.info("random");
    let err = contract
        .execute(
            deps.as_mut(),
            env.clone(),
            random_info.clone(),
            send_msg.clone(),
        )
        .unwrap_err();
    assert_eq!(
        err,
        ContractError::Cw721(Cw721ContractError::Ownership(OwnershipError::NotOwner))
    );

    // but owner can
    let venus_info: MessageInfo = addrs.info("venus");
    let res = contract
        .execute(
            deps.as_mut(),
            env.clone(),
            venus_info.clone(),
            send_msg.clone(),
        )
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
            assert_eq!(contract_addr, &target.to_string())
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
            .add_attribute("recipient", target.to_string())
            .add_attribute("token_id", token_id.clone())
    );

    // assert invalid nft throws error
    let mint_date = env.block.time;
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let error = contract
        .execute(deps.as_mut(), env, random_info, send_msg)
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id,
            mint_date,
            expiration
        }
    );
}

#[test]
fn test_approve_revoke() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);

    // Mint a token
    let token_id = "grow".to_string();
    let token_uri = "https://www.merriam-webster.com/dictionary/grow".to_string();
    let demeter = addrs.addr("demeter");
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: demeter.to_string(),
        token_uri: Some(token_uri),
        extension: None,
    };

    let mut env = mock_env();
    let minter = addrs.info("minter");
    contract
        .execute(deps.as_mut(), env.clone(), minter, mint_msg)
        .unwrap();

    // token owner shows in approval query
    let res = contract
        .query_approval_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            token_id.clone(),
            demeter.to_string(),
            false,
            false,
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovalResponse {
            approval: Approval {
                spender: demeter.clone(),
                expires: Expiration::Never {}
            }
        }
    );

    // Give random transferring power
    let random = addrs.addr("random");
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: random.to_string(),
        token_id: token_id.clone(),
        expires: None,
    };
    let owner_info = addrs.info("demeter");
    let res = contract
        .execute(deps.as_mut(), env.clone(), owner_info, approve_msg)
        .unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "approve")
            .add_attribute("sender", demeter.to_string())
            .add_attribute("spender", random.to_string())
            .add_attribute("token_id", token_id.clone())
    );

    // test approval query
    let res = contract
        .query_approval_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            token_id.clone(),
            random.to_string(),
            true,
            false,
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovalResponse {
            approval: Approval {
                spender: random.clone(),
                expires: Expiration::Never {}
            }
        }
    );

    // random can now transfer
    let random_info = addrs.info("random");
    let person = addrs.addr("person");
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: person.to_string(),
        token_id: token_id.clone(),
    };
    contract
        .execute(deps.as_mut(), env.clone(), random_info, transfer_msg)
        .unwrap();

    // Approvals are removed / cleared
    let query_msg = QueryMsg::OwnerOf {
        token_id: token_id.clone(),
        include_expired: None,
        include_expired_nft: None,
    };
    let res: OwnerOfResponse = from_json(
        contract
            .query(deps.as_ref(), env.clone(), query_msg.clone())
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
        .execute(
            deps.as_mut(),
            env.clone(),
            owner_info.clone(),
            approve_msg.clone(),
        )
        .unwrap();

    let revoke_msg = Cw721ExecuteMsg::Revoke {
        spender: random.to_string(),
        token_id: token_id.clone(),
    };
    contract
        .execute(
            deps.as_mut(),
            env.clone(),
            owner_info.clone(),
            revoke_msg.clone(),
        )
        .unwrap();

    // Approvals are now removed / cleared
    let res: OwnerOfResponse = from_json(
        contract
            .query(deps.as_ref(), env.clone(), query_msg)
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

    // assert approval of invalid nft throws error
    let mint_date = env.block.time;
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let error = contract
        .execute(deps.as_mut(), env.clone(), owner_info.clone(), approve_msg)
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id: token_id.clone(),
            mint_date,
            expiration
        }
    );

    // assert revoke of invalid nft throws error
    let error = contract
        .execute(deps.as_mut(), env, owner_info, revoke_msg)
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id,
            mint_date,
            expiration
        }
    );
}

#[test]
fn test_approve_all_revoke_all() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);

    // Mint a couple tokens (from the same owner)
    let token_id1 = "grow1".to_string();
    let token_uri1 = "https://www.merriam-webster.com/dictionary/grow1".to_string();

    let token_id2 = "grow2".to_string();
    let token_uri2 = "https://www.merriam-webster.com/dictionary/grow2".to_string();
    let demeter = addrs.addr("demeter");
    let mint_msg1 = Cw721ExecuteMsg::Mint {
        token_id: token_id1.clone(),
        owner: demeter.to_string(),
        token_uri: Some(token_uri1),
        extension: None,
    };

    let minter = addrs.info("minter");
    contract
        .execute(deps.as_mut(), mock_env(), minter.clone(), mint_msg1)
        .unwrap();

    let mint_msg2 = Cw721ExecuteMsg::Mint {
        token_id: token_id2.clone(),
        owner: demeter.to_string(),
        token_uri: Some(token_uri2),
        extension: None,
    };

    contract
        .execute(deps.as_mut(), mock_env(), minter, mint_msg2)
        .unwrap();

    // paginate the token_ids
    let tokens = contract
        .query_all_tokens_include_expired_nft(deps.as_ref(), mock_env(), None, Some(1), false)
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id1.clone()], tokens.tokens);
    let tokens = contract
        .query_all_tokens_include_expired_nft(
            deps.as_ref(),
            mock_env(),
            Some(token_id1.clone()),
            Some(3),
            false,
        )
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id2.clone()], tokens.tokens);

    // demeter gives random full (operator) power over her tokens
    let random = addrs.addr("random");
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: random.to_string(),
        expires: None,
    };
    let owner = addrs.info("demeter");
    let res = contract
        .execute(deps.as_mut(), mock_env(), owner, approve_all_msg)
        .unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "approve_all")
            .add_attribute("sender", demeter.to_string())
            .add_attribute("operator", random.to_string())
    );

    // random can now transfer
    let random_info = addrs.info("random");
    let person = addrs.addr("person");
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: person.to_string(),
        token_id: token_id1,
    };
    contract
        .execute(deps.as_mut(), mock_env(), random_info.clone(), transfer_msg)
        .unwrap();
    let other_contract = addrs.addr("other_contract");
    // random can now send
    let inner_msg = WasmMsg::Execute {
        contract_addr: other_contract.to_string(),
        msg: to_json_binary("You now also have the growing power").unwrap(),
        funds: vec![],
    };
    let msg: CosmosMsg = CosmosMsg::Wasm(inner_msg);

    let send_msg = Cw721ExecuteMsg::SendNft {
        contract: other_contract.to_string(),
        token_id: token_id2,
        msg: to_json_binary(&msg).unwrap(),
    };
    contract
        .execute(deps.as_mut(), mock_env(), random_info, send_msg)
        .unwrap();

    // Approve_all, revoke_all, and check for empty, to test revoke_all
    let operator = addrs.addr("operator");
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: operator.to_string(),
        expires: None,
    };
    // person is now the owner of the tokens
    let owner = addrs.info("person");
    contract
        .execute(deps.as_mut(), mock_env(), owner, approve_all_msg)
        .unwrap();

    // query for operator should return approval
    let res = contract
        .base_contract
        .query_operator(
            deps.as_ref(),
            &mock_env(),
            person.to_string(),
            operator.to_string(),
            true,
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorResponse {
            approval: Approval {
                spender: operator.clone(),
                expires: Expiration::Never {}
            }
        }
    );

    // query for other should throw error
    let other = addrs.addr("other");
    let res = contract.base_contract.query_operator(
        deps.as_ref(),
        &mock_env(),
        person.to_string(),
        other.to_string(),
        true,
    );
    match res {
        Err(StdError::NotFound { kind, .. }) => assert_eq!(kind, "Approval not found"),
        _ => panic!("Unexpected error"),
    }

    let res = contract
        .base_contract
        .query_operators(
            deps.as_ref(),
            &mock_env(),
            person.to_string(),
            true,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![cw721::Approval {
                spender: operator.clone(),
                expires: Expiration::Never {}
            }]
        }
    );

    // second approval
    let buddy_expires = Expiration::AtHeight(1234567);
    let buddy = addrs.addr("buddy");
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: buddy.to_string(),
        expires: Some(buddy_expires),
    };
    let owner_info = addrs.info("person");
    contract
        .execute(
            deps.as_mut(),
            mock_env(),
            owner_info.clone(),
            approve_all_msg,
        )
        .unwrap();

    // and paginate queries
    let res = contract
        .base_contract
        .query_operators(
            deps.as_ref(),
            &mock_env(),
            person.to_string(),
            true,
            Some(operator.to_string()),
            Some(1),
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![cw721::Approval {
                spender: buddy.clone(),
                expires: buddy_expires,
            }]
        }
    );
    let res = contract
        .base_contract
        .query_operators(
            deps.as_ref(),
            &mock_env(),
            person.to_string(),
            true,
            None,
            Some(2),
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![
                cw721::Approval {
                    spender: operator.clone(),
                    expires: Expiration::Never {}
                },
                cw721::Approval {
                    spender: buddy.clone(),
                    expires: buddy_expires,
                }
            ]
        }
    );

    let revoke_all_msg = Cw721ExecuteMsg::RevokeAll {
        operator: operator.to_string(),
    };
    contract
        .execute(deps.as_mut(), mock_env(), owner_info, revoke_all_msg)
        .unwrap();

    // query for operator should return error
    let res = contract.base_contract.query_operator(
        deps.as_ref(),
        &mock_env(),
        person.to_string(),
        operator.to_string(),
        true,
    );
    match res {
        Err(StdError::NotFound { kind, .. }) => assert_eq!(kind, "Approval not found"),
        _ => panic!("Unexpected error"),
    }

    // Approvals are removed / cleared without affecting others
    let res = contract
        .base_contract
        .query_operators(
            deps.as_ref(),
            &mock_env(),
            person.to_string(),
            false,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![cw721::Approval {
                spender: buddy.clone(),
                expires: buddy_expires,
            }]
        }
    );

    // ensure the filter works (nothing should be here
    let mut late_env = mock_env();
    late_env.block.height = 1234568; //expired
    let res = contract
        .base_contract
        .query_operators(
            deps.as_ref(),
            &late_env,
            person.to_string(),
            false,
            None,
            None,
        )
        .unwrap();
    assert_eq!(0, res.operators.len());

    // query operator should also return error
    let res = contract.base_contract.query_operator(
        deps.as_ref(),
        &late_env,
        person.to_string(),
        buddy.to_string(),
        false,
    );

    match res {
        Err(StdError::NotFound { kind, .. }) => assert_eq!(kind, "Approval not found"),
        _ => panic!("Unexpected error"),
    }
}

#[test]
fn test_tokens_by_owner() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);
    let minter_info = addrs.info("minter");

    // Mint a couple tokens (from the same owner)
    let token_id1 = "grow1".to_string();
    let demeter = addrs.addr("demeter");
    let token_id2 = "grow2".to_string();
    let ceres = addrs.addr("ceres");
    let token_id3 = "sing".to_string();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id1.clone(),
        owner: demeter.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), mock_env(), minter_info.clone(), mint_msg)
        .unwrap();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id2.clone(),
        owner: ceres.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), mock_env(), minter_info.clone(), mint_msg)
        .unwrap();

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id3.clone(),
        owner: demeter.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), mock_env(), minter_info, mint_msg)
        .unwrap();

    // get all tokens in order:
    let expected = vec![token_id1.clone(), token_id2.clone(), token_id3.clone()];
    let tokens = contract
        .query_all_tokens_include_expired_nft(deps.as_ref(), mock_env(), None, None, false)
        .unwrap();
    assert_eq!(&expected, &tokens.tokens);
    // paginate
    let tokens = contract
        .query_all_tokens_include_expired_nft(deps.as_ref(), mock_env(), None, Some(2), false)
        .unwrap();
    assert_eq!(&expected[..2], &tokens.tokens[..]);
    let tokens = contract
        .query_all_tokens_include_expired_nft(
            deps.as_ref(),
            mock_env(),
            Some(expected[1].clone()),
            None,
            false,
        )
        .unwrap();
    assert_eq!(&expected[2..], &tokens.tokens[..]);

    // get by owner
    let by_ceres = vec![token_id2];
    let by_demeter = vec![token_id1, token_id3];
    // all tokens by owner
    let tokens = contract
        .query_tokens_include_expired_nft(
            deps.as_ref(),
            mock_env(),
            demeter.to_string(),
            None,
            None,
            false,
        )
        .unwrap();
    assert_eq!(&by_demeter, &tokens.tokens);
    let tokens = contract
        .query_tokens_include_expired_nft(
            deps.as_ref(),
            mock_env(),
            ceres.to_string(),
            None,
            None,
            false,
        )
        .unwrap();
    assert_eq!(&by_ceres, &tokens.tokens);

    // paginate for demeter
    let tokens = contract
        .query_tokens_include_expired_nft(
            deps.as_ref(),
            mock_env(),
            demeter.to_string(),
            None,
            Some(1),
            false,
        )
        .unwrap();
    assert_eq!(&by_demeter[..1], &tokens.tokens[..]);
    let tokens = contract
        .query_tokens_include_expired_nft(
            deps.as_ref(),
            mock_env(),
            demeter.to_string(),
            Some(by_demeter[0].clone()),
            Some(3),
            false,
        )
        .unwrap();
    assert_eq!(&by_demeter[1..], &tokens.tokens[..]);
}

#[test]
fn test_nft_info() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);
    let minter = addrs.info("minter");

    let token_id = "grow1".to_string();
    let owner = addrs.addr("ark");

    let mut env = mock_env();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), env.clone(), minter, mint_msg)
        .unwrap();

    // assert valid nft is returned
    contract
        .query_nft_info_include_expired_nft(deps.as_ref(), env.clone(), token_id.clone(), false)
        .unwrap();

    // assert invalid nft throws error
    let mint_date = env.block.time;
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let error = contract
        .query_nft_info_include_expired_nft(deps.as_ref(), env, token_id.clone(), false)
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id,
            mint_date,
            expiration
        }
    );
}

#[test]
fn test_all_nft_info() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);
    let minter_info = addrs.info("minter");

    let token_id = "grow1".to_string();
    let owner = addrs.addr("ark");

    let mut env = mock_env();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), env.clone(), minter_info, mint_msg)
        .unwrap();

    // assert valid nft is returned
    contract
        .query_all_nft_info_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            token_id.clone(),
            false,
            false,
        )
        .unwrap();

    // assert invalid nft throws error
    let mint_date = env.block.time;
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let error = contract
        .query_all_nft_info_include_expired_nft(deps.as_ref(), env, token_id.clone(), false, false)
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id,
            mint_date,
            expiration
        }
    );
}

#[test]
fn test_owner_of() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);
    let minter_info = addrs.info("minter");

    let token_id = "grow1".to_string();
    let owner = addrs.addr("ark");

    let mut env = mock_env();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), env.clone(), minter_info, mint_msg)
        .unwrap();

    // assert valid nft is returned
    contract
        .query_owner_of_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            token_id.clone(),
            false,
            false,
        )
        .unwrap();

    // assert invalid nft throws error
    let mint_date = env.block.time;
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let error = contract
        .query_owner_of_include_expired_nft(deps.as_ref(), env, token_id.clone(), false, false)
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id,
            mint_date,
            expiration
        }
    );
}

#[test]
fn test_approval() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);
    let minter_info = addrs.info("minter");

    let token_id = "grow1".to_string();
    let owner = addrs.addr("ark");

    let mut env = mock_env();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), env.clone(), minter_info, mint_msg)
        .unwrap();

    // assert valid nft is returned
    contract
        .query_approval_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            token_id.clone(),
            owner.to_string(),
            false,
            false,
        )
        .unwrap();

    // assert invalid nft throws error
    let mint_date = env.block.time;
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let error = contract
        .query_approval_include_expired_nft(
            deps.as_ref(),
            env,
            token_id.clone(),
            owner.to_string(),
            false,
            false,
        )
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id,
            mint_date,
            expiration
        }
    );
}

#[test]
fn test_approvals() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);
    let minter_info = addrs.info("minter");

    let token_id = "grow1".to_string();
    let owner = addrs.addr("ark");

    let mut env = mock_env();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), env.clone(), minter_info, mint_msg)
        .unwrap();

    // assert valid nft is returned
    contract
        .query_approvals_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            token_id.clone(),
            false,
            false,
        )
        .unwrap();

    // assert invalid nft throws error
    let mint_date = env.block.time;
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let error = contract
        .query_approvals_include_expired_nft(deps.as_ref(), env, token_id.clone(), false, false)
        .unwrap_err();
    assert_eq!(
        error,
        ContractError::NftExpired {
            token_id,
            mint_date,
            expiration
        }
    );
}

#[test]
fn test_tokens() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);
    let minter_info = addrs.info("minter");

    let token_id = "grow1".to_string();
    let owner = addrs.addr("ark");

    let mut env = mock_env();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), env.clone(), minter_info, mint_msg)
        .unwrap();

    // assert valid nft is returned
    contract
        .query_tokens_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            owner.to_string(),
            None,
            None,
            false,
        )
        .unwrap();

    // assert invalid nft is not returned
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let tokens = contract
        .query_tokens_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            owner.to_string(),
            None,
            None,
            false,
        )
        .unwrap();
    assert_eq!(tokens, TokensResponse { tokens: vec![] });

    // assert invalid nft is returned
    let tokens = contract
        .query_tokens_include_expired_nft(deps.as_ref(), env, owner.to_string(), None, None, true)
        .unwrap();
    assert_eq!(
        tokens,
        TokensResponse {
            tokens: [token_id].to_vec()
        }
    );
}

#[test]
fn test_all_tokens() {
    let mut deps = mock_dependencies();
    let mut addrs = MockAddrFactory::new(deps.api);
    let creator = addrs.addr("creator");
    let minter = addrs.addr("minter");
    let contract = setup_contract(deps.as_mut(), 1, &creator, &minter);
    let minter_info = addrs.info("minter");

    let token_id = "grow1".to_string();
    let owner = addrs.addr("ark");

    let mut env = mock_env();
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: None,
        extension: None,
    };
    contract
        .execute(deps.as_mut(), env.clone(), minter_info, mint_msg)
        .unwrap();

    // assert valid nft is returned
    contract
        .query_all_tokens_include_expired_nft(deps.as_ref(), env.clone(), None, None, false)
        .unwrap();

    // assert invalid nft is not returned
    let expiration = env.block.time.plus_days(1);
    env.block.time = expiration;
    let tokens = contract
        .query_tokens_include_expired_nft(
            deps.as_ref(),
            env.clone(),
            owner.to_string(),
            None,
            None,
            false,
        )
        .unwrap();
    assert_eq!(tokens, TokensResponse { tokens: vec![] });

    // assert invalid nft is returned
    let tokens = contract
        .query_all_tokens_include_expired_nft(deps.as_ref(), env, None, None, true)
        .unwrap();
    assert_eq!(
        tokens,
        TokensResponse {
            tokens: [token_id].to_vec()
        }
    );
}
