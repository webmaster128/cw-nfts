use cosmwasm_std::{
    Addr, Api, BankMsg, Binary, Coin, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, StdResult, Storage,
};
use cw_ownable::{
    assert_owner, get_ownership, initialize_owner, update_ownership, Action, Ownership,
    OwnershipError,
};
use cw_storage_plus::Item;
use cw_utils::Expiration;

use crate::{
    error::Cw721ContractError,
    extension::{
        Cw721BaseExtensions, Cw721EmptyExtensions, Cw721Extensions, Cw721OnchainExtensions,
    },
    helpers::value_or_empty,
    msg::{CollectionInfoMsg, Cw721InstantiateMsg, Cw721MigrateMsg, NftInfoMsg},
    query::query_collection_info_and_extension,
    receiver::Cw721ReceiveMsg,
    state::{CollectionInfo, Cw721Config, NftInfo, MINTER},
    traits::{
        Cw721CustomMsg, Cw721Execute, Cw721State, FromAttributesState, StateFactory,
        ToAttributesState,
    },
    Approval, DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg, EmptyOptionalCollectionExtension,
    EmptyOptionalCollectionExtensionMsg, EmptyOptionalNftExtension, EmptyOptionalNftExtensionMsg,
};

// ------- instantiate -------
pub fn instantiate_with_version<TCollectionExtension, TCollectionExtensionMsg, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    msg: Cw721InstantiateMsg<TCollectionExtensionMsg>,
    contract_name: &str,
    contract_version: &str,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError>
where
    TCollectionExtension: Cw721State + ToAttributesState + FromAttributesState,
    TCollectionExtensionMsg: Cw721CustomMsg + StateFactory<TCollectionExtension>,
{
    cw2::set_contract_version(deps.storage, contract_name, contract_version)?;
    instantiate(deps, env, info, msg)
}

pub fn instantiate<TCollectionExtension, TCollectionExtensionMsg, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    msg: Cw721InstantiateMsg<TCollectionExtensionMsg>,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError>
where
    TCollectionExtension: Cw721State + ToAttributesState + FromAttributesState,
    TCollectionExtensionMsg: Cw721CustomMsg + StateFactory<TCollectionExtension>,
{
    let config = Cw721Config::<Option<Empty>>::default();

    // ---- update collection info before(!) creator and minter is set ----
    let collection_metadata_msg = CollectionInfoMsg {
        name: Some(msg.name),
        symbol: Some(msg.symbol),
        extension: msg.collection_info_extension,
    };
    let collection_info = collection_metadata_msg.create(deps.as_ref(), env, info.into(), None)?;
    let extension_attributes = collection_info.extension.to_attributes_state()?;
    let collection_info = collection_info.into();
    config
        .collection_info
        .save(deps.storage, &collection_info)?;
    for attr in extension_attributes {
        config
            .collection_extension
            .save(deps.storage, attr.key.clone(), &attr)?;
    }

    // ---- set minter and creator ----
    // use info.sender if None is passed
    let minter: &str = match msg.minter.as_deref() {
        Some(minter) => minter,
        None => info.sender.as_str(),
    };
    initialize_minter(deps.storage, deps.api, Some(minter))?;

    // use info.sender if None is passed
    let creator: &str = match msg.creator.as_deref() {
        Some(creator) => creator,
        None => info.sender.as_str(),
    };
    initialize_creator(deps.storage, deps.api, Some(creator))?;

    if let Some(withdraw_address) = msg.withdraw_address.clone() {
        let creator = deps.api.addr_validate(creator)?;
        set_withdraw_address::<TCustomResponseMsg>(deps, &creator, withdraw_address)?;
    }

    Ok(Response::default()
        .add_attribute("minter", minter)
        .add_attribute("creator", creator))
}

// ------- helper cw721 functions -------
pub fn initialize_creator(
    storage: &mut dyn Storage,
    api: &dyn Api,
    creator: Option<&str>,
) -> StdResult<Ownership<Addr>> {
    initialize_owner(storage, api, creator)
}

pub fn initialize_minter(
    storage: &mut dyn Storage,
    api: &dyn Api,
    minter: Option<&str>,
) -> StdResult<Ownership<Addr>> {
    MINTER.initialize_owner(storage, api, minter)
}

pub fn transfer_nft<TNftExtension>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    recipient: &str,
    token_id: &str,
) -> Result<NftInfo<TNftExtension>, Cw721ContractError>
where
    TNftExtension: Cw721State,
{
    let config = Cw721Config::<TNftExtension>::default();
    let mut token = config.nft_info.load(deps.storage, token_id)?;
    // ensure we have permissions
    check_can_send(deps.as_ref(), env, info.sender.as_str(), &token)?;
    // set owner and remove existing approvals
    token.owner = deps.api.addr_validate(recipient)?;
    token.approvals = vec![];
    config.nft_info.save(deps.storage, token_id, &token)?;
    Ok(token)
}

pub fn send_nft<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    contract: String,
    token_id: String,
    msg: Binary,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    // Transfer token
    transfer_nft::<TNftExtension>(deps, env, info, &contract, &token_id)?;

    let send = Cw721ReceiveMsg {
        sender: info.sender.to_string(),
        token_id: token_id.clone(),
        msg,
    };

    // Send message
    Ok(Response::new()
        .add_message(send.into_cosmos_msg(contract.clone())?)
        .add_attribute("action", "send_nft")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("recipient", contract)
        .add_attribute("token_id", token_id))
}

pub fn approve<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    spender: String,
    token_id: String,
    expires: Option<Expiration>,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    update_approvals::<TNftExtension>(deps, env, info, &spender, &token_id, true, expires)?;

    Ok(Response::new()
        .add_attribute("action", "approve")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("spender", spender)
        .add_attribute("token_id", token_id))
}

#[allow(clippy::too_many_arguments)]
pub fn update_approvals<TNftExtension>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    spender: &str,
    token_id: &str,
    // if add == false, remove. if add == true, remove then set with this expiration
    add: bool,
    expires: Option<Expiration>,
) -> Result<NftInfo<TNftExtension>, Cw721ContractError>
where
    TNftExtension: Cw721State,
{
    let config = Cw721Config::<TNftExtension>::default();
    let mut token = config.nft_info.load(deps.storage, token_id)?;
    // ensure we have permissions
    check_can_approve(deps.as_ref(), env, info.sender.as_str(), &token)?;

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

pub fn revoke<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    spender: String,
    token_id: String,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError>
where
    TNftExtension: Cw721State,
{
    update_approvals::<TNftExtension>(deps, env, info, &spender, &token_id, false, None)?;

    Ok(Response::new()
        .add_attribute("action", "revoke")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("spender", spender)
        .add_attribute("token_id", token_id))
}

pub fn approve_all<TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    operator: String,
    expires: Option<Expiration>,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
    // reject expired data as invalid
    let expires = expires.unwrap_or_default();
    if expires.is_expired(&env.block) {
        return Err(Cw721ContractError::Expired {});
    }

    // set the operator for us
    let operator_addr = deps.api.addr_validate(&operator)?;
    let config = Cw721Config::<Option<Empty>>::default();
    config
        .operators
        // stores info.sender as key (=granter, NFT owner) and operator as value (operator only(!) has control over NFTs of granter)
        // check is done in `check_can_send()`
        .save(deps.storage, (&info.sender, &operator_addr), &expires)?;

    Ok(Response::new()
        .add_attribute("action", "approve_all")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("operator", operator))
}

pub fn revoke_all<TCustomResponseMsg>(
    deps: DepsMut,
    _env: &Env,
    info: &MessageInfo,
    operator: String,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
    let operator_addr = deps.api.addr_validate(&operator)?;
    let config = Cw721Config::<Option<Empty>>::default();
    config
        .operators
        .remove(deps.storage, (&info.sender, &operator_addr));

    Ok(Response::new()
        .add_attribute("action", "revoke_all")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("operator", operator))
}

pub fn burn_nft<TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    token_id: String,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
    let config = Cw721Config::<Option<Empty>>::default();
    let token = config.nft_info.load(deps.storage, &token_id)?;
    check_can_send(deps.as_ref(), env, info.sender.as_str(), &token)?;

    config.nft_info.remove(deps.storage, &token_id)?;
    config.decrement_tokens(deps.storage)?;

    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("token_id", token_id))
}

pub fn update_collection_info<TCollectionExtension, TCollectionExtensionMsg, TCustomResponseMsg>(
    deps: DepsMut,
    info: Option<&MessageInfo>,
    env: &Env,
    msg: CollectionInfoMsg<TCollectionExtensionMsg>,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError>
where
    TCollectionExtension: Cw721State + ToAttributesState + FromAttributesState,
    TCollectionExtensionMsg: Cw721CustomMsg + StateFactory<TCollectionExtension>,
    TCustomResponseMsg: CustomMsg,
{
    let config = Cw721Config::<Option<Empty>>::default();
    let current = query_collection_info_and_extension::<TCollectionExtension>(deps.as_ref())?;
    let collection_info = msg.create(deps.as_ref(), env, info, Some(&current))?;
    let extension_attributes = collection_info.extension.to_attributes_state()?;
    config
        .collection_info
        .save(deps.storage, &collection_info.into())?;
    for attr in extension_attributes {
        config
            .collection_extension
            .save(deps.storage, attr.key.clone(), &attr)?;
    }

    let response = Response::new().add_attribute("action", "update_collection_info");
    if let Some(info) = info {
        Ok(response.add_attribute("sender", info.sender.to_string()))
    } else {
        Ok(response)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn mint<TNftExtension, TNftExtensionMsg, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    token_id: String,
    owner: String,
    token_uri: Option<String>,
    extension: TNftExtensionMsg,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError>
where
    TNftExtension: Cw721State,
    TNftExtensionMsg: Cw721CustomMsg + StateFactory<TNftExtension>,
    TCustomResponseMsg: CustomMsg,
{
    // create the token
    let token_msg = NftInfoMsg {
        owner: owner.clone(),
        approvals: vec![],
        token_uri: token_uri.clone(),
        extension,
    };
    let token = token_msg.create(deps.as_ref(), env, info.into(), None)?;
    let config = Cw721Config::<TNftExtension>::default();
    config
        .nft_info
        .update(deps.storage, &token_id, |old| match old {
            Some(_) => Err(Cw721ContractError::Claimed {}),
            None => Ok(token),
        })?;

    config.increment_tokens(deps.storage)?;

    let mut res = Response::new()
        .add_attribute("action", "mint")
        .add_attribute("minter", info.sender.to_string())
        .add_attribute("owner", owner)
        .add_attribute("token_id", token_id);
    if let Some(token_uri) = token_uri {
        res = res.add_attribute("token_uri", value_or_empty(&token_uri));
    }
    Ok(res)
}

pub fn update_minter_ownership<TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    action: Action,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
    let ownership = MINTER.update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::new()
        .add_attribute("update_minter_ownership", info.sender.to_string())
        .add_attributes(ownership.into_attributes()))
}

pub fn update_creator_ownership<TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    action: Action,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
    let ownership = update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::new()
        .add_attribute("update_creator_ownership", info.sender.to_string())
        .add_attributes(ownership.into_attributes()))
}

/// The creator is the only one eligible to update NFT's token uri and onchain metadata (`NftInfo.extension`).
/// NOTE: approvals and owner are not affected by this call, since they belong to the NFT owner.
pub fn update_nft_info<TNftExtension, TNftExtensionMsg, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: Option<&MessageInfo>,
    token_id: String,
    token_uri: Option<String>,
    msg: TNftExtensionMsg,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError>
where
    TNftExtension: Cw721State,
    TNftExtensionMsg: Cw721CustomMsg + StateFactory<TNftExtension>,
    TCustomResponseMsg: CustomMsg,
{
    let contract = Cw721Config::<TNftExtension>::default();
    let current_nft_info = contract.nft_info.load(deps.storage, &token_id)?;
    let nft_info_msg = NftInfoMsg {
        owner: current_nft_info.owner.to_string(),
        approvals: current_nft_info.approvals.clone(),
        token_uri,
        extension: msg,
    };
    let updated = nft_info_msg.create(deps.as_ref(), env, info, Some(&current_nft_info))?;
    contract.nft_info.save(deps.storage, &token_id, &updated)?;
    Ok(Response::new()
        .add_attribute("action", "update_nft_info")
        .add_attribute("token_id", token_id))
}

pub fn set_withdraw_address<TCustomResponseMsg>(
    deps: DepsMut,
    sender: &Addr,
    address: String,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
    assert_owner(deps.storage, sender)?;
    deps.api.addr_validate(&address)?;
    let config = Cw721Config::<Option<Empty>>::default();
    config.withdraw_address.save(deps.storage, &address)?;
    Ok(Response::new()
        .add_attribute("action", "set_withdraw_address")
        .add_attribute("address", address))
}

pub fn remove_withdraw_address<TCustomResponseMsg>(
    storage: &mut dyn Storage,
    sender: &Addr,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
    assert_owner(storage, sender)?;
    let config = Cw721Config::<Option<Empty>>::default();
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

pub fn withdraw_funds<TCustomResponseMsg>(
    storage: &mut dyn Storage,
    amount: &Coin,
) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
    let withdraw_address = Cw721Config::<Option<Empty>>::default()
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

/// returns true if the sender can execute approve or reject on the contract
pub fn check_can_approve<TNftExtension>(
    deps: Deps,
    env: &Env,
    sender: &str,
    token: &NftInfo<TNftExtension>,
) -> Result<(), Cw721ContractError>
where
    TNftExtension: Cw721State,
{
    let sender = deps.api.addr_validate(sender)?;
    // owner can approve
    if token.owner == sender {
        return Ok(());
    }
    // operator can approve
    let config = Cw721Config::<TNftExtension>::default();
    let op = config
        .operators
        .may_load(deps.storage, (&token.owner, &sender))?;
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

/// returns true if the sender can transfer ownership of the token
pub fn check_can_send<TNftExtension>(
    deps: Deps,
    env: &Env,
    sender: &str,
    token: &NftInfo<TNftExtension>,
) -> Result<(), Cw721ContractError> {
    let sender = deps.api.addr_validate(sender)?;
    // owner can send
    if token.owner == sender {
        return Ok(());
    }

    // any non-expired token approval can send
    if token
        .approvals
        .iter()
        .any(|apr| apr.spender == sender && !apr.is_expired(&env.block))
    {
        return Ok(());
    }

    // operator can send
    let config = Cw721Config::<Option<Empty>>::default();
    let op = config
        .operators
        // has token owner approved/gave grant to sender for full control over owner's NFTs?
        .may_load(deps.storage, (&token.owner, &sender))?;

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

pub fn assert_minter(storage: &dyn Storage, sender: &Addr) -> Result<(), Cw721ContractError> {
    if MINTER.assert_owner(storage, sender).is_err() {
        return Err(Cw721ContractError::NotMinter {});
    }
    Ok(())
}

pub fn assert_creator(storage: &dyn Storage, sender: &Addr) -> Result<(), Cw721ContractError> {
    if assert_owner(storage, sender).is_err() {
        return Err(Cw721ContractError::NotCreator {});
    }
    Ok(())
}

// ------- migrate -------
pub fn migrate(
    deps: DepsMut,
    env: Env,
    msg: Cw721MigrateMsg,
    contract_name: &str,
    contract_version: &str,
) -> Result<Response, Cw721ContractError> {
    let response = Response::<Empty>::default();
    // first migrate legacy data ...
    let response = migrate_legacy_minter_and_creator(deps.storage, deps.api, &env, &msg, response)?;
    let response = migrate_legacy_collection_info(deps.storage, &env, &msg, response)?;
    // ... then migrate
    let response = migrate_version(deps.storage, contract_name, contract_version, response)?;
    // ... and update creator and minter AFTER legacy migration
    let response = migrate_creator(deps.storage, deps.api, &env, &msg, response)?;
    let response = migrate_minter(deps.storage, deps.api, &env, &msg, response)?;
    Ok(response)
}

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

pub fn migrate_creator(
    storage: &mut dyn Storage,
    api: &dyn Api,
    _env: &Env,
    msg: &Cw721MigrateMsg,
    response: Response,
) -> StdResult<Response> {
    match msg {
        Cw721MigrateMsg::WithUpdate { creator, .. } => {
            if let Some(creator) = creator {
                initialize_owner(storage, api, Some(creator.as_str()))?;
                return Ok(response.add_attribute("creator", creator));
            }
        }
    }
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
                return Ok(response.add_attribute("minter", minter));
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
    let creator_and_minter = match get_ownership(storage) {
        // v0.17/18 ownership migration
        Ok(ownership) => {
            // owner is used for both: creator and minter
            // since it is already set for creator, we only need to migrate minter
            let owner = ownership.owner.map(|a| a.to_string());
            MINTER.initialize_owner(storage, api, owner.as_deref())?;
            owner
        }
        // migration below v0.17
        Err(StdError::NotFound { .. }) => {
            let legacy_minter_store: Item<Addr> = Item::new("minter");
            let legacy_minter = legacy_minter_store.load(storage)?;
            MINTER.initialize_owner(storage, api, Some(legacy_minter.as_str()))?;
            initialize_owner(storage, api, Some(legacy_minter.as_str()))?;
            Some(legacy_minter.to_string())
        }
        // store access failed
        Err(e) => return Err(e.into()),
    };
    Ok(response.add_attribute(
        "creator_and_minter",
        creator_and_minter.unwrap_or("none".to_string()),
    ))
}

/// Migrates only in case collection_info is not present
pub fn migrate_legacy_collection_info(
    storage: &mut dyn Storage,
    env: &Env,
    _msg: &Cw721MigrateMsg,
    response: Response,
) -> Result<Response, Cw721ContractError> {
    let contract = Cw721Config::<Empty>::default();
    match contract.collection_info.may_load(storage)? {
        Some(_) => Ok(response),
        None => {
            // contract info = legacy collection info
            let legacy_collection_info_store: Item<cw721_016::ContractInfoResponse> =
                Item::new("nft_info");
            let legacy_collection_info = legacy_collection_info_store.load(storage)?;
            let collection_info = CollectionInfo {
                name: legacy_collection_info.name.clone(),
                symbol: legacy_collection_info.symbol.clone(),
                updated_at: env.block.time,
            };
            contract.collection_info.save(storage, &collection_info)?;
            Ok(response
                .add_attribute("migrated collection name", legacy_collection_info.name)
                .add_attribute("migrated collection symbol", legacy_collection_info.symbol))
        }
    }
}

impl<'a>
    Cw721Execute<
        DefaultOptionalNftExtension,
        DefaultOptionalNftExtensionMsg,
        DefaultOptionalCollectionExtension,
        DefaultOptionalCollectionExtensionMsg,
        Empty,
        Empty,
    > for Cw721OnchainExtensions<'a>
{
}

impl<'a>
    Cw721Execute<
        EmptyOptionalNftExtension,
        EmptyOptionalNftExtensionMsg,
        DefaultOptionalCollectionExtension,
        DefaultOptionalCollectionExtensionMsg,
        Empty,
        Empty,
    > for Cw721BaseExtensions<'a>
{
}

impl<'a>
    Cw721Execute<
        EmptyOptionalNftExtension,
        EmptyOptionalNftExtensionMsg,
        EmptyOptionalCollectionExtension,
        EmptyOptionalCollectionExtensionMsg,
        Empty,
        Empty,
    > for Cw721EmptyExtensions<'a>
{
}

impl<
        'a,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        TExtensionMsg,
        TExtensionQueryMsg,
        TCustomResponseMsg,
    >
    Cw721Execute<
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        TExtensionMsg,
        TCustomResponseMsg,
    >
    for Cw721Extensions<
        'a,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        TExtensionMsg,
        TExtensionQueryMsg,
        TCustomResponseMsg,
    >
where
    TNftExtension: Cw721State,
    TNftExtensionMsg: Cw721CustomMsg + StateFactory<TNftExtension>,
    TCollectionExtension: Cw721State + ToAttributesState + FromAttributesState,
    TCollectionExtensionMsg: Cw721CustomMsg + StateFactory<TCollectionExtension>,
    TCustomResponseMsg: CustomMsg,
{
}
