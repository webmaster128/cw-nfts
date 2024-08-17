use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{
    Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg, Uint128,
};

use cw1155::{
    ApproveAllEvent, Balance, Cw1155BatchReceiveMsg, Cw1155ReceiveMsg, Expiration, TransferEvent,
};
use cw2::set_contract_version;
use cw_utils::Event;

use crate::error::ContractError;
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
    ) -> Result<Response, ContractError> {
        let env = ExecuteEnv { deps, env, info };
        match msg {
            ExecuteMsg::Mint(msg) => self.mint(env, msg),
            ExecuteMsg::SendFrom {
                from,
                to,
                token_id,
                value,
                msg,
            } => self.send_from(env, from, to, token_id, value, msg),
            ExecuteMsg::BatchSendFrom {
                from,
                to,
                batch,
                msg,
            } => self.batch_send_from(env, from, to, batch, msg),
            ExecuteMsg::Burn {
                from,
                token_id,
                value,
            } => self.burn(env, from, token_id, value),
            ExecuteMsg::BatchBurn { from, batch } => self.batch_burn(env, from, batch),
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
    pub fn mint(&self, env: ExecuteEnv, msg: MintMsg<T>) -> Result<Response, ContractError> {
        let ExecuteEnv { mut deps, info, .. } = env;
        let to_addr = deps.api.addr_validate(&msg.to)?;

        if info.sender != self.minter.load(deps.storage)? {
            return Err(ContractError::Unauthorized {});
        }

        let mut rsp = Response::default();

        let event = self.execute_transfer_inner(
            &mut deps,
            None,
            Some(to_addr),
            msg.token_id.clone(),
            msg.value,
        )?;
        event.add_attributes(&mut rsp);

        // insert if not exist (if it is the first mint)
        if !self.tokens.has(deps.storage, &msg.token_id) {
            // Add token info
            let token_info = TokenInfo {
                token_uri: msg.token_uri,
                extension: msg.extension,
            };

            self.tokens.save(deps.storage, &msg.token_id, &token_info)?;

            // Increase num token
            self.increment_tokens(deps.storage, &msg.token_id, &msg.value)?;
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
    ) -> Result<Response, ContractError> {
        let from_addr = env.deps.api.addr_validate(&from)?;
        let to_addr = env.deps.api.addr_validate(&to)?;

        let ExecuteEnv {
            mut deps,
            env,
            info,
        } = env;

        self.guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

        let mut rsp = Response::default();

        let event = self.execute_transfer_inner(
            &mut deps,
            Some(from_addr),
            Some(to_addr),
            token_id.clone(),
            amount,
        )?;
        event.add_attributes(&mut rsp);

        if let Some(msg) = msg {
            rsp.messages = vec![SubMsg::new(
                Cw1155ReceiveMsg {
                    operator: info.sender.to_string(),
                    from: Some(from),
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
        batch: Vec<(String, Uint128)>,
        msg: Option<Binary>,
    ) -> Result<Response, ContractError> {
        let ExecuteEnv {
            mut deps,
            env,
            info,
        } = env;

        let from_addr = deps.api.addr_validate(&from)?;
        let to_addr = deps.api.addr_validate(&to)?;

        self.guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

        let mut rsp = Response::default();
        for (token_id, amount) in batch.iter() {
            let event = self.execute_transfer_inner(
                &mut deps,
                Some(from_addr.clone()),
                Some(to_addr.clone()),
                token_id.clone(),
                *amount,
            )?;
            event.add_attributes(&mut rsp);
        }

        if let Some(msg) = msg {
            rsp.messages = vec![SubMsg::new(
                Cw1155BatchReceiveMsg {
                    operator: info.sender.to_string(),
                    from: Some(from),
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
        from: String,
        token_id: String,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        let ExecuteEnv {
            mut deps,
            info,
            env,
        } = env;

        let from_addr = deps.api.addr_validate(&from)?;

        // whoever can transfer these tokens can burn
        self.guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

        let mut rsp = Response::default();
        let event = self.execute_transfer_inner(
            &mut deps,
            Some(from_addr),
            None,
            token_id.clone(),
            amount,
        )?;

        self.decrement_tokens(deps.storage, &token_id, &amount)?;

        event.add_attributes(&mut rsp);
        Ok(rsp)
    }

    pub fn batch_burn(
        &self,
        env: ExecuteEnv,
        from: String,
        batch: Vec<(String, Uint128)>,
    ) -> Result<Response, ContractError> {
        let ExecuteEnv {
            mut deps,
            info,
            env,
        } = env;

        let from_addr = deps.api.addr_validate(&from)?;

        self.guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

        let mut rsp = Response::default();
        for (token_id, amount) in batch.into_iter() {
            let event = self.execute_transfer_inner(
                &mut deps,
                Some(from_addr.clone()),
                None,
                token_id.clone(),
                amount,
            )?;

            self.decrement_tokens(deps.storage, &token_id, &amount)?;

            event.add_attributes(&mut rsp);
        }

        Ok(rsp)
    }

    pub fn approve_all(
        &self,
        env: ExecuteEnv,
        operator: String,
        expires: Option<Expiration>,
    ) -> Result<Response, ContractError> {
        let ExecuteEnv { deps, info, env } = env;

        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&env.block) {
            return Err(ContractError::Expired {});
        }

        // set the operator for us
        let operator_addr = deps.api.addr_validate(&operator)?;
        self.approves
            .save(deps.storage, (&info.sender, &operator_addr), &expires)?;

        let mut rsp = Response::default();
        ApproveAllEvent {
            sender: &info.sender.to_string(),
            operator: &operator,
            approved: true,
        }
        .add_attributes(&mut rsp);
        Ok(rsp)
    }

    pub fn revoke_all(&self, env: ExecuteEnv, operator: String) -> Result<Response, ContractError> {
        let ExecuteEnv { deps, info, .. } = env;
        let operator_addr = deps.api.addr_validate(&operator)?;
        self.approves
            .remove(deps.storage, (&info.sender, &operator_addr));

        let mut rsp = Response::default();
        ApproveAllEvent {
            sender: &info.sender.to_string(),
            operator: &operator,
            approved: false,
        }
        .add_attributes(&mut rsp);
        Ok(rsp)
    }

    /// When from is None: mint new coins
    /// When to is None: burn coins
    /// When both are None: no token balance is changed, pointless but valid
    ///
    /// Make sure permissions are checked before calling this.
    fn execute_transfer_inner(
        &self,
        deps: &mut DepsMut,
        from: Option<Addr>,
        to: Option<Addr>,
        token_id: String,
        amount: Uint128,
    ) -> Result<TransferEvent, ContractError> {
        if let Some(from_addr) = from.clone() {
            self.balances.update(
                deps.storage,
                (from_addr, token_id.clone()),
                |balance: Option<Balance>| -> StdResult<_> {
                    let mut new_balance = balance.unwrap();
                    new_balance.amount = new_balance.amount.checked_sub(amount)?;
                    Ok(new_balance)
                },
            )?;
        }

        if let Some(to_addr) = to.clone() {
            self.balances.update(
                deps.storage,
                (to_addr.clone(), token_id.clone()),
                |balance: Option<Balance>| -> StdResult<_> {
                    let mut new_balance: Balance = if let Some(balance) = balance {
                        balance
                    } else {
                        Balance {
                            owner: to_addr.clone(),
                            amount: Uint128::new(0),
                            token_id: token_id.clone(),
                        }
                    };

                    new_balance.amount = new_balance.amount.checked_add(amount)?;
                    Ok(new_balance)
                },
            )?;
        }

        Ok(TransferEvent {
            from: from.map(|x| x.to_string()),
            to: to.map(|x| x.to_string()),
            token_id: token_id.clone(),
            amount,
        })
    }

    /// returns true if the sender can execute approve or reject on the contract
    pub fn check_can_approve(
        &self,
        deps: Deps,
        env: &Env,
        owner: &Addr,
        operator: &Addr,
    ) -> StdResult<bool> {
        // owner can approve
        if owner == operator {
            return Ok(true);
        }
        // operator can approve
        let op = self.approves.may_load(deps.storage, (owner, operator))?;
        Ok(match op {
            Some(ex) => !ex.is_expired(&env.block),
            None => false,
        })
    }

    fn guard_can_approve(
        &self,
        deps: Deps,
        env: &Env,
        owner: &Addr,
        operator: &Addr,
    ) -> Result<(), ContractError> {
        if !self.check_can_approve(deps, env, owner, operator)? {
            Err(ContractError::Unauthorized {})
        } else {
            Ok(())
        }
    }
}
