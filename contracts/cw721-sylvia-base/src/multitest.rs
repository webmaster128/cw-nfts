use cosmwasm_std::{to_json_binary, Addr, Empty, StdError};
use cw721::{
    Approval, ApprovalResponse, ContractInfoResponse, NftInfoResponse, NumTokensResponse,
    OperatorResponse, OperatorsResponse, OwnerOfResponse, TokensResponse,
};
use cw_multi_test::App as MtApp;
use cw_ownable::{Action, Expiration, OwnershipError};
use sylvia::multitest::App;

use crate::base::sv::test_utils::Cw721Interface;
use crate::responses::MinterResponse;
use crate::{
    contract::sv::multitest_utils::{CodeId, Cw721ContractProxy},
    contract::InstantiateMsgData,
    ContractError,
};

const CREATOR: &str = "creator";
const RANDOM: &str = "random";

pub struct TestCase<'a> {
    nft_contract: Cw721ContractProxy<'a, MtApp>,
}

impl TestCase<'_> {
    // Lifetimes with Sylvia are fun. Open to a better way of doing this
    pub fn new<'b>(app: &'b App<MtApp>) -> TestCase<'b> {
        let code_id = CodeId::store_code(app);

        TestCase::<'b> {
            nft_contract: code_id
                .instantiate(InstantiateMsgData {
                    name: "Sylvia".to_string(),
                    symbol: "SYLVIA".to_string(),
                    minter: CREATOR.to_string(),
                })
                .with_label("cw721-sylvia-base contract")
                .call(CREATOR)
                .unwrap(),
        }
    }
}

#[test]
fn test_instantiate() {
    let app = App::default();
    let code_id = CodeId::store_code(&app);

    let nft_contract = code_id
        .instantiate(InstantiateMsgData {
            name: "Sylvia".to_string(),
            symbol: "SYLVIA".to_string(),
            minter: CREATOR.to_string(),
        })
        .with_label("cw721-sylvia-base contract")
        .call(CREATOR)
        .unwrap();

    // Check contract metadata was set
    let contract_info = nft_contract
        .cw721_interface_proxy()
        .contract_info()
        .unwrap();
    assert_eq!(
        ContractInfoResponse {
            name: "Sylvia".to_string(),
            symbol: "SYLVIA".to_string()
        },
        contract_info
    );

    // Query minter
    let minter = nft_contract.minter().unwrap();
    assert_eq!(
        MinterResponse {
            minter: Some(CREATOR.to_string())
        },
        minter
    );

    // Check number of tokens is zero
    let count = nft_contract.cw721_interface_proxy().num_tokens().unwrap();
    assert_eq!(NumTokensResponse { count: 0 }, count);
}

#[test]
fn test_mint() {
    let app = App::default();
    let TestCase { nft_contract } = TestCase::new(&app);

    // Only minter / owner can mint
    let res = nft_contract
        .mint(
            "1".to_string(),
            RANDOM.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(RANDOM)
        .unwrap_err();
    assert_eq!(res, ContractError::Ownership(OwnershipError::NotOwner));

    // Minter / owner can mint an NFT
    nft_contract
        .mint(
            "1".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    // Check number of tokens is now one
    let count = nft_contract.cw721_interface_proxy().num_tokens().unwrap();
    assert_eq!(NumTokensResponse { count: 1 }, count);

    // Query token info
    let info = nft_contract
        .cw721_interface_proxy()
        .nft_info("1".to_string())
        .unwrap();
    assert_eq!(
        NftInfoResponse::<Empty> {
            token_uri: Some("https://example.com".to_string()),
            extension: Empty {}
        },
        info
    );

    // Creator mints an NFT for a random address
    nft_contract
        .mint(
            "2".to_string(),
            RANDOM.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    // Query new tokens by owner
    let tokens = nft_contract
        .cw721_interface_proxy()
        .tokens(CREATOR.to_string(), None, None)
        .unwrap();
    assert_eq!(
        TokensResponse {
            tokens: vec!["1".to_string()]
        },
        tokens
    );

    // Query all tokens
    let all_tokens = nft_contract
        .cw721_interface_proxy()
        .all_tokens(None, None)
        .unwrap();
    assert_eq!(
        TokensResponse {
            tokens: vec!["1".to_string(), "2".to_string()]
        },
        all_tokens
    );
}

#[test]
fn test_burn() {
    let app = App::default();
    let TestCase { nft_contract } = TestCase::new(&app);

    // Mint NFT
    nft_contract
        .mint(
            "1".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    // Check number of tokens is now one
    let count = nft_contract.cw721_interface_proxy().num_tokens().unwrap();
    assert_eq!(NumTokensResponse { count: 1 }, count);

    // Non minter / owner cannot burn
    nft_contract
        .cw721_interface_proxy()
        .burn("1".to_string())
        .call(RANDOM)
        .unwrap_err();

    // Only owner can burn
    nft_contract
        .cw721_interface_proxy()
        .burn("1".to_string())
        .call(CREATOR)
        .unwrap();

    // Token count has been reduced
    let count = nft_contract.cw721_interface_proxy().num_tokens().unwrap();
    assert_eq!(NumTokensResponse { count: 0 }, count);
}

#[test]
fn test_transfer() {
    let app = App::default();
    let TestCase { nft_contract } = TestCase::new(&app);

    // Mint NFT
    nft_contract
        .mint(
            "1".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    // Owned by creator
    let nft_ownership = nft_contract
        .cw721_interface_proxy()
        .owner_of("1".to_string(), false)
        .unwrap();
    assert_eq!(
        OwnerOfResponse {
            owner: CREATOR.to_string(),
            approvals: vec![]
        },
        nft_ownership
    );

    // Random address can't transfer
    let err = nft_contract
        .cw721_interface_proxy()
        .transfer_nft(RANDOM.to_string(), "1".to_string())
        .call(RANDOM)
        .unwrap_err();
    assert_eq!(ContractError::Ownership(OwnershipError::NotOwner), err);

    // Only owner can transfer
    nft_contract
        .cw721_interface_proxy()
        .transfer_nft(RANDOM.to_string(), "1".to_string())
        .call(CREATOR)
        .unwrap();

    // Ownership has changed
    let nft_ownership = nft_contract
        .cw721_interface_proxy()
        .owner_of("1".to_string(), false)
        .unwrap();
    assert_eq!(
        OwnerOfResponse {
            owner: RANDOM.to_string(),
            approvals: vec![]
        },
        nft_ownership
    );

    // Now that random is the owner, creator can't transfer it back
    // even though they are the minter.
    let err = nft_contract
        .cw721_interface_proxy()
        .transfer_nft(CREATOR.to_string(), "1".to_string())
        .call(CREATOR)
        .unwrap_err();
    assert_eq!(ContractError::Ownership(OwnershipError::NotOwner), err);
}

// NOTE: when a reciever test contract is implemented, we don't have to panic here.
#[should_panic(
    expected = "called `Result::unwrap()` on an `Err` value: Error executing WasmMsg:\n  sender: creator\n  Execute { contract_addr: \"contract0\", msg: {\"send_nft\":{\"contract\":\"contract0\",\"token_id\":\"1\",\"msg\":\"e30=\"}}, funds: [] }\n\nCaused by:\n    0: Error executing WasmMsg:\n         sender: contract0\n         Execute { contract_addr: \"contract0\", msg: {\"receive_nft\":{\"sender\":\"creator\",\"token_id\":\"1\",\"msg\":\"e30=\"}}, funds: [] }\n    1: Error parsing into type cw721_sylvia_base::contract::sv::ContractExecMsg: Unsupported message received: {\"receive_nft\":{\"msg\":\"e30=\",\"sender\":\"creator\",\"token_id\":\"1\"}}. Messages supported by this contract: approve, approve_all, burn, revoke, revoke_all, send_nft, transfer_nft, mint, update_ownership"
)]
#[test]
fn test_send() {
    let app = App::default();
    let TestCase { nft_contract } = TestCase::new(&app);

    // Mint NFT
    nft_contract
        .mint(
            "1".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    // Owned by creator
    let nft_ownership = nft_contract
        .cw721_interface_proxy()
        .owner_of("1".to_string(), false)
        .unwrap();
    assert_eq!(
        OwnerOfResponse {
            owner: CREATOR.to_string(),
            approvals: vec![]
        },
        nft_ownership
    );

    // Random address can't send
    let err = nft_contract
        .cw721_interface_proxy()
        .send_nft(
            nft_contract.contract_addr.clone().into_string(),
            "1".to_string(),
            to_json_binary(&Empty {}).unwrap(),
        )
        .call(RANDOM)
        .unwrap_err();
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner));

    // Creator can send
    // This will panic as the base contract does not implement RecieveNft
    nft_contract
        .cw721_interface_proxy()
        .send_nft(
            nft_contract.contract_addr.into_string(),
            "1".to_string(),
            to_json_binary(&Empty {}).unwrap(),
        )
        .call(CREATOR)
        .unwrap_err();
}

#[test]
fn test_update_minter() {
    let app = App::default();
    let TestCase { nft_contract } = TestCase::new(&app);

    // Minter Mints NFT
    nft_contract
        .mint(
            "1".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    // Update the owner to "random". The new owner should be able to
    // mint new tokens, the old one should not.
    nft_contract
        .update_ownership(Action::TransferOwnership {
            new_owner: RANDOM.to_string(),
            expiry: None,
        })
        .call(CREATOR)
        .unwrap();

    // Minter does not change until ownership transfer completes.
    let minter = nft_contract.minter().unwrap();
    assert_eq!(minter.minter, Some(CREATOR.to_string()));

    // Pending ownership transfer should be discoverable via query.
    let ownership = nft_contract.ownership().unwrap();

    assert_eq!(
        ownership,
        cw_ownable::Ownership::<Addr> {
            owner: Some(Addr::unchecked(CREATOR)),
            pending_owner: Some(Addr::unchecked(RANDOM)),
            pending_expiry: None,
        }
    );

    // Accept the ownership transfer.
    nft_contract
        .update_ownership(Action::AcceptOwnership)
        .call(RANDOM)
        .unwrap();

    // Minter changes after ownership transfer is accepted.
    let minter = nft_contract.minter().unwrap();
    assert_eq!(minter.minter, Some(RANDOM.to_string()));

    // Old owner can not mint.
    let err = nft_contract
        .mint(
            "2".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap_err();
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner));

    // New owner can mint.
    nft_contract
        .mint(
            "2".to_string(),
            RANDOM.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(RANDOM)
        .unwrap();
}

#[test]
fn test_approving_revoking() {
    let app = App::default();
    let TestCase { nft_contract } = TestCase::new(&app);

    // Minter Mints NFT
    nft_contract
        .mint(
            "1".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    // Token owner shows in approval query
    let res = nft_contract
        .cw721_interface_proxy()
        .approval("1".to_string(), CREATOR.to_string(), false)
        .unwrap();

    assert_eq!(
        res,
        ApprovalResponse {
            approval: Approval {
                spender: CREATOR.to_string(),
                expires: Expiration::Never {}
            }
        }
    );

    // Give random transferring power
    nft_contract
        .cw721_interface_proxy()
        .approve(
            RANDOM.to_string(),
            "1".to_string(),
            Some(Expiration::AtHeight(1000000)),
        )
        .call(CREATOR)
        .unwrap();

    // Test approval query
    let res = nft_contract
        .cw721_interface_proxy()
        .approval("1".to_string(), RANDOM.to_string(), false)
        .unwrap();
    assert_eq!(
        res,
        ApprovalResponse {
            approval: Approval {
                spender: String::from(RANDOM),
                expires: Expiration::AtHeight(1000000)
            }
        }
    );

    // Random can now transfer NFT to its address
    nft_contract
        .cw721_interface_proxy()
        .transfer_nft(RANDOM.to_string(), "1".to_string())
        .call(RANDOM)
        .unwrap();

    // Approvals are removed / cleared
    let res = nft_contract
        .cw721_interface_proxy()
        .owner_of("1".to_string(), false)
        .unwrap();
    assert_eq!(
        res,
        OwnerOfResponse {
            owner: String::from(RANDOM),
            approvals: vec![],
        }
    );

    // Approve, revoke, and check for empty, to test revoke
    nft_contract
        .cw721_interface_proxy()
        .approve(CREATOR.to_string(), "1".to_string(), None)
        .call(RANDOM)
        .unwrap();
    nft_contract
        .cw721_interface_proxy()
        .revoke(CREATOR.to_string(), "1".to_string())
        .call(RANDOM)
        .unwrap();

    // Approvals are now removed / cleared
    let res = nft_contract
        .cw721_interface_proxy()
        .owner_of("1".to_string(), false)
        .unwrap();

    assert_eq!(
        res,
        OwnerOfResponse {
            owner: String::from(RANDOM),
            approvals: vec![],
        }
    );
}

#[test]
fn approving_all_revoking_all() {
    let app = App::default();
    let TestCase { nft_contract } = TestCase::new(&app);

    // Minter Mints a couple NFTs for themselves
    nft_contract
        .mint(
            "1".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    nft_contract
        .mint(
            "2".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    // Paginate token ids
    let tokens = nft_contract
        .cw721_interface_proxy()
        .all_tokens(None, Some(1))
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec!["1".to_string()], tokens.tokens);
    let tokens = nft_contract
        .cw721_interface_proxy()
        .all_tokens(Some("1".to_string()), Some(3))
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec!["2".to_string()], tokens.tokens);

    // Creator gives random full (operator) power over her tokens
    nft_contract
        .cw721_interface_proxy()
        .approve_all(RANDOM.to_string(), None)
        .call(CREATOR)
        .unwrap();

    // Random can now transfer
    nft_contract
        .cw721_interface_proxy()
        .transfer_nft(RANDOM.to_string(), "1".to_string())
        .call(RANDOM)
        .unwrap();

    // Random can now transfer
    nft_contract
        .cw721_interface_proxy()
        .transfer_nft(RANDOM.to_string(), "2".to_string())
        .call(RANDOM)
        .unwrap();

    // Approve_all, revoke_all, and check for empty, to test revoke_all
    nft_contract
        .cw721_interface_proxy()
        .approve_all(CREATOR.to_string(), Some(Expiration::AtHeight(1000000)))
        .call(RANDOM)
        .unwrap();

    // Query for operator should return approvals
    let res = nft_contract
        .cw721_interface_proxy()
        .operators(RANDOM.to_string(), false, None, None)
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![cw721::Approval {
                spender: String::from(CREATOR),
                expires: Expiration::AtHeight(1000000)
            }]
        }
    );

    // Second approval
    nft_contract
        .cw721_interface_proxy()
        .approve_all("second".to_string(), Some(Expiration::AtHeight(1000000)))
        .call(RANDOM)
        .unwrap();

    // Paginate queries
    let res = nft_contract
        .cw721_interface_proxy()
        .operators(RANDOM.to_string(), true, None, Some(1))
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![cw721::Approval {
                spender: String::from(CREATOR),
                expires: Expiration::AtHeight(1000000),
            }]
        }
    );
    let res = nft_contract
        .cw721_interface_proxy()
        .operators(RANDOM.to_string(), true, Some(CREATOR.to_string()), Some(1))
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![cw721::Approval {
                spender: String::from("second"),
                expires: Expiration::AtHeight(1000000),
            }]
        }
    );

    // Test operator query
    let res = nft_contract
        .cw721_interface_proxy()
        .operator(CREATOR.to_string(), RANDOM.to_string(), false)
        .unwrap();
    assert_eq!(
        res,
        OperatorResponse {
            approval: Approval {
                spender: String::from(RANDOM),
                expires: Expiration::Never {}
            }
        }
    );

    // Revoke all approvals for CREATOR
    nft_contract
        .cw721_interface_proxy()
        .revoke_all(CREATOR.to_string())
        .call(RANDOM)
        .unwrap();

    // Approvals are removed / cleared without affecting others
    let res = nft_contract
        .cw721_interface_proxy()
        .operators(RANDOM.to_string(), false, None, None)
        .unwrap();
    assert_eq!(
        res,
        OperatorsResponse {
            operators: vec![Approval {
                spender: "second".to_string(),
                expires: Expiration::AtHeight(1000000)
            }]
        }
    );

    // Query for old operator should return error
    let res = nft_contract.cw721_interface_proxy().operator(
        RANDOM.to_string(),
        CREATOR.to_string(),
        false,
    );
    match res {
        Err(ContractError::Std(StdError::GenericErr { msg })) => {
            assert_eq!(msg, "Querier contract error: Approval not found not found")
        }
        _ => panic!("Unexpected error"),
    }

    // Ensure the filter works (approvals should expire)
    let mut block = nft_contract.app.block_info();
    block.height += 1000000;
    nft_contract.app.set_block(block);

    let res = nft_contract
        .cw721_interface_proxy()
        .operators(RANDOM.to_string(), false, None, None)
        .unwrap();
    assert_eq!(0, res.operators.len());

    // Query operator should also return error
    let res = nft_contract.cw721_interface_proxy().operator(
        RANDOM.to_string(),
        CREATOR.to_string(),
        false,
    );
    match res {
        Err(ContractError::Std(StdError::GenericErr { msg })) => {
            assert_eq!(msg, "Querier contract error: Approval not found not found")
        }
        _ => panic!("Unexpected error"),
    }
}

#[should_panic(
    expected = "called `Result::unwrap()` on an `Err` value: Error executing WasmMsg:\n  sender: creator\n  Execute { contract_addr: \"contract0\", msg: {\"send_nft\":{\"contract\":\"contract0\",\"token_id\":\"1\",\"msg\":\"e30=\"}}, funds: [] }\n\nCaused by:\n    0: Error executing WasmMsg:\n         sender: contract0\n         Execute { contract_addr: \"contract0\", msg: {\"receive_nft\":{\"sender\":\"creator\",\"token_id\":\"1\",\"msg\":\"e30=\"}}, funds: [] }\n    1: Error parsing into type cw721_sylvia_base::contract::sv::ContractExecMsg: Unsupported message received: {\"receive_nft\":{\"msg\":\"e30=\",\"sender\":\"creator\",\"token_id\":\"1\"}}. Messages supported by this contract: approve, approve_all, burn, revoke, revoke_all, send_nft, transfer_nft, mint, update_ownership"
)]
#[test]
fn test_send_with_approval() {
    let app = App::default();
    let TestCase { nft_contract } = TestCase::new(&app);

    // Mint NFT
    nft_contract
        .mint(
            "1".to_string(),
            CREATOR.to_string(),
            Some("https://example.com".to_string()),
        )
        .call(CREATOR)
        .unwrap();

    // Grant approval
    nft_contract
        .cw721_interface_proxy()
        .approve(
            RANDOM.to_string(),
            "1".to_string(),
            Some(Expiration::Never {}),
        )
        .call(CREATOR)
        .unwrap();

    // Random address can send
    // This will panic as the base contract does not implement RecieveNft
    nft_contract
        .cw721_interface_proxy()
        .send_nft(
            nft_contract.contract_addr.into_string(),
            "1".to_string(),
            to_json_binary(&Empty {}).unwrap(),
        )
        .call(CREATOR)
        .unwrap_err();
}
