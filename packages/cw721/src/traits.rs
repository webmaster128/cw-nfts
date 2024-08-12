use std::fmt::Debug;

use cosmwasm_std::{
    to_json_binary, Addr, Api, Binary, Coin, CosmosMsg, CustomMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, QuerierWrapper, Response, StdResult, Storage, WasmMsg, WasmQuery,
};
use cw_ownable::{Action, Ownership};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};

#[allow(deprecated)]
use crate::{
    error::Cw721ContractError,
    execute::{
        approve, approve_all, burn_nft, initialize_creator, initialize_minter, instantiate,
        instantiate_with_version, migrate, mint, remove_withdraw_address, revoke, revoke_all,
        send_nft, set_withdraw_address, transfer_nft, update_collection_info,
        update_creator_ownership, update_minter_ownership, update_nft_info, withdraw_funds,
    },
    msg::{
        AllNftInfoResponse, ApprovalResponse, ApprovalsResponse,
        CollectionInfoAndExtensionResponse, CollectionInfoMsg, Cw721ExecuteMsg,
        Cw721InstantiateMsg, Cw721MigrateMsg, Cw721QueryMsg, MinterResponse, NftInfoResponse,
        NumTokensResponse, OperatorResponse, OperatorsResponse, OwnerOfResponse, TokensResponse,
    },
    query::{
        query_all_nft_info, query_all_tokens, query_approval, query_approvals,
        query_collection_extension_attributes, query_collection_info,
        query_collection_info_and_extension, query_creator_ownership, query_minter,
        query_minter_ownership, query_nft_info, query_num_tokens, query_operator, query_operators,
        query_owner_of, query_tokens, query_withdraw_address,
    },
    state::CollectionInfo,
    Attribute,
};
use crate::{
    msg::AllInfoResponse,
    query::{query_all_info, query_nft_by_extension},
    Approval,
};

/// This is an exact copy of `CustomMsg`, since implementing a trait for a type from another crate is not possible.
///
/// Possible:
/// `impl<T> Cw721CustomMsg for Option<T> where T: Cw721CustomMsg {}`
///
/// Not possible:
/// `impl<T> CustomMsg for Option<T> where T: CustomMsg {}`
///
/// This will be removed once the `CustomMsg` trait is moved to the `cosmwasm_std` crate: https://github.com/CosmWasm/cosmwasm/issues/2056
pub trait Cw721CustomMsg: Serialize + Clone + Debug + PartialEq + JsonSchema {}

pub trait Cw721State: Serialize + DeserializeOwned + Clone + Debug {}

impl Cw721State for Empty {}
impl<T> Cw721State for Option<T> where T: Cw721State {}

impl Cw721CustomMsg for Empty {}
impl<T> Cw721CustomMsg for Option<T> where T: Cw721CustomMsg {}

/// e.g. for checking whether an NFT has specific traits (metadata).
pub trait Contains {
    fn contains(&self, other: &Self) -> bool;
}

pub trait StateFactory<TState> {
    fn create(
        &self,
        deps: Option<Deps>,
        env: Option<&Env>,
        info: Option<&MessageInfo>,
        current: Option<&TState>,
    ) -> Result<TState, Cw721ContractError>;
    fn validate(
        &self,
        deps: Option<Deps>,
        env: Option<&Env>,
        info: Option<&MessageInfo>,
        current: Option<&TState>,
    ) -> Result<(), Cw721ContractError>;
}

impl StateFactory<Empty> for Empty {
    fn create(
        &self,
        _deps: Option<Deps>,
        _env: Option<&Env>,
        _info: Option<&MessageInfo>,
        _current: Option<&Empty>,
    ) -> Result<Empty, Cw721ContractError> {
        Ok(Empty {})
    }

    fn validate(
        &self,
        _deps: Option<Deps>,
        _env: Option<&Env>,
        _info: Option<&MessageInfo>,
        _current: Option<&Empty>,
    ) -> Result<(), Cw721ContractError> {
        Ok(())
    }
}

pub trait ToAttributesState {
    fn to_attributes_state(&self) -> Result<Vec<Attribute>, Cw721ContractError>;
}

impl<T> ToAttributesState for Option<T>
where
    T: ToAttributesState,
{
    fn to_attributes_state(&self) -> Result<Vec<Attribute>, Cw721ContractError> {
        match self {
            Some(inner) => inner.to_attributes_state(),
            None => Ok(vec![]),
        }
    }
}

pub trait FromAttributesState: Sized {
    fn from_attributes_state(value: &[Attribute]) -> Result<Self, Cw721ContractError>;
}

impl<T> FromAttributesState for Option<T>
where
    T: FromAttributesState,
{
    fn from_attributes_state(value: &[Attribute]) -> Result<Self, Cw721ContractError> {
        if value.is_empty() {
            Ok(None)
        } else {
            T::from_attributes_state(value).map(Some)
        }
    }
}

/// Trait with generic onchain nft and collection extensions used to execute the contract logic and contains default implementations for all messages.
pub trait Cw721Execute<
    // NftInfo extension (onchain metadata).
    TNftExtension,
    // NftInfo extension msg for onchain metadata.
    TNftExtensionMsg,
    // CollectionInfo extension (onchain attributes).
    TCollectionExtension,
    // CollectionInfo extension msg for onchain collection attributes.
    TCollectionExtensionMsg,
    // Custom extension msg for custom contract logic. Default implementation is a no-op.
    TExtensionMsg,
    // Defines for `CosmosMsg::Custom<T>` in response. Barely used, so `Empty` can be used.
    TCustomResponseMsg,
> where
    TNftExtension: Cw721State,
    TNftExtensionMsg: Cw721CustomMsg + StateFactory<TNftExtension>,
    TCollectionExtension: Cw721State + ToAttributesState + FromAttributesState,
    TCollectionExtensionMsg: Cw721CustomMsg + StateFactory<TCollectionExtension>,
    TCustomResponseMsg: CustomMsg,
{
    fn instantiate_with_version(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        msg: Cw721InstantiateMsg<TCollectionExtensionMsg>,
        contract_name: &str,
        contract_version: &str,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        instantiate_with_version(deps, env, info, msg, contract_name, contract_version)
    }

    fn instantiate(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        msg: Cw721InstantiateMsg<TCollectionExtensionMsg>,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        instantiate(deps, env, info, msg)
    }

    fn execute(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        msg: Cw721ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtensionMsg>,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        match msg {
            Cw721ExecuteMsg::UpdateCollectionInfo { collection_info } => {
                self.update_collection_info(deps, info.into(), env.into(), collection_info)
            }
            Cw721ExecuteMsg::Mint {
                token_id,
                owner,
                token_uri,
                extension,
            } => self.mint(deps, env, info, token_id, owner, token_uri, extension),
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
            #[allow(deprecated)]
            Cw721ExecuteMsg::UpdateOwnership(action) => {
                self.update_minter_ownership(deps.api, deps.storage, env, info, action)
            }
            Cw721ExecuteMsg::UpdateMinterOwnership(action) => {
                self.update_minter_ownership(deps.api, deps.storage, env, info, action)
            }
            Cw721ExecuteMsg::UpdateCreatorOwnership(action) => {
                self.update_creator_ownership(deps.api, deps.storage, env, info, action)
            }
            #[allow(deprecated)]
            Cw721ExecuteMsg::UpdateExtension { msg } => {
                self.execute_extension(deps, env, info, msg)
            }
            Cw721ExecuteMsg::UpdateNftInfo {
                token_id,
                token_uri,
                extension,
            } => self.update_nft_info(deps, env, info, token_id, token_uri, extension),
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
        migrate(deps, env, msg, contract_name, contract_version)
    }

    // ------- ERC721-based functions -------
    fn transfer_nft(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        recipient: String,
        token_id: String,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        transfer_nft::<TNftExtension>(deps, env, info, &recipient, &token_id)?;

        Ok(Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", info.sender.to_string())
            .add_attribute("recipient", recipient)
            .add_attribute("token_id", token_id))
    }

    fn send_nft(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        contract: String,
        token_id: String,
        msg: Binary,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        send_nft::<TNftExtension, TCustomResponseMsg>(deps, env, info, contract, token_id, msg)
    }

    fn approve(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        approve::<TNftExtension, TCustomResponseMsg>(deps, env, info, spender, token_id, expires)
    }

    fn revoke(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        spender: String,
        token_id: String,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        revoke::<TNftExtension, TCustomResponseMsg>(deps, env, info, spender, token_id)
    }

    fn approve_all(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        operator: String,
        expires: Option<Expiration>,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        approve_all::<TCustomResponseMsg>(deps, env, info, operator, expires)
    }

    fn revoke_all(
        &self,
        deps: DepsMut,
        _env: &Env,
        info: &MessageInfo,
        operator: String,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        revoke_all::<TCustomResponseMsg>(deps, _env, info, operator)
    }

    fn burn_nft(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        token_id: String,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        burn_nft::<TCustomResponseMsg>(deps, env, info, token_id)
    }

    // ------- opionated cw721 functions -------
    fn initialize_creator(
        &self,
        storage: &mut dyn Storage,
        api: &dyn Api,
        creator: Option<&str>,
    ) -> StdResult<Ownership<Addr>> {
        initialize_creator(storage, api, creator)
    }

    fn initialize_minter(
        &self,
        storage: &mut dyn Storage,
        api: &dyn Api,
        minter: Option<&str>,
    ) -> StdResult<Ownership<Addr>> {
        initialize_minter(storage, api, minter)
    }

    fn update_collection_info(
        &self,
        deps: DepsMut,
        info: Option<&MessageInfo>,
        env: Option<&Env>,
        msg: CollectionInfoMsg<TCollectionExtensionMsg>,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        update_collection_info::<TCollectionExtension, TCollectionExtensionMsg, TCustomResponseMsg>(
            deps, info, env, msg,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn mint(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        token_id: String,
        owner: String,
        token_uri: Option<String>,
        extension: TNftExtensionMsg,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        mint::<TNftExtension, TNftExtensionMsg, TCustomResponseMsg>(
            deps, env, info, token_id, owner, token_uri, extension,
        )
    }

    fn update_minter_ownership(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        env: &Env,
        info: &MessageInfo,
        action: Action,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        update_minter_ownership::<TCustomResponseMsg>(api, storage, env, info, action)
    }

    fn update_creator_ownership(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        env: &Env,
        info: &MessageInfo,
        action: Action,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        update_creator_ownership::<TCustomResponseMsg>(api, storage, env, info, action)
    }

    /// Custom msg execution. This is a no-op in default implementation.
    fn execute_extension(
        &self,
        _deps: DepsMut,
        _env: &Env,
        _info: &MessageInfo,
        _msg: TExtensionMsg,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        Ok(Response::default())
    }

    /// The creator is the only one eligible to update NFT's token uri and onchain metadata (`NftInfo.extension`).
    /// NOTE: approvals and owner are not affected by this call, since they belong to the NFT owner.
    fn update_nft_info(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        token_id: String,
        token_uri: Option<String>,
        msg: TNftExtensionMsg,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        update_nft_info::<TNftExtension, TNftExtensionMsg, TCustomResponseMsg>(
            deps,
            env.into(),
            info.into(),
            token_id,
            token_uri,
            msg,
        )
    }

    fn set_withdraw_address(
        &self,
        deps: DepsMut,
        sender: &Addr,
        address: String,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        set_withdraw_address::<TCustomResponseMsg>(deps, sender, address)
    }

    fn remove_withdraw_address(
        &self,
        storage: &mut dyn Storage,
        sender: &Addr,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        remove_withdraw_address::<TCustomResponseMsg>(storage, sender)
    }

    fn withdraw_funds(
        &self,
        storage: &mut dyn Storage,
        amount: &Coin,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        withdraw_funds::<TCustomResponseMsg>(storage, amount)
    }
}

/// Trait with generic onchain nft and collection extensions used to query the contract state and contains default implementations for all queries.
pub trait Cw721Query<
    // NftInfo extension (onchain metadata).
    TNftExtension,
    // CollectionInfo extension (onchain attributes).
    TCollectionExtension,
    // Custom query msg for custom contract logic. Default implementation returns an empty binary.
    TExtensionQueryMsg,
> where
    TNftExtension: Cw721State + Contains,
    TCollectionExtension: Cw721State + FromAttributesState,
    TExtensionQueryMsg: Cw721CustomMsg,
{
    fn query(
        &self,
        deps: Deps,
        env: &Env,
        msg: Cw721QueryMsg<TNftExtension, TCollectionExtension, TExtensionQueryMsg>,
    ) -> Result<Binary, Cw721ContractError> {
        match msg {
            #[allow(deprecated)]
            Cw721QueryMsg::Minter {} => Ok(to_json_binary(&self.query_minter(deps.storage)?)?),
            #[allow(deprecated)]
            Cw721QueryMsg::ContractInfo {} => Ok(to_json_binary(
                &self.query_collection_info_and_extension(deps)?,
            )?),
            Cw721QueryMsg::GetCollectionInfoAndExtension {} => Ok(to_json_binary(
                &self.query_collection_info_and_extension(deps)?,
            )?),
            Cw721QueryMsg::GetAllInfo {} => Ok(to_json_binary(&self.query_all_info(deps, env)?)?),
            Cw721QueryMsg::GetCollectionExtensionAttributes {} => Ok(to_json_binary(
                &self.query_collection_extension_attributes(deps)?,
            )?),
            Cw721QueryMsg::NftInfo { token_id } => Ok(to_json_binary(
                &self.query_nft_info(deps.storage, token_id)?,
            )?),
            Cw721QueryMsg::GetNftByExtension {
                extension,
                start_after,
                limit,
            } => Ok(to_json_binary(&self.query_nft_by_extension(
                deps.storage,
                extension,
                start_after,
                limit,
            )?)?),
            Cw721QueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => Ok(to_json_binary(&self.query_owner_of(
                deps,
                env,
                token_id,
                include_expired.unwrap_or(false),
            )?)?),
            Cw721QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            } => Ok(to_json_binary(&self.query_all_nft_info(
                deps,
                env,
                token_id,
                include_expired.unwrap_or(false),
            )?)?),
            Cw721QueryMsg::Operator {
                owner,
                operator,
                include_expired,
            } => Ok(to_json_binary(&self.query_operator(
                deps,
                env,
                owner,
                operator,
                include_expired.unwrap_or(false),
            )?)?),
            Cw721QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            } => Ok(to_json_binary(&self.query_operators(
                deps,
                env,
                owner,
                include_expired.unwrap_or(false),
                start_after,
                limit,
            )?)?),
            Cw721QueryMsg::NumTokens {} => {
                Ok(to_json_binary(&self.query_num_tokens(deps.storage)?)?)
            }
            Cw721QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            } => Ok(to_json_binary(&self.query_tokens(
                deps,
                env,
                owner,
                start_after,
                limit,
            )?)?),
            Cw721QueryMsg::AllTokens { start_after, limit } => Ok(to_json_binary(
                &self.query_all_tokens(deps, env, start_after, limit)?,
            )?),
            Cw721QueryMsg::Approval {
                token_id,
                spender,
                include_expired,
            } => Ok(to_json_binary(&self.query_approval(
                deps,
                env,
                token_id,
                spender,
                include_expired.unwrap_or(false),
            )?)?),
            Cw721QueryMsg::Approvals {
                token_id,
                include_expired,
            } => Ok(to_json_binary(&self.query_approvals(
                deps,
                env,
                token_id,
                include_expired.unwrap_or(false),
            )?)?),
            #[allow(deprecated)]
            Cw721QueryMsg::Ownership {} => {
                Ok(to_json_binary(&self.query_minter_ownership(deps.storage)?)?)
            }
            Cw721QueryMsg::GetMinterOwnership {} => {
                Ok(to_json_binary(&self.query_minter_ownership(deps.storage)?)?)
            }
            Cw721QueryMsg::GetCreatorOwnership {} => Ok(to_json_binary(
                &self.query_creator_ownership(deps.storage)?,
            )?),
            Cw721QueryMsg::Extension { msg } => self.query_extension(deps, env, msg),
            Cw721QueryMsg::GetCollectionExtension { msg } => {
                self.query_custom_collection_extension(deps, env, msg)
            }
            Cw721QueryMsg::GetWithdrawAddress {} => {
                Ok(to_json_binary(&self.query_withdraw_address(deps)?)?)
            }
        }
    }

    #[deprecated(since = "0.19.0", note = "Please use query_minter_ownership instead")]
    /// Deprecated: use query_minter_ownership instead! Will be removed in next release!
    fn query_minter(&self, storage: &dyn Storage) -> StdResult<MinterResponse> {
        #[allow(deprecated)]
        query_minter(storage)
    }

    fn query_minter_ownership(&self, storage: &dyn Storage) -> StdResult<Ownership<Addr>> {
        query_minter_ownership(storage)
    }

    fn query_creator_ownership(&self, storage: &dyn Storage) -> StdResult<Ownership<Addr>> {
        query_creator_ownership(storage)
    }

    fn query_collection_info(&self, deps: Deps) -> StdResult<CollectionInfo> {
        query_collection_info(deps.storage)
    }

    fn query_collection_extension_attributes(&self, deps: Deps) -> StdResult<Vec<Attribute>> {
        query_collection_extension_attributes(deps)
    }

    fn query_collection_info_and_extension(
        &self,
        deps: Deps,
    ) -> Result<CollectionInfoAndExtensionResponse<TCollectionExtension>, Cw721ContractError>
    where
        TCollectionExtension: FromAttributesState,
    {
        query_collection_info_and_extension(deps)
    }

    fn query_all_info(&self, deps: Deps, env: &Env) -> StdResult<AllInfoResponse> {
        query_all_info(deps, env)
    }

    fn query_num_tokens(&self, storage: &dyn Storage) -> StdResult<NumTokensResponse> {
        query_num_tokens(storage)
    }

    fn query_nft_info(
        &self,
        storage: &dyn Storage,
        token_id: String,
    ) -> StdResult<NftInfoResponse<TNftExtension>> {
        query_nft_info::<TNftExtension>(storage, token_id)
    }

    fn query_nft_by_extension(
        &self,
        storage: &dyn Storage,
        extension: TNftExtension,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<Option<Vec<NftInfoResponse<TNftExtension>>>> {
        query_nft_by_extension::<TNftExtension>(storage, extension, start_after, limit)
    }

    fn query_owner_of(
        &self,
        deps: Deps,
        env: &Env,
        token_id: String,
        include_expired_approval: bool,
    ) -> StdResult<OwnerOfResponse> {
        query_owner_of(deps, env, token_id, include_expired_approval)
    }

    /// operator returns the approval status of an operator for a given owner if exists
    fn query_operator(
        &self,
        deps: Deps,
        env: &Env,
        owner: String,
        operator: String,
        include_expired_approval: bool,
    ) -> StdResult<OperatorResponse> {
        query_operator(deps, env, owner, operator, include_expired_approval)
    }

    /// operators returns all operators owner given access to
    fn query_operators(
        &self,
        deps: Deps,
        env: &Env,
        owner: String,
        include_expired_approval: bool,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<OperatorsResponse> {
        query_operators(
            deps,
            env,
            owner,
            include_expired_approval,
            start_after,
            limit,
        )
    }

    fn query_approval(
        &self,
        deps: Deps,
        env: &Env,
        token_id: String,
        spender: String,
        include_expired_approval: bool,
    ) -> StdResult<ApprovalResponse> {
        query_approval(deps, env, token_id, deps.api.addr_validate(&spender)?, include_expired_approval)
    }

    /// approvals returns all approvals owner given access to
    fn query_approvals(
        &self,
        deps: Deps,
        env: &Env,
        token_id: String,
        include_expired_approval: bool,
    ) -> StdResult<ApprovalsResponse> {
        query_approvals(deps, env, token_id, include_expired_approval)
    }

    fn query_tokens(
        &self,
        deps: Deps,
        _env: &Env,
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        query_tokens(deps, _env, owner, start_after, limit)
    }

    fn query_all_tokens(
        &self,
        deps: Deps,
        _env: &Env,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        query_all_tokens(deps, _env, start_after, limit)
    }

    fn query_all_nft_info(
        &self,
        deps: Deps,
        env: &Env,
        token_id: String,
        include_expired_approval: bool,
    ) -> StdResult<AllNftInfoResponse<TNftExtension>> {
        query_all_nft_info::<TNftExtension>(deps, env, token_id, include_expired_approval)
    }

    /// Custom msg query. Default implementation returns an empty binary.
    fn query_extension(
        &self,
        _deps: Deps,
        _env: &Env,
        _msg: TExtensionQueryMsg,
    ) -> Result<Binary, Cw721ContractError> {
        Ok(Binary::default())
    }

    /// use GetCollectionInfoAndExtension instead.
    /// No-op / empty extension query returning empty binary, needed for inferring type parameter during compile
    ///
    /// Note: it may be extended in case there are use cases e.g. for specific NFT metadata query.
    fn query_custom_collection_extension(
        &self,
        _deps: Deps,
        _env: &Env,
        _msg: TCollectionExtension,
    ) -> Result<Binary, Cw721ContractError> {
        Ok(Binary::default())
    }

    fn query_withdraw_address(&self, deps: Deps) -> StdResult<Option<String>> {
        query_withdraw_address(deps)
    }
}

/// Generic trait with onchain nft and collection extensions used to call query and execute messages for a given CW721 addr.
pub trait Cw721Calls<
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
    TExtensionMsg,
    TExtensionQueryMsg,
> where
    TNftExtensionMsg: Cw721CustomMsg,
    TNftExtension: Cw721State,
    TCollectionExtension: Cw721State,
    TCollectionExtensionMsg: Cw721CustomMsg,
    TExtensionMsg: Cw721CustomMsg,
    TExtensionQueryMsg: Cw721CustomMsg,
{
    /// Returns the CW721 address.
    fn addr(&self) -> Addr;

    /// Executes the CW721 contract with the given message.
    fn call(
        &self,
        msg: Cw721ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtensionMsg>,
    ) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&msg)?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    /// Queries the CW721 contract with the given message.
    fn query<T: DeserializeOwned>(
        &self,
        querier: &QuerierWrapper,
        req: Cw721QueryMsg<TNftExtension, TCollectionExtension, TExtensionQueryMsg>,
    ) -> StdResult<T> {
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_json_binary(&req)?,
        }
        .into();
        querier.query(&query)
    }

    /*** queries ***/
    fn owner_of<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        token_id: T,
        include_expired: bool,
    ) -> StdResult<OwnerOfResponse> {
        let req = Cw721QueryMsg::OwnerOf {
            token_id: token_id.into(),
            include_expired: Some(include_expired),
        };
        self.query(querier, req)
    }

    fn approval<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        token_id: T,
        spender: T,
        include_expired: Option<bool>,
    ) -> StdResult<ApprovalResponse> {
        let req = Cw721QueryMsg::Approval {
            token_id: token_id.into(),
            spender: spender.into(),
            include_expired,
        };
        let res: ApprovalResponse = self.query(querier, req)?;
        Ok(res)
    }

    fn approvals<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        token_id: T,
        include_expired: Option<bool>,
    ) -> StdResult<ApprovalsResponse> {
        let req = Cw721QueryMsg::Approvals {
            token_id: token_id.into(),
            include_expired,
        };
        let res: ApprovalsResponse = self.query(querier, req)?;
        Ok(res)
    }

    fn all_operators<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        owner: T,
        include_expired: bool,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<Vec<Approval>> {
        let req = Cw721QueryMsg::AllOperators {
            owner: owner.into(),
            include_expired: Some(include_expired),
            start_after,
            limit,
        };
        let res: OperatorsResponse = self.query(querier, req)?;
        Ok(res.operators)
    }

    fn num_tokens(&self, querier: &QuerierWrapper) -> StdResult<u64> {
        let req = Cw721QueryMsg::NumTokens {};
        let res: NumTokensResponse = self.query(querier, req)?;
        Ok(res.count)
    }

    /// This is a helper to get the metadata and extension data in one call
    fn collection_info<U: DeserializeOwned>(
        &self,
        querier: &QuerierWrapper,
    ) -> StdResult<CollectionInfoAndExtensionResponse<U>> {
        let req = Cw721QueryMsg::GetCollectionInfoAndExtension {};
        self.query(querier, req)
    }

    /// With NFT onchain metadata
    fn nft_info<T: Into<String>, U: DeserializeOwned>(
        &self,
        querier: &QuerierWrapper,
        token_id: T,
    ) -> StdResult<NftInfoResponse<U>> {
        let req = Cw721QueryMsg::NftInfo {
            token_id: token_id.into(),
        };
        self.query(querier, req)
    }

    /// With NFT onchain metadata
    fn all_nft_info<T: Into<String>, U: DeserializeOwned>(
        &self,
        querier: &QuerierWrapper,
        token_id: T,
        include_expired: bool,
    ) -> StdResult<AllNftInfoResponse<U>> {
        let req = Cw721QueryMsg::AllNftInfo {
            token_id: token_id.into(),
            include_expired: Some(include_expired),
        };
        self.query(querier, req)
    }

    /// With enumerable extension
    fn tokens<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        owner: T,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        let req = Cw721QueryMsg::Tokens {
            owner: owner.into(),
            start_after,
            limit,
        };
        self.query(querier, req)
    }

    /// With enumerable extension
    fn all_tokens(
        &self,
        querier: &QuerierWrapper,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        let req = Cw721QueryMsg::AllTokens { start_after, limit };
        self.query(querier, req)
    }

    /// returns true if the contract supports the metadata extension
    fn has_metadata(&self, querier: &QuerierWrapper) -> bool {
        self.collection_info::<Empty>(querier).is_ok()
    }

    /// returns true if the contract supports the enumerable extension
    fn has_enumerable(&self, querier: &QuerierWrapper) -> bool {
        self.tokens(querier, self.addr(), None, Some(1)).is_ok()
    }
}
