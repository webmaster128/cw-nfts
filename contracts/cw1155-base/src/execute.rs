use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{
    Addr, Binary, DepsMut, Env, Event, MessageInfo, Response, StdResult, Storage, SubMsg, Uint128,
};

use cw1155::{
    ApproveAllEvent, Balance, BurnEvent, Cw1155BatchReceiveMsg, Cw1155ContractError,
    Cw1155ReceiveMsg, Expiration, MintEvent, RevokeAllEvent, TokenAmount, TransferEvent,
};
use cw2::set_contract_version;

use crate::msg::{ExecuteMsg, InstantiateMsg, MintMsg};
use crate::state::{Cw1155Contract, TokenInfo};

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cw721-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

impl<'a, T> Cw1155Contract<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        let minter = deps.api.addr_validate(&msg.minter)?;
        self.minter.save(deps.storage, &minter)?;
        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<T>,
    ) -> Result<Response, Cw1155ContractError> {
        let env = ExecuteEnv { deps, env, info };
        match msg {
            ExecuteMsg::Mint(msg) => self.mint(env, msg),
            ExecuteMsg::SendFrom {
                from,
                to,
                token_id,
                amount,
                msg,
            } => self.send_from(env, from, to, token_id, amount, msg),
            ExecuteMsg::BatchSendFrom {
                from,
                to,
                batch,
                msg,
            } => self.batch_send_from(env, from, to, batch, msg),
            ExecuteMsg::Burn { token_id, amount } => self.burn(env, token_id, amount),
            ExecuteMsg::BatchBurn { batch } => self.batch_burn(env, batch),
            ExecuteMsg::ApproveAll { operator, expires } => {
                self.approve_all(env, operator, expires)
            }
            ExecuteMsg::RevokeAll { operator } => self.revoke_all(env, operator),
        }
    }
}

/// To mitigate clippy::too_many_arguments warning
pub struct ExecuteEnv<'a> {
    deps: DepsMut<'a>,
    env: Env,
    info: MessageInfo,
}

// helper
impl<'a, T> Cw1155Contract<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    pub fn mint(&self, env: ExecuteEnv, msg: MintMsg<T>) -> Result<Response, Cw1155ContractError> {
        let ExecuteEnv { mut deps, info, .. } = env;
        let to_addr = deps.api.addr_validate(&msg.to)?;

        if info.sender != self.minter.load(deps.storage)? {
            return Err(Cw1155ContractError::Unauthorized {});
        }

        let mut rsp = Response::default();

        let event = self.update_transfer_state(
            &mut deps,
            None,
            Some(to_addr),
            vec![TokenAmount {
                token_id: msg.token_id.to_string(),
                amount: msg.amount,
            }],
        )?;
        rsp = rsp.add_event(event);

        // insert if not exist (if it is the first mint)
        if !self.tokens.has(deps.storage, &msg.token_id) {
            // Add token info
            let token_info = TokenInfo {
                token_uri: msg.token_uri,
                extension: msg.extension,
            };

            self.tokens.save(deps.storage, &msg.token_id, &token_info)?;

            // Increase num token
            self.increment_tokens(deps.storage, &msg.token_id, &msg.amount)?;
        }

        Ok(rsp)
    }

    pub fn send_from(
        &self,
        env: ExecuteEnv,
        from: String,
        to: String,
        token_id: String,
        amount: Uint128,
        msg: Option<Binary>,
    ) -> Result<Response, Cw1155ContractError> {
        let ExecuteEnv {
            mut deps,
            env,
            info,
        } = env;

        let from = deps.api.addr_validate(&from)?;
        let to = deps.api.addr_validate(&to)?;

        let balance_update =
            self.verify_approval(deps.storage, &env, &info, &from, &token_id, amount)?;

        let mut rsp = Response::default();

        let event = self.update_transfer_state(
            &mut deps,
            Some(from.clone()),
            Some(to.clone()),
            vec![TokenAmount {
                token_id: token_id.to_string(),
                amount: balance_update.amount,
            }],
        )?;
        rsp = rsp.add_event(event);

        if let Some(msg) = msg {
            rsp.messages = vec![SubMsg::new(
                Cw1155ReceiveMsg {
                    operator: info.sender.to_string(),
                    from: Some(from.to_string()),
                    amount,
                    token_id,
                    msg,
                }
                .into_cosmos_msg(to)?,
            )]
        }

        Ok(rsp)
    }

    pub fn batch_send_from(
        &self,
        env: ExecuteEnv,
        from: String,
        to: String,
        batch: Vec<TokenAmount>,
        msg: Option<Binary>,
    ) -> Result<Response, Cw1155ContractError> {
        let ExecuteEnv {
            mut deps,
            env,
            info,
        } = env;

        let from = deps.api.addr_validate(&from)?;
        let to = deps.api.addr_validate(&to)?;

        let batch = self.verify_approvals(deps.storage, &env, &info, &from, batch)?;

        let mut rsp = Response::default();
        let event = self.update_transfer_state(
            &mut deps,
            Some(from.clone()),
            Some(to.clone()),
            batch.to_vec(),
        )?;
        rsp = rsp.add_event(event);

        if let Some(msg) = msg {
            rsp.messages = vec![SubMsg::new(
                Cw1155BatchReceiveMsg {
                    operator: info.sender.to_string(),
                    from: Some(from.to_string()),
                    batch,
                    msg,
                }
                .into_cosmos_msg(to)?,
            )]
        };

        Ok(rsp)
    }

    pub fn burn(
        &self,
        env: ExecuteEnv,
        token_id: String,
        amount: Uint128,
    ) -> Result<Response, Cw1155ContractError> {
        let ExecuteEnv {
            mut deps,
            info,
            env,
        } = env;

        let from = &info.sender;

        // whoever can transfer these tokens can burn
        let balance_update =
            self.verify_approval(deps.storage, &env, &info, from, &token_id, amount)?;

        let mut rsp = Response::default();

        let event = self.update_transfer_state(
            &mut deps,
            Some(from.clone()),
            None,
            vec![TokenAmount {
                token_id: token_id.to_string(),
                amount: balance_update.amount,
            }],
        )?;
        rsp = rsp.add_event(event);

        Ok(rsp)
    }

    pub fn batch_burn(
        &self,
        env: ExecuteEnv,
        batch: Vec<TokenAmount>,
    ) -> Result<Response, Cw1155ContractError> {
        let ExecuteEnv {
            mut deps,
            info,
            env,
        } = env;

        let from = &info.sender;

        let batch = self.verify_approvals(deps.storage, &env, &info, from, batch)?;

        let mut rsp = Response::default();
        let event = self.update_transfer_state(&mut deps, Some(from.clone()), None, batch)?;
        rsp = rsp.add_event(event);

        Ok(rsp)
    }

    pub fn approve_all(
        &self,
        env: ExecuteEnv,
        operator: String,
        expires: Option<Expiration>,
    ) -> Result<Response, Cw1155ContractError> {
        let ExecuteEnv { deps, info, env } = env;

        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&env.block) {
            return Err(Cw1155ContractError::Expired {});
        }

        // set the operator for us
        let operator = deps.api.addr_validate(&operator)?;
        self.approves
            .save(deps.storage, (&info.sender, &operator), &expires)?;

        let mut rsp = Response::default();

        let event = ApproveAllEvent::new(&info.sender, &operator).into();
        rsp = rsp.add_event(event);

        Ok(rsp)
    }

    pub fn revoke_all(
        &self,
        env: ExecuteEnv,
        operator: String,
    ) -> Result<Response, Cw1155ContractError> {
        let ExecuteEnv { deps, info, .. } = env;
        let operator_addr = deps.api.addr_validate(&operator)?;
        self.approves
            .remove(deps.storage, (&info.sender, &operator_addr));

        let mut rsp = Response::default();

        let event = RevokeAllEvent::new(&info.sender, &operator_addr).into();
        rsp = rsp.add_event(event);

        Ok(rsp)
    }

    /// When from is None: mint new tokens
    /// When to is None: burn tokens
    /// When both are Some: transfer tokens
    ///
    /// Make sure permissions are checked before calling this.
    fn update_transfer_state(
        &self,
        deps: &mut DepsMut,
        from: Option<Addr>,
        to: Option<Addr>,
        tokens: Vec<TokenAmount>,
    ) -> Result<Event, Cw1155ContractError> {
        if let Some(from) = &from {
            for TokenAmount { token_id, amount } in tokens.iter() {
                self.balances.update(
                    deps.storage,
                    (from.clone(), token_id.clone()),
                    |balance: Option<Balance>| -> StdResult<_> {
                        let mut new_balance = balance.unwrap();
                        new_balance.amount = new_balance.amount.checked_sub(*amount)?;
                        Ok(new_balance)
                    },
                )?;
            }
        }

        if let Some(to) = &to {
            for TokenAmount { token_id, amount } in tokens.iter() {
                self.balances.update(
                    deps.storage,
                    (to.clone(), token_id.clone()),
                    |balance: Option<Balance>| -> StdResult<_> {
                        let mut new_balance: Balance = if let Some(balance) = balance {
                            balance
                        } else {
                            Balance {
                                owner: to.clone(),
                                amount: Uint128::zero(),
                                token_id: token_id.to_string(),
                            }
                        };

                        new_balance.amount = new_balance.amount.checked_add(*amount)?;
                        Ok(new_balance)
                    },
                )?;
            }
        }

        let event = if let Some(from) = &from {
            if let Some(to) = &to {
                // transfer
                TransferEvent::new(from, to, tokens).into()
            } else {
                // burn
                for TokenAmount { token_id, amount } in &tokens {
                    self.decrement_tokens(deps.storage, token_id, amount)?;
                }
                BurnEvent::new(from, tokens).into()
            }
        } else if let Some(to) = &to {
            // mint
            for TokenAmount { token_id, amount } in &tokens {
                self.increment_tokens(deps.storage, token_id, amount)?;
            }
            MintEvent::new(to, tokens).into()
        } else {
            panic!("Invalid transfer: from and to cannot both be None")
        };

        Ok(event)
    }

    /// returns valid token amount if the sender can execute or is approved to execute
    pub fn verify_approval(
        &self,
        storage: &dyn Storage,
        env: &Env,
        info: &MessageInfo,
        owner: &Addr,
        token_id: &str,
        amount: Uint128,
    ) -> Result<TokenAmount, Cw1155ContractError> {
        let operator = &info.sender;

        let owner_balance = self
            .balances
            .load(storage, (owner.clone(), token_id.to_string()))?;
        let balance_update = TokenAmount {
            token_id: token_id.to_string(),
            amount: owner_balance.amount.min(amount),
        };

        // owner or operator can approve
        if owner == operator
            || match self.approves.may_load(storage, (owner, operator))? {
                Some(ex) => !ex.is_expired(&env.block),
                None => false,
            }
        {
            Ok(balance_update)
        } else {
            Err(Cw1155ContractError::Unauthorized {})
        }
    }

    /// returns valid token amounts if the sender can execute or is approved to execute on all provided tokens
    pub fn verify_approvals(
        &self,
        storage: &dyn Storage,
        env: &Env,
        info: &MessageInfo,
        owner: &Addr,
        tokens: Vec<TokenAmount>,
    ) -> Result<Vec<TokenAmount>, Cw1155ContractError> {
        tokens
            .iter()
            .map(|TokenAmount { token_id, amount }| {
                self.verify_approval(storage, &env, info, owner, token_id, *amount)
            })
            .collect()
    }
}
