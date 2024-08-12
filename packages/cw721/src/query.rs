use cosmwasm_std::{
    Addr, BlockInfo, CustomMsg, Deps, Empty, Env, Order, StdError, StdResult, Storage,
};
use cw_ownable::Ownership;
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, Expiration};

use crate::{
    error::Cw721ContractError,
    extension::{
        Cw721BaseExtensions, Cw721EmptyExtensions, Cw721Extensions, Cw721OnchainExtensions,
    },
    msg::{
        AllInfoResponse, AllNftInfoResponse, ApprovalResponse, ApprovalsResponse,
        CollectionInfoAndExtensionResponse, MinterResponse, NftInfoResponse, NumTokensResponse,
        OperatorResponse, OperatorsResponse, OwnerOfResponse, TokensResponse,
    },
    state::{
        Approval, CollectionExtensionAttributes, CollectionInfo, Cw721Config, NftInfo, CREATOR,
        MINTER,
    },
    traits::{Contains, Cw721CustomMsg, Cw721Query, Cw721State, FromAttributesState},
    DefaultOptionalCollectionExtension, DefaultOptionalNftExtension,
    EmptyOptionalCollectionExtension, EmptyOptionalNftExtension,
};

pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 1000;

pub fn parse_approval(item: StdResult<(Addr, Expiration)>) -> StdResult<Approval> {
    item.map(|(spender, expires)| Approval { spender, expires })
}

pub fn humanize_approvals<TNftExtension>(
    block: &BlockInfo,
    nft_info: &NftInfo<TNftExtension>,
    include_expired_approval: bool,
) -> Vec<Approval>
where
    TNftExtension: Cw721State,
{
    nft_info
        .approvals
        .iter()
        .filter(|apr| include_expired_approval || !apr.is_expired(block))
        .map(humanize_approval)
        .collect()
}

pub fn humanize_approval(approval: &Approval) -> Approval {
    Approval {
        spender: approval.spender.clone(),
        expires: approval.expires,
    }
}

// --- query helpers ---
#[deprecated(since = "0.19.0", note = "Please use query_minter_ownership instead")]
/// Deprecated: use query_minter_ownership instead! Will be removed in next release!
pub fn query_minter(storage: &dyn Storage) -> StdResult<MinterResponse> {
    let minter = MINTER
        .get_ownership(storage)?
        .owner
        .map(|a| a.into_string());

    Ok(MinterResponse { minter })
}

pub fn query_minter_ownership(storage: &dyn Storage) -> StdResult<Ownership<Addr>> {
    MINTER.get_ownership(storage)
}

pub fn query_creator_ownership(storage: &dyn Storage) -> StdResult<Ownership<Addr>> {
    CREATOR.get_ownership(storage)
}

pub fn query_collection_info(storage: &dyn Storage) -> StdResult<CollectionInfo> {
    let config = Cw721Config::<Option<Empty>>::default();
    config.collection_info.load(storage)
}

pub fn query_collection_extension_attributes(
    deps: Deps,
) -> StdResult<CollectionExtensionAttributes> {
    let config = Cw721Config::<Option<Empty>>::default();
    config
        .collection_extension
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv| Ok(kv?.1))
        .collect()
}

pub fn query_collection_info_and_extension<TCollectionExtension>(
    deps: Deps,
) -> Result<CollectionInfoAndExtensionResponse<TCollectionExtension>, Cw721ContractError>
where
    TCollectionExtension: Cw721State + FromAttributesState,
{
    let collection_info = query_collection_info(deps.storage)?;
    let attributes = query_collection_extension_attributes(deps)?;
    let extension = FromAttributesState::from_attributes_state(&attributes)?;
    Ok(CollectionInfoAndExtensionResponse {
        name: collection_info.name,
        symbol: collection_info.symbol,
        updated_at: collection_info.updated_at,
        extension,
    })
}

pub fn query_all_info(deps: Deps, env: &Env) -> StdResult<AllInfoResponse> {
    let collection_info = query_collection_info(deps.storage)?;
    let attributes = query_collection_extension_attributes(deps)?;
    let num_tokens = Cw721Config::<Option<Empty>>::default().token_count(deps.storage)?;
    let contract_info = deps
        .querier
        .query_wasm_contract_info(env.contract.address.clone())?;
    Ok(AllInfoResponse {
        collection_info,
        collection_extension: attributes,
        num_tokens,
        contract_info,
    })
}

pub fn query_num_tokens(storage: &dyn Storage) -> StdResult<NumTokensResponse> {
    let count = Cw721Config::<Option<Empty>>::default().token_count(storage)?;
    Ok(NumTokensResponse { count })
}

pub fn query_nft_info<TNftExtension>(
    storage: &dyn Storage,
    token_id: String,
) -> StdResult<NftInfoResponse<TNftExtension>>
where
    TNftExtension: Cw721State,
{
    let info = Cw721Config::<TNftExtension>::default()
        .nft_info
        .load(storage, &token_id)?;
    Ok(NftInfoResponse {
        token_uri: info.token_uri,
        extension: info.extension,
    })
}

pub fn query_nft_by_extension<TNftExtension>(
    storage: &dyn Storage,
    extension: TNftExtension,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Option<Vec<NftInfoResponse<TNftExtension>>>>
where
    TNftExtension: Cw721State + Contains,
{
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let nfts: Vec<Option<NftInfo<TNftExtension>>> = Cw721Config::<TNftExtension>::default()
        .nft_info
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|kv| {
            let nft = kv?.1;
            let result = if nft.extension.contains(&extension) {
                Some(nft)
            } else {
                None
            };
            Ok(result)
        })
        .collect::<StdResult<_>>()?;
    let filtered = nfts
        .iter()
        .filter_map(|n| n.clone())
        .map(|n| NftInfoResponse {
            token_uri: n.token_uri,
            extension: n.extension,
        })
        .collect::<Vec<NftInfoResponse<TNftExtension>>>();
    if filtered.is_empty() {
        Ok(None)
    } else {
        Ok(Some(filtered))
    }
}

pub fn query_owner_of(
    deps: Deps,
    env: &Env,
    token_id: String,
    include_expired_approval: bool,
) -> StdResult<OwnerOfResponse> {
    let nft_info = Cw721Config::<Option<Empty>>::default()
        .nft_info
        .load(deps.storage, &token_id)?;
    Ok(OwnerOfResponse {
        owner: nft_info.owner.to_string(),
        approvals: humanize_approvals(&env.block, &nft_info, include_expired_approval),
    })
}

/// operator returns the approval status of an operator for a given owner if exists
pub fn query_operator(
    deps: Deps,
    env: &Env,
    owner: String,
    operator: String,
    include_expired_approval: bool,
) -> StdResult<OperatorResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let operator_addr = deps.api.addr_validate(&operator)?;

    let info = Cw721Config::<Option<Empty>>::default()
        .operators
        .may_load(deps.storage, (&owner_addr, &operator_addr))?;

    if let Some(expires) = info {
        if !include_expired_approval && expires.is_expired(&env.block) {
            return Err(StdError::not_found("Approval not found"));
        }

        return Ok(OperatorResponse {
            approval: Approval {
                spender: operator_addr,
                expires,
            },
        });
    }

    Err(StdError::not_found("Approval not found"))
}

/// operators returns all operators owner given access to
pub fn query_operators(
    deps: Deps,
    env: &Env,
    owner: String,
    include_expired_approval: bool,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<OperatorsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_addr = maybe_addr(deps.api, start_after)?;
    let start = start_addr.as_ref().map(Bound::exclusive);

    let owner_addr = deps.api.addr_validate(&owner)?;
    let res: StdResult<Vec<_>> = Cw721Config::<Option<Empty>>::default()
        .operators
        .prefix(&owner_addr)
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|r| {
            include_expired_approval || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block)
        })
        .take(limit)
        .map(parse_approval)
        .collect();
    Ok(OperatorsResponse { operators: res? })
}

pub fn query_approval(
    deps: Deps,
    env: &Env,
    token_id: String,
    spender: Addr,
    include_expired_approval: bool,
) -> StdResult<ApprovalResponse> {
    let token = Cw721Config::<Option<Empty>>::default()
        .nft_info
        .load(deps.storage, &token_id)?;

    // token owner has absolute approval
    if token.owner == spender {
        let approval = Approval {
            spender: token.owner,
            expires: Expiration::Never {},
        };
        return Ok(ApprovalResponse { approval });
    }

    let filtered: Vec<_> = token
        .approvals
        .into_iter()
        .filter(|t| t.spender == spender)
        .filter(|t| include_expired_approval || !t.is_expired(&env.block))
        .map(|a| Approval {
            spender: a.spender,
            expires: a.expires,
        })
        .collect();

    if filtered.is_empty() {
        return Err(StdError::not_found("Approval not found"));
    }
    // we expect only one item
    let approval = filtered[0].clone();

    Ok(ApprovalResponse { approval })
}

/// approvals returns all approvals owner given access to
pub fn query_approvals(
    deps: Deps,
    env: &Env,
    token_id: String,
    include_expired_approval: bool,
) -> StdResult<ApprovalsResponse> {
    let token = Cw721Config::<Option<Empty>>::default()
        .nft_info
        .load(deps.storage, &token_id)?;
    let approvals: Vec<_> = token
        .approvals
        .into_iter()
        .filter(|t| include_expired_approval || !t.is_expired(&env.block))
        .map(|a| Approval {
            spender: a.spender,
            expires: a.expires,
        })
        .collect();

    Ok(ApprovalsResponse { approvals })
}

pub fn query_tokens(
    deps: Deps,
    _env: &Env,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let owner_addr = deps.api.addr_validate(&owner)?;
    let tokens: Vec<String> = Cw721Config::<Option<Empty>>::default()
        .nft_info
        .idx
        .owner
        .prefix(owner_addr)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(TokensResponse { tokens })
}

pub fn query_all_tokens(
    deps: Deps,
    _env: &Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let tokens: StdResult<Vec<String>> = Cw721Config::<Option<Empty>>::default()
        .nft_info
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(k, _)| k))
        .collect();

    Ok(TokensResponse { tokens: tokens? })
}

pub fn query_all_nft_info<TNftExtension>(
    deps: Deps,
    env: &Env,
    token_id: String,
    include_expired_approval: bool,
) -> StdResult<AllNftInfoResponse<TNftExtension>>
where
    TNftExtension: Cw721State,
{
    let nft_info = Cw721Config::<TNftExtension>::default()
        .nft_info
        .load(deps.storage, &token_id)?;
    Ok(AllNftInfoResponse {
        access: OwnerOfResponse {
            owner: nft_info.owner.to_string(),
            approvals: humanize_approvals(&env.block, &nft_info, include_expired_approval),
        },
        info: NftInfoResponse {
            token_uri: nft_info.token_uri,
            extension: nft_info.extension,
        },
    })
}

pub fn query_withdraw_address(deps: Deps) -> StdResult<Option<String>> {
    Cw721Config::<Option<Empty>>::default()
        .withdraw_address
        .may_load(deps.storage)
}

impl<'a> Cw721Query<DefaultOptionalNftExtension, DefaultOptionalCollectionExtension, Empty>
    for Cw721OnchainExtensions<'a>
{
}

impl<'a> Cw721Query<EmptyOptionalNftExtension, DefaultOptionalCollectionExtension, Empty>
    for Cw721BaseExtensions<'a>
{
}

impl<'a> Cw721Query<EmptyOptionalNftExtension, EmptyOptionalCollectionExtension, Empty>
    for Cw721EmptyExtensions<'a>
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
    > Cw721Query<TNftExtension, TCollectionExtension, TExtensionQueryMsg>
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
    TNftExtension: Cw721State + Contains,
    TNftExtensionMsg: Cw721CustomMsg,
    TCollectionExtension: Cw721State + FromAttributesState,
    TCollectionExtensionMsg: Cw721CustomMsg,
    TExtensionQueryMsg: Cw721CustomMsg,
    TCustomResponseMsg: CustomMsg,
{
}
