#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{
        to_binary, Addr, Binary, Empty, OverflowError, Response, StdError, Uint128,
    };

    use crate::{ContractError, Cw1155Contract, ExecuteMsg, InstantiateMsg, MintMsg};
    use cw1155::{
        AllBalancesResponse, ApprovedForAllResponse, Balance, BalanceResponse,
        BatchBalanceResponse, Cw1155BatchReceiveMsg, Cw1155QueryMsg, Expiration,
        IsApprovedForAllResponse, NumTokensResponse, TokenInfoResponse, TokensResponse,
    };

    #[test]
    fn check_transfers() {
        let contract = Cw1155Contract::default();
        // A long test case that try to cover as many cases as possible.
        // Summary of what it does:
        // - try mint without permission, fail
        // - mint with permission, success
        // - query balance of receipant, success
        // - try transfer without approval, fail
        // - approve
        // - transfer again, success
        // - query balance of transfer participants
        // - try batch transfer without approval, fail
        // - approve and try batch transfer again, success
        // - batch query balances
        // - user1 revoke approval to minter
        // - query approval status
        // - minter try to transfer, fail
        // - user1 burn token1
        // - user1 batch burn token2 and token3
        let token1 = "token1".to_owned();
        let token2 = "token2".to_owned();
        let token3 = "token3".to_owned();
        let minter = String::from("minter");
        let user1 = String::from("user1");
        let user2 = String::from("user2");

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = contract
            .instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg)
            .unwrap();
        assert_eq!(0, res.messages.len());

        // invalid mint, user1 don't mint permission
        let mint_msg = ExecuteMsg::Mint(MintMsg::<Empty> {
            to: user1.clone(),
            token_id: token1.clone(),
            value: 1u64.into(),
            token_uri: None,
            extension: None,
        });
        assert!(matches!(
            contract.execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.as_ref(), &[]),
                mint_msg.clone(),
            ),
            Err(ContractError::Unauthorized {})
        ));

        // valid mint
        assert_eq!(
            contract
                .execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(minter.as_ref(), &[]),
                    mint_msg,
                )
                .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token1)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("to", &user1)
        );

        // query balance
        assert_eq!(
            to_binary(&BalanceResponse {
                balance: 1u64.into()
            }),
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Balance {
                    owner: user1.clone(),
                    token_id: token1.clone(),
                }
            ),
        );

        let transfer_msg = ExecuteMsg::SendFrom {
            from: user1.clone(),
            to: user2.clone(),
            token_id: token1.clone(),
            value: 1u64.into(),
            msg: None,
        };

        // not approved yet
        assert!(matches!(
            contract.execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                transfer_msg.clone(),
            ),
            Err(ContractError::Unauthorized {})
        ));

        // approve
        contract
            .execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.as_ref(), &[]),
                ExecuteMsg::ApproveAll {
                    operator: minter.clone(),
                    expires: None,
                },
            )
            .unwrap();

        // transfer
        assert_eq!(
            contract
                .execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(minter.as_ref(), &[]),
                    transfer_msg,
                )
                .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token1)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
                .add_attribute("to", &user2)
        );

        // query balance
        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Balance {
                    owner: user2.clone(),
                    token_id: token1.clone(),
                }
            ),
            to_binary(&BalanceResponse {
                balance: 1u64.into()
            }),
        );
        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Balance {
                    owner: user1.clone(),
                    token_id: token1.clone(),
                }
            ),
            to_binary(&BalanceResponse {
                balance: 0u64.into()
            }),
        );

        // mint token2 and token3
        contract
            .execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                ExecuteMsg::Mint(MintMsg::<Empty> {
                    to: user2.clone(),
                    token_id: token2.clone(),
                    value: 1u64.into(),
                    token_uri: None,
                    extension: None,
                }),
            )
            .unwrap();

        contract
            .execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                ExecuteMsg::Mint(MintMsg::<Empty> {
                    to: user2.clone(),
                    token_id: token3.clone(),
                    value: 1u64.into(),
                    token_uri: None,
                    extension: None,
                }),
            )
            .unwrap();

        // invalid batch transfer, (user2 not approved yet)
        let batch_transfer_msg = ExecuteMsg::BatchSendFrom {
            from: user2.clone(),
            to: user1.clone(),
            batch: vec![
                (token1.clone(), 1u64.into()),
                (token2.clone(), 1u64.into()),
                (token3.clone(), 1u64.into()),
            ],
            msg: None,
        };
        assert!(matches!(
            contract.execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                batch_transfer_msg.clone(),
            ),
            Err(ContractError::Unauthorized {}),
        ));

        // user2 approve
        contract
            .execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user2.as_ref(), &[]),
                ExecuteMsg::ApproveAll {
                    operator: minter.clone(),
                    expires: None,
                },
            )
            .unwrap();

        // valid batch transfer
        assert_eq!(
            contract
                .execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(minter.as_ref(), &[]),
                    batch_transfer_msg,
                )
                .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token1)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user2)
                .add_attribute("to", &user1)
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token2)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user2)
                .add_attribute("to", &user1)
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token3)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user2)
                .add_attribute("to", &user1)
        );

        // batch query
        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::BatchBalance {
                    owner: user1.clone(),
                    token_ids: vec![token1.clone(), token2.clone(), token3.clone()],
                }
            ),
            to_binary(&BatchBalanceResponse {
                balances: vec![1u64.into(), 1u64.into(), 1u64.into()]
            }),
        );

        // user1 revoke approval
        contract
            .execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.as_ref(), &[]),
                ExecuteMsg::RevokeAll {
                    operator: minter.clone(),
                },
            )
            .unwrap();

        // query approval status
        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::IsApprovedForAll {
                    owner: user1.clone(),
                    operator: minter.clone(),
                }
            ),
            to_binary(&IsApprovedForAllResponse { approved: false }),
        );

        // tranfer without approval
        assert!(matches!(
            contract.execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                ExecuteMsg::SendFrom {
                    from: user1.clone(),
                    to: user2,
                    token_id: token1.clone(),
                    value: 1u64.into(),
                    msg: None,
                },
            ),
            Err(ContractError::Unauthorized {})
        ));

        // burn token1
        assert_eq!(
            contract
                .execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(user1.as_ref(), &[]),
                    ExecuteMsg::Burn {
                        from: user1.clone(),
                        token_id: token1.clone(),
                        value: 1u64.into(),
                    }
                )
                .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token1)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
        );

        // burn them all
        assert_eq!(
            contract
                .execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(user1.as_ref(), &[]),
                    ExecuteMsg::BatchBurn {
                        from: user1.clone(),
                        batch: vec![(token2.clone(), 1u64.into()), (token3.clone(), 1u64.into())]
                    }
                )
                .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token2)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token3)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
        );
    }

    #[test]
    fn check_send_contract() {
        let contract = Cw1155Contract::default();
        let receiver = String::from("receive_contract");
        let minter = String::from("minter");
        let user1 = String::from("user1");
        let token2 = "token2".to_owned();
        let dummy_msg = Binary::default();

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = contract
            .instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg)
            .unwrap();
        assert_eq!(0, res.messages.len());

        contract
            .execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                ExecuteMsg::Mint(MintMsg::<Empty> {
                    to: user1.clone(),
                    token_id: token2.clone(),
                    value: 1u64.into(),
                    token_uri: None,
                    extension: None,
                }),
            )
            .unwrap();

        // BatchSendFrom
        assert_eq!(
            contract
                .execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(user1.as_ref(), &[]),
                    ExecuteMsg::BatchSendFrom {
                        from: user1.clone(),
                        to: receiver.clone(),
                        batch: vec![(token2.clone(), 1u64.into())],
                        msg: Some(dummy_msg.clone()),
                    },
                )
                .unwrap(),
            Response::new()
                .add_message(
                    Cw1155BatchReceiveMsg {
                        operator: user1.clone(),
                        from: Some(user1.clone()),
                        batch: vec![(token2.clone(), 1u64.into())],
                        msg: dummy_msg,
                    }
                    .into_cosmos_msg(receiver.clone())
                    .unwrap()
                )
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token2)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
                .add_attribute("to", &receiver)
        );
    }

    #[test]
    fn check_queries() {
        let contract = Cw1155Contract::default();
        // mint multiple types of tokens, and query them
        // grant approval to multiple operators, and query them
        let tokens = (0..10).map(|i| format!("token{}", i)).collect::<Vec<_>>();
        let users = (0..10).map(|i| format!("user{}", i)).collect::<Vec<_>>();
        let minter = String::from("minter");

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = contract
            .instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg)
            .unwrap();
        assert_eq!(0, res.messages.len());

        for token_id in tokens.clone() {
            contract
                .execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(minter.as_ref(), &[]),
                    ExecuteMsg::Mint(MintMsg::<Empty> {
                        to: users[0].clone(),
                        token_id: token_id.clone(),
                        value: 1u64.into(),
                        token_uri: None,
                        extension: None,
                    }),
                )
                .unwrap();
        }

        for user in users[1..].iter() {
            contract
                .execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(minter.as_ref(), &[]),
                    ExecuteMsg::Mint(MintMsg::<Empty> {
                        to: user.clone(),
                        token_id: tokens[9].clone(),
                        value: 1u64.into(),
                        token_uri: None,
                        extension: None,
                    }),
                )
                .unwrap();
        }

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::NumTokens {
                    token_id: tokens[0].clone(),
                },
            ),
            to_binary(&NumTokensResponse {
                count: Uint128::new(1),
            })
        );

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::NumTokens {
                    token_id: tokens[0].clone(),
                },
            ),
            to_binary(&NumTokensResponse {
                count: Uint128::new(1),
            })
        );

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::AllBalances {
                    token_id: tokens[9].clone(),
                    start_after: None,
                    limit: Some(5)
                },
            ),
            to_binary(&AllBalancesResponse {
                balances: users[..5]
                    .iter()
                    .map(|user| {
                        Balance {
                            owner: Addr::unchecked(user),
                            amount: Uint128::new(1),
                            token_id: tokens[9].clone(),
                        }
                    })
                    .collect(),
            })
        );

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::AllBalances {
                    token_id: tokens[9].clone(),
                    start_after: Some("user5".to_owned()),
                    limit: Some(5)
                },
            ),
            to_binary(&AllBalancesResponse {
                balances: users[6..]
                    .iter()
                    .map(|user| {
                        Balance {
                            owner: Addr::unchecked(user),
                            amount: Uint128::new(1),
                            token_id: tokens[9].clone(),
                        }
                    })
                    .collect(),
            })
        );

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Tokens {
                    owner: users[0].clone(),
                    start_after: None,
                    limit: Some(5),
                },
            ),
            to_binary(&TokensResponse {
                tokens: tokens[..5].to_owned()
            })
        );

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Tokens {
                    owner: users[0].clone(),
                    start_after: Some("token5".to_owned()),
                    limit: Some(5),
                },
            ),
            to_binary(&TokensResponse {
                tokens: tokens[6..].to_owned()
            })
        );

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::AllTokens {
                    start_after: Some("token5".to_owned()),
                    limit: Some(5),
                },
            ),
            to_binary(&TokensResponse {
                tokens: tokens[6..].to_owned()
            })
        );

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::TokenInfo {
                    token_id: "token5".to_owned()
                },
            ),
            to_binary(&TokenInfoResponse::<Empty> {
                token_uri: None,
                extension: None,
            }),
        );

        for user in users[1..].iter() {
            contract
                .execute(
                    deps.as_mut(),
                    mock_env(),
                    mock_info(users[0].as_ref(), &[]),
                    ExecuteMsg::ApproveAll {
                        operator: user.clone(),
                        expires: None,
                    },
                )
                .unwrap();
        }

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::ApprovedForAll {
                    owner: users[0].clone(),
                    include_expired: None,
                    start_after: Some(String::from("user2")),
                    limit: Some(1),
                },
            ),
            to_binary(&ApprovedForAllResponse {
                operators: vec![cw1155::Approval {
                    spender: users[3].clone(),
                    expires: Expiration::Never {}
                }],
            })
        );
    }

    #[test]
    fn approval_expires() {
        let contract = Cw1155Contract::default();
        let mut deps = mock_dependencies();
        let token1 = "token1".to_owned();
        let minter = String::from("minter");
        let user1 = String::from("user1");
        let user2 = String::from("user2");

        let env = {
            let mut env = mock_env();
            env.block.height = 10;
            env
        };

        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = contract
            .instantiate(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg)
            .unwrap();
        assert_eq!(0, res.messages.len());

        contract
            .execute(
                deps.as_mut(),
                env.clone(),
                mock_info(minter.as_ref(), &[]),
                ExecuteMsg::Mint(MintMsg::<Empty> {
                    to: user1.clone(),
                    token_id: token1,
                    value: 1u64.into(),
                    token_uri: None,
                    extension: None,
                }),
            )
            .unwrap();

        // invalid expires should be rejected
        assert!(matches!(
            contract.execute(
                deps.as_mut(),
                env.clone(),
                mock_info(user1.as_ref(), &[]),
                ExecuteMsg::ApproveAll {
                    operator: user2.clone(),
                    expires: Some(Expiration::AtHeight(5)),
                },
            ),
            Err(_)
        ));

        contract
            .execute(
                deps.as_mut(),
                env.clone(),
                mock_info(user1.as_ref(), &[]),
                ExecuteMsg::ApproveAll {
                    operator: user2.clone(),
                    expires: Some(Expiration::AtHeight(100)),
                },
            )
            .unwrap();

        let query_msg = Cw1155QueryMsg::IsApprovedForAll {
            owner: user1,
            operator: user2,
        };
        assert_eq!(
            contract.query(deps.as_ref(), env, query_msg.clone()),
            to_binary(&IsApprovedForAllResponse { approved: true })
        );

        let env = {
            let mut env = mock_env();
            env.block.height = 100;
            env
        };

        assert_eq!(
            contract.query(deps.as_ref(), env, query_msg,),
            to_binary(&IsApprovedForAllResponse { approved: false })
        );
    }

    #[test]
    fn mint_overflow() {
        let contract = Cw1155Contract::default();
        let mut deps = mock_dependencies();
        let token1 = "token1".to_owned();
        let minter = String::from("minter");
        let user1 = String::from("user1");

        let env = mock_env();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = contract
            .instantiate(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg)
            .unwrap();
        assert_eq!(0, res.messages.len());

        contract
            .execute(
                deps.as_mut(),
                env.clone(),
                mock_info(minter.as_ref(), &[]),
                ExecuteMsg::Mint(MintMsg::<Empty> {
                    to: user1.clone(),
                    token_id: token1.clone(),
                    value: u128::MAX.into(),
                    token_uri: None,
                    extension: None,
                }),
            )
            .unwrap();

        assert!(matches!(
            contract.execute(
                deps.as_mut(),
                env,
                mock_info(minter.as_ref(), &[]),
                ExecuteMsg::Mint(MintMsg::<Empty> {
                    to: user1,
                    token_id: token1,
                    value: 1u64.into(),
                    token_uri: None,
                    extension: None,
                }),
            ),
            Err(ContractError::Std(StdError::Overflow {
                source: OverflowError { .. },
                ..
            }))
        ));
    }

    #[test]
    fn token_uri() {
        let contract = Cw1155Contract::default();
        let minter = String::from("minter");
        let user1 = String::from("user1");
        let token1 = "token1".to_owned();
        let url1 = "url1".to_owned();
        let url2 = "url2".to_owned();

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = contract
            .instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg)
            .unwrap();
        assert_eq!(0, res.messages.len());

        // first mint
        contract
            .execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                ExecuteMsg::Mint(MintMsg::<Empty> {
                    to: user1.clone(),
                    token_id: token1.clone(),
                    value: 1u64.into(),
                    token_uri: Some(url1.clone()),
                    extension: None,
                }),
            )
            .unwrap();

        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::TokenInfo {
                    token_id: token1.clone()
                }
            ),
            to_binary(&TokenInfoResponse::<Empty> {
                token_uri: Some(url1.clone()),
                extension: None,
            })
        );

        // mint after the first mint
        contract
            .execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                ExecuteMsg::Mint(MintMsg::<Empty> {
                    to: user1,
                    token_id: token1.clone(),
                    value: 1u64.into(),
                    token_uri: Some(url2),
                    extension: None,
                }),
            )
            .unwrap();

        // url doesn't changed
        assert_eq!(
            contract.query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::TokenInfo { token_id: token1 }
            ),
            to_binary(&TokenInfoResponse::<Empty> {
                token_uri: Some(url1),
                extension: None,
            })
        );
    }
}
