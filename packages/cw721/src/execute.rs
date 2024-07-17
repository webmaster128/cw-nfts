use std::fmt::Display;
use cosmwasm_std::{
    Addr, Api, BankMsg, Binary, Coin, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdResult, Storage,
};
use cw_ownable::{Action, Ownership, OwnershipError};
use cw_storage_plus::Item;
use cw_utils::Expiration;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{
    error::Cw721ContractError,
    msg::{Cw721ExecuteMsg, Cw721InstantiateMsg, Cw721MigrateMsg},
    receiver::Cw721ReceiveMsg,
    state::{CollectionInfo, Cw721Config, DefaultOptionMetadataExtension, NftInfo, MINTER},
    Approval,
};

pub trait Cw721Execute<
    // Metadata defined in NftInfo (used for mint).
    TMetadataExtension,
    // Defines for `CosmosMsg::Custom<T>` in response. Barely used, so `Empty` can be used.
    TCustomResponseMessage,
    // Message passed for updating metadata.
    TMetadataExtensionMsg,
> where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
    TCustomResponseMessage: CustomMsg,
    TMetadataExtensionMsg: CustomMsg,
{
    fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: Cw721InstantiateMsg,
        contract_name: &str,
        contract_version: &str,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        cw2::set_contract_version(deps.storage, contract_name, contract_version)?;
        let config = Cw721Config::<Empty, Empty, Empty>::default();
        let collection_info = CollectionInfo {
            name: msg.name,
            symbol: msg.symbol,
        };
        config
            .collection_info
            .save(deps.storage, &collection_info)?;

        let minter = match msg.minter {
            Some(owner) => deps.api.addr_validate(&owner)?,
            None => info.sender,
        };
        self.initialize_minter(deps.storage, deps.api, Some(minter.as_ref()))?;

        if let Some(withdraw_address) = msg.withdraw_address {
            self.set_withdraw_address(deps, &minter, withdraw_address)?;
        }

        Ok(Response::default().add_attribute("minter", minter))
    }

    fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw721ExecuteMsg<TMetadataExtension, TMetadataExtensionMsg>,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        match msg {
            Cw721ExecuteMsg::Mint {
                token_id,
                owner,
                token_uri,
                extension,
            } => self.mint(deps, info, token_id, owner, token_uri, extension),
            Cw721ExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            } => self.approve(deps, env, info, spender, token_id, expires),
            Cw721ExecuteMsg::Revoke { spender, token_id } => {
                self.revoke(deps, env, info, spender, token_id)
            }
            Cw721ExecuteMsg::ApproveAll { operator, expires } => {
                self.approve_all(deps, env, info, operator, expires)
            }
            Cw721ExecuteMsg::RevokeAll { operator } => self.revoke_all(deps, env, info, operator),
            Cw721ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => self.transfer_nft(deps, env, info, recipient, token_id),
            Cw721ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => self.send_nft(deps, env, info, contract, token_id, msg),
            Cw721ExecuteMsg::Burn { token_id } => self.burn_nft(deps, env, info, token_id),
            Cw721ExecuteMsg::UpdateOwnership(action) => {
                self.update_minter_ownership(deps, env, info, action)
            }
            Cw721ExecuteMsg::Extension { msg } => {
                self.update_metadata_extension(deps, env, info, msg)
            }
            Cw721ExecuteMsg::SetWithdrawAddress { address } => {
                self.set_withdraw_address(deps, &info.sender, address)
            }
            Cw721ExecuteMsg::RemoveWithdrawAddress {} => {
                self.remove_withdraw_address(deps.storage, &info.sender)
            }
            Cw721ExecuteMsg::WithdrawFunds { amount } => self.withdraw_funds(deps.storage, &amount),
        }
    }

    fn migrate(
        &self,
        deps: DepsMut,
        env: Env,
        msg: Cw721MigrateMsg,
        contract_name: &str,
        contract_version: &str,
    ) -> Result<Response, Cw721ContractError> {
        let response = Response::<Empty>::default();
        // first migrate legacy data ...
        let response =
            migrate_legacy_minter_and_creator(deps.storage, deps.api, &env, &msg, response)?;
        let response = migrate_legacy_collection_info(deps.storage, &env, &msg, response)?;
        // ... then migrate
        let response = migrate_version(deps.storage, contract_name, contract_version, response)?;
        // ... and update creator and minter AFTER legacy migration
        let response = migrate_minter(deps.storage, deps.api, &env, &msg, response)?;
        Ok(response)
    }

    // ------- ERC721-based functions -------
    fn transfer_nft(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        recipient: String,
        token_id: String,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        _transfer_nft::<TMetadataExtension>(deps, &env, &info, &recipient, &token_id)?;

        Ok(Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", info.sender)
            .add_attribute("recipient", recipient)
            .add_attribute("token_id", token_id))
    }

    fn send_nft(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract: String,
        token_id: String,
        msg: Binary,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        // Transfer token
        _transfer_nft::<TMetadataExtension>(deps, &env, &info, &contract, &token_id)?;

        let send = Cw721ReceiveMsg {
            sender: info.sender.to_string(),
            token_id: token_id.clone(),
            msg,
        };

        // Send message
        Ok(Response::new()
            .add_message(send.into_cosmos_msg(contract.clone())?)
            .add_attribute("action", "send_nft")
            .add_attribute("sender", info.sender)
            .add_attribute("recipient", contract)
            .add_attribute("token_id", token_id))
    }

    fn approve(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        _update_approvals::<TMetadataExtension>(
            deps, &env, &info, &spender, &token_id, true, expires,
        )?;

        Ok(Response::new()
            .add_attribute("action", "approve")
            .add_attribute("sender", info.sender)
            .add_attribute("spender", spender)
            .add_attribute("token_id", token_id))
    }

    fn revoke(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        spender: String,
        token_id: String,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        _update_approvals::<TMetadataExtension>(
            deps, &env, &info, &spender, &token_id, false, None,
        )?;

        Ok(Response::new()
            .add_attribute("action", "revoke")
            .add_attribute("sender", info.sender)
            .add_attribute("spender", spender)
            .add_attribute("token_id", token_id))
    }

    fn approve_all(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        operator: String,
        expires: Option<Expiration>,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&env.block) {
            return Err(Cw721ContractError::Expired {});
        }

        // set the operator for us
        let operator_addr = deps.api.addr_validate(&operator)?;
        let config = Cw721Config::<
            TMetadataExtension,
            TCustomResponseMessage,
            TMetadataExtensionMsg,
        >::default();
        config
            .operators
            // stores info.sender as key (=granter, NFT owner) and operator as value (operator only(!) has control over NFTs of granter)
            // check is done in `check_can_send()`
            .save(deps.storage, (&info.sender, &operator_addr), &expires)?;

        Ok(Response::new()
            .add_attribute("action", "approve_all")
            .add_attribute("sender", info.sender)
            .add_attribute("operator", operator))
    }

    fn revoke_all(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        operator: String,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        let operator_addr = deps.api.addr_validate(&operator)?;
        let config = Cw721Config::<
            TMetadataExtension,
            TCustomResponseMessage,
            TMetadataExtensionMsg,
        >::default();
        config
            .operators
            .remove(deps.storage, (&info.sender, &operator_addr));

        Ok(Response::new()
            .add_attribute("action", "revoke_all")
            .add_attribute("sender", info.sender)
            .add_attribute("operator", operator))
    }

    fn burn_nft(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        let config = Cw721Config::<
            TMetadataExtension,
            TCustomResponseMessage,
            TMetadataExtensionMsg,
        >::default();
        let token = config.nft_info.load(deps.storage, &token_id)?;
        check_can_send(deps.as_ref(), &env, &info, &token)?;

        config.nft_info.remove(deps.storage, &token_id)?;
        config.decrement_tokens(deps.storage)?;

        Ok(Response::new()
            .add_attribute("action", "burn")
            .add_attribute("sender", info.sender)
            .add_attribute("token_id", token_id))
    }

    // ------- opionated cw721 functions -------
    fn initialize_minter(
        &self,
        storage: &mut dyn Storage,
        api: &dyn Api,
        minter: Option<&str>,
    ) -> StdResult<Ownership<Addr>> {
        MINTER.initialize_owner(storage, api, minter)
    }

    fn mint(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        token_id: String,
        owner: String,
        token_uri: Option<String>,
        extension: TMetadataExtension,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        MINTER.assert_owner(deps.storage, &info.sender)?;

        // create the token
        let token = NftInfo {
            owner: deps.api.addr_validate(&owner)?,
            approvals: vec![],
            token_uri,
            extension,
        };
        let config = Cw721Config::<TMetadataExtension, Empty, Empty>::default();
        config
            .nft_info
            .update(deps.storage, &token_id, |old| match old {
                Some(_) => Err(Cw721ContractError::Claimed {}),
                None => Ok(token),
            })?;

        config.increment_tokens(deps.storage)?;

        Ok(Response::new()
            .add_attribute("action", "mint")
            .add_attribute("minter", info.sender)
            .add_attribute("owner", owner)
            .add_attribute("token_id", token_id))
    }

    fn update_minter_ownership(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        action: Action,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        let ownership =
            MINTER.update_ownership(deps.api, deps.storage, &env.block, &info.sender, action)?;
        Ok(Response::new()
            .add_attribute("update_minter_ownership", info.sender)
            .add_attributes(ownership.into_attributes()))
    }

    /// Allows creator to update onchain metadata. For now this is a no-op.
    fn update_metadata_extension(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        _msg: TMetadataExtensionMsg,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;
        Ok(Response::new().add_attribute("action", "update_metadata_extension"))
    }

    fn set_withdraw_address(
        &self,
        deps: DepsMut,
        sender: &Addr,
        address: String,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        cw_ownable::assert_owner(deps.storage, sender)?;
        deps.api.addr_validate(&address)?;
        let config = Cw721Config::<
            TMetadataExtension,
            TCustomResponseMessage,
            TMetadataExtensionMsg,
        >::default();
        config.withdraw_address.save(deps.storage, &address)?;
        Ok(Response::new()
            .add_attribute("action", "set_withdraw_address")
            .add_attribute("address", address))
    }

    fn remove_withdraw_address(
        &self,
        storage: &mut dyn Storage,
        sender: &Addr,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        cw_ownable::assert_owner(storage, sender)?;
        let config = Cw721Config::<
            TMetadataExtension,
            TCustomResponseMessage,
            TMetadataExtensionMsg,
        >::default();
        let address = config.withdraw_address.may_load(storage)?;
        match address {
            Some(address) => {
                config.withdraw_address.remove(storage);
                Ok(Response::new()
                    .add_attribute("action", "remove_withdraw_address")
                    .add_attribute("address", address))
            }
            None => Err(Cw721ContractError::NoWithdrawAddress {}),
        }
    }

    fn withdraw_funds(
        &self,
        storage: &mut dyn Storage,
        amount: &Coin,
    ) -> Result<Response<TCustomResponseMessage>, Cw721ContractError> {
        let withdraw_address = Cw721Config::<
            TMetadataExtension,
            TCustomResponseMessage,
            TMetadataExtensionMsg,
        >::default()
        .withdraw_address
        .may_load(storage)?;
        match withdraw_address {
            Some(address) => {
                let msg = BankMsg::Send {
                    to_address: address,
                    amount: vec![amount.clone()],
                };
                Ok(Response::new()
                    .add_message(msg)
                    .add_attribute("action", "withdraw_funds")
                    .add_attribute("amount", amount.amount.to_string())
                    .add_attribute("denom", amount.denom.to_string()))
            }
            None => Err(Cw721ContractError::NoWithdrawAddress {}),
        }
    }
}

// ------- helper cw721 functions -------
fn _transfer_nft<TMetadataExtension>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    recipient: &str,
    token_id: &str,
) -> Result<NftInfo<TMetadataExtension>, Cw721ContractError>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
{
    let config = Cw721Config::<TMetadataExtension, Empty, Empty>::default();
    let mut token = config.nft_info.load(deps.storage, token_id)?;
    // ensure we have permissions
    check_can_send(deps.as_ref(), env, info, &token)?;
    // set owner and remove existing approvals
    token.owner = deps.api.addr_validate(recipient)?;
    token.approvals = vec![];
    config.nft_info.save(deps.storage, token_id, &token)?;
    Ok(token)
}

#[allow(clippy::too_many_arguments)]
fn _update_approvals<TMetadataExtension>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    spender: &str,
    token_id: &str,
    // if add == false, remove. if add == true, remove then set with this expiration
    add: bool,
    expires: Option<Expiration>,
) -> Result<NftInfo<TMetadataExtension>, Cw721ContractError>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
{
    let config = Cw721Config::<TMetadataExtension, Empty, Empty>::default();
    let mut token = config.nft_info.load(deps.storage, token_id)?;
    // ensure we have permissions
    check_can_approve(deps.as_ref(), env, info, &token)?;

    // update the approval list (remove any for the same spender before adding)
    let spender_addr = deps.api.addr_validate(spender)?;
    token.approvals.retain(|apr| apr.spender != spender_addr);

    // only difference between approve and revoke
    if add {
        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&env.block) {
            return Err(Cw721ContractError::Expired {});
        }
        let approval = Approval {
            spender: spender_addr,
            expires,
        };
        token.approvals.push(approval);
    }

    config.nft_info.save(deps.storage, token_id, &token)?;

    Ok(token)
}

/// returns true if the sender can execute approve or reject on the contract
pub fn check_can_approve<TMetadataExtension>(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    token: &NftInfo<TMetadataExtension>,
) -> Result<(), Cw721ContractError>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
{
    // owner can approve
    if token.owner == info.sender {
        return Ok(());
    }
    // operator can approve
    let config = Cw721Config::<TMetadataExtension, Empty, Empty>::default();
    let op = config
        .operators
        .may_load(deps.storage, (&token.owner, &info.sender))?;
    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(Cw721ContractError::Ownership(OwnershipError::NotOwner))
            } else {
                Ok(())
            }
        }
        None => Err(Cw721ContractError::Ownership(OwnershipError::NotOwner)),
    }
}

/// returns true iff the sender can transfer ownership of the token
pub fn check_can_send<TMetadataExtension>(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    token: &NftInfo<TMetadataExtension>,
) -> Result<(), Cw721ContractError> {
    // owner can send
    if token.owner == info.sender {
        return Ok(());
    }

    // any non-expired token approval can send
    if token
        .approvals
        .iter()
        .any(|apr| apr.spender == info.sender && !apr.is_expired(&env.block))
    {
        return Ok(());
    }

    // operator can send
    let config = Cw721Config::<Empty, Empty, Empty>::default();
    let op = config
        .operators
        // has token owner approved/gave grant to sender for full control over owner's NFTs?
        .may_load(deps.storage, (&token.owner, &info.sender))?;

    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(Cw721ContractError::Ownership(OwnershipError::NotOwner))
            } else {
                Ok(())
            }
        }
        None => Err(Cw721ContractError::Ownership(OwnershipError::NotOwner)),
    }
}

// ------- migrate -------
pub fn migrate_version(
    storage: &mut dyn Storage,
    contradct_name: &str,
    contract_version: &str,
    response: Response,
) -> StdResult<Response> {
    let response = response
        .add_attribute("from_version", cw2::get_contract_version(storage)?.version)
        .add_attribute("to_version", contract_version);

    // update contract version
    cw2::set_contract_version(storage, contradct_name, contract_version)?;
    Ok(response)
}

pub fn migrate_minter(
    storage: &mut dyn Storage,
    api: &dyn Api,
    _env: &Env,
    msg: &Cw721MigrateMsg,
    response: Response,
) -> StdResult<Response> {
    match msg {
        Cw721MigrateMsg::WithUpdate { minter, .. } => {
            if let Some(minter) = minter {
                MINTER.initialize_owner(storage, api, Some(minter.as_str()))?;
                return Ok(response.add_attribute("creator", minter));
            }
        }
    }
    Ok(response)
}

/// Migrates only in case ownership is not present
/// !!! Important note here: !!!
/// - creator owns the contract and can update collection info
/// - minter can mint new tokens
///
/// Before v0.19.0 there were confusing naming conventions:
/// - v0.17.0: minter was replaced by cw_ownable, as a result minter is owner
/// - v0.16.0 and below: minter was stored in dedicated `minter` store (so NOT using cw_ownable at all)
pub fn migrate_legacy_minter_and_creator(
    storage: &mut dyn Storage,
    api: &dyn Api,
    _env: &Env,
    _msg: &Cw721MigrateMsg,
    response: Response,
) -> Result<Response, Cw721ContractError> {
    let minter = MINTER.item.may_load(storage)?;
    // no migration in case minter is already set
    if minter.is_some() {
        return Ok(response);
    }
    // in v0.17/18 cw_ownable::OWNERSHIP was used for minter, now it is used for creator
    let ownership_previously_used_as_minter = OwnershipStore::new("collection_minter")
        .item
        .may_load(storage)?;
    let creator_and_minter = match ownership_previously_used_as_minter {
        // v0.18 migration
        Some(ownership) => {
            // owner is used for both: creator and minter
            // since it is already set for creator, we only need to migrate minter
            let owner = ownership.owner.map(|a| a.to_string());
            MINTER.initialize_owner(storage, api, owner.as_deref())?;
            owner
        }
        // v0.17 and older migration
        None => {
            let legacy_minter_store: Item<Addr> = Item::new("minter");
            let legacy_minter = legacy_minter_store.load(storage)?;
            MINTER.initialize_owner(storage, api, Some(legacy_minter.as_str()))?;
            Some(legacy_minter.to_string())
        }
    };
    Ok(response.add_attribute("creator_and_minter", none_or(creator_and_minter.as_ref())))
}
fn none_or<T: Display>(or: Option<&T>) -> String {
    or.map_or_else(|| "none".to_string(), |or| or.to_string())
}
/// Migrates only in case collection_info is not present
pub fn migrate_legacy_collection_info(
    storage: &mut dyn Storage,
    _env: &Env,
    _msg: &Cw721MigrateMsg,
    response: Response,
) -> Result<Response, Cw721ContractError> {
    let contract = Cw721Config::<DefaultOptionMetadataExtension, Empty, Empty>::default();
    match contract.collection_info.may_load(storage)? {
        Some(_) => Ok(response),
        None => {
            // contract info is legacy collection info
            let legacy_collection_info_store: Item<cw721_016::ContractInfoResponse> =
                Item::new("nft_info");
            let legacy_collection_info = legacy_collection_info_store.load(storage)?;
            let collection_info = CollectionInfo {
                name: legacy_collection_info.name.clone(),
                symbol: legacy_collection_info.symbol.clone(),
            };
            contract.collection_info.save(storage, &collection_info)?;
            Ok(response
                .add_attribute("migrated collection name", legacy_collection_info.name)
                .add_attribute("migrated collection symbol", legacy_collection_info.symbol))
        }
    }
}
