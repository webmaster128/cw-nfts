use cosmwasm_std::{
    to_json_binary, Addr, Binary, BlockInfo, Deps, Empty, Env, Order, StdError, StdResult, Storage,
};
use cw_ownable::Ownership;
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, Expiration};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{
    msg::{
        AllNftInfoResponse, ApprovalResponse, ApprovalsResponse, Cw721QueryMsg, MinterResponse,
        NftInfoResponse, NumTokensResponse, OperatorResponse, OperatorsResponse, OwnerOfResponse,
        TokensResponse,
    },
    state::{Approval, CollectionInfo, Cw721Config, NftInfo, MINTER},
};

pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 1000;

pub trait Cw721Query<
    // Metadata defined in NftInfo.
    TMetadataExtension,
> where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
{
    fn query(
        &self,
        deps: Deps,
        env: Env,
        msg: Cw721QueryMsg<TMetadataExtension>,
    ) -> StdResult<Binary> {
        match msg {
            Cw721QueryMsg::Minter {} => to_json_binary(&self.query_minter(deps.storage)?),
            Cw721QueryMsg::ContractInfo {} => {
                to_json_binary(&self.query_collection_info(deps, env)?)
            }
            Cw721QueryMsg::NftInfo { token_id } => {
                to_json_binary(&self.query_nft_info(deps, env, token_id)?)
            }
            Cw721QueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => to_json_binary(&self.query_owner_of(
                deps,
                env,
                token_id,
                include_expired.unwrap_or(false),
            )?),
            Cw721QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            } => to_json_binary(&self.query_all_nft_info(
                deps,
                env,
                token_id,
                include_expired.unwrap_or(false),
            )?),
            Cw721QueryMsg::Operator {
                owner,
                operator,
                include_expired,
            } => to_json_binary(&self.query_operator(
                deps,
                env,
                owner,
                operator,
                include_expired.unwrap_or(false),
            )?),
            Cw721QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            } => to_json_binary(&self.query_operators(
                deps,
                env,
                owner,
                include_expired.unwrap_or(false),
                start_after,
                limit,
            )?),
            Cw721QueryMsg::NumTokens {} => to_json_binary(&self.query_num_tokens(deps, env)?),
            Cw721QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            } => to_json_binary(&self.query_tokens(deps, env, owner, start_after, limit)?),
            Cw721QueryMsg::AllTokens { start_after, limit } => {
                to_json_binary(&self.query_all_tokens(deps, env, start_after, limit)?)
            }
            Cw721QueryMsg::Approval {
                token_id,
                spender,
                include_expired,
            } => to_json_binary(&self.query_approval(
                deps,
                env,
                token_id,
                spender,
                include_expired.unwrap_or(false),
            )?),
            Cw721QueryMsg::Approvals {
                token_id,
                include_expired,
            } => to_json_binary(&self.query_approvals(
                deps,
                env,
                token_id,
                include_expired.unwrap_or(false),
            )?),
            Cw721QueryMsg::Ownership {} => {
                to_json_binary(&self.query_minter_ownership(deps.storage)?)
            }
            Cw721QueryMsg::Extension { msg } => {
                to_json_binary(&self.query_extension(deps, env, msg)?)
            }
            Cw721QueryMsg::GetWithdrawAddress {} => {
                to_json_binary(&self.query_withdraw_address(deps)?)
            }
        }
    }

    fn query_minter(&self, storage: &dyn Storage) -> StdResult<MinterResponse> {
        let minter = MINTER
            .get_ownership(storage)?
            .owner
            .map(|a| a.into_string());

        Ok(MinterResponse { minter })
    }

    fn query_minter_ownership(&self, storage: &dyn Storage) -> StdResult<Ownership<Addr>> {
        MINTER.get_ownership(storage)
    }

    fn query_collection_info(&self, deps: Deps, _env: Env) -> StdResult<CollectionInfo> {
        Cw721Config::<TMetadataExtension, Empty, Empty>::default()
            .collection_info
            .load(deps.storage)
    }

    fn query_num_tokens(&self, deps: Deps, _env: Env) -> StdResult<NumTokensResponse> {
        let count =
            Cw721Config::<TMetadataExtension, Empty, Empty>::default().token_count(deps.storage)?;
        Ok(NumTokensResponse { count })
    }

    fn query_nft_info(
        &self,
        deps: Deps,
        _env: Env,
        token_id: String,
    ) -> StdResult<NftInfoResponse<TMetadataExtension>> {
        let info = Cw721Config::<TMetadataExtension, Empty, Empty>::default()
            .nft_info
            .load(deps.storage, &token_id)?;
        Ok(NftInfoResponse {
            token_uri: info.token_uri,
            extension: info.extension,
        })
    }

    fn query_owner_of(
        &self,
        deps: Deps,
        env: Env,
        token_id: String,
        include_expired_approval: bool,
    ) -> StdResult<OwnerOfResponse> {
        let nft_info = Cw721Config::<TMetadataExtension, Empty, Empty>::default()
            .nft_info
            .load(deps.storage, &token_id)?;
        Ok(OwnerOfResponse {
            owner: nft_info.owner.to_string(),
            approvals: humanize_approvals(&env.block, &nft_info, include_expired_approval),
        })
    }

    /// operator returns the approval status of an operator for a given owner if exists
    fn query_operator(
        &self,
        deps: Deps,
        env: Env,
        owner: String,
        operator: String,
        include_expired_approval: bool,
    ) -> StdResult<OperatorResponse> {
        let owner_addr = deps.api.addr_validate(&owner)?;
        let operator_addr = deps.api.addr_validate(&operator)?;

        let info = Cw721Config::<TMetadataExtension, Empty, Empty>::default()
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
    fn query_operators(
        &self,
        deps: Deps,
        env: Env,
        owner: String,
        include_expired_approval: bool,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<OperatorsResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start_addr = maybe_addr(deps.api, start_after)?;
        let start = start_addr.as_ref().map(Bound::exclusive);

        let owner_addr = deps.api.addr_validate(&owner)?;
        let res: StdResult<Vec<_>> = Cw721Config::<TMetadataExtension, Empty, Empty>::default()
            .operators
            .prefix(&owner_addr)
            .range(deps.storage, start, None, Order::Ascending)
            .filter(|r| {
                include_expired_approval
                    || r.is_err()
                    || !r.as_ref().unwrap().1.is_expired(&env.block)
            })
            .take(limit)
            .map(parse_approval)
            .collect();
        Ok(OperatorsResponse { operators: res? })
    }

    fn query_approval(
        &self,
        deps: Deps,
        env: Env,
        token_id: String,
        spender: String,
        include_expired_approval: bool,
    ) -> StdResult<ApprovalResponse> {
        let token = Cw721Config::<TMetadataExtension, Empty, Empty>::default()
            .nft_info
            .load(deps.storage, &token_id)?;

        // token owner has absolute approval
        if token.owner.to_string() == spender {
            let approval = Approval {
                spender: token.owner,
                expires: Expiration::Never {},
            };
            return Ok(ApprovalResponse { approval });
        }

        let filtered: Vec<_> = token
            .approvals
            .into_iter()
            .filter(|t| t.spender.to_string() == spender)
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
    fn query_approvals(
        &self,
        deps: Deps,
        env: Env,
        token_id: String,
        include_expired_approval: bool,
    ) -> StdResult<ApprovalsResponse> {
        let token = Cw721Config::<TMetadataExtension, Empty, Empty>::default()
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

    fn query_tokens(
        &self,
        deps: Deps,
        _env: Env,
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

        let owner_addr = deps.api.addr_validate(&owner)?;
        let tokens: Vec<String> = Cw721Config::<TMetadataExtension, Empty, Empty>::default()
            .nft_info
            .idx
            .owner
            .prefix(owner_addr)
            .keys(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .collect::<StdResult<Vec<_>>>()?;

        Ok(TokensResponse { tokens })
    }

    fn query_all_tokens(
        &self,
        deps: Deps,
        _env: Env,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

        let tokens: StdResult<Vec<String>> =
            Cw721Config::<TMetadataExtension, Empty, Empty>::default()
                .nft_info
                .range(deps.storage, start, None, Order::Ascending)
                .take(limit)
                .map(|item| item.map(|(k, _)| k))
                .collect();

        Ok(TokensResponse { tokens: tokens? })
    }

    fn query_all_nft_info(
        &self,
        deps: Deps,
        env: Env,
        token_id: String,
        include_expired_approval: bool,
    ) -> StdResult<AllNftInfoResponse<TMetadataExtension>> {
        let nft_info = Cw721Config::<TMetadataExtension, Empty, Empty>::default()
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

    /// No-op returning empty Binary
    fn query_extension(
        &self,
        _deps: Deps,
        _env: Env,
        _msg: TMetadataExtension,
    ) -> StdResult<Binary> {
        Ok(Binary::default())
    }

    fn query_withdraw_address(&self, deps: Deps) -> StdResult<Option<String>> {
        Cw721Config::<TMetadataExtension, Empty, Empty>::default()
            .withdraw_address
            .may_load(deps.storage)
    }
}

pub fn parse_approval(item: StdResult<(Addr, Expiration)>) -> StdResult<Approval> {
    item.map(|(spender, expires)| Approval { spender, expires })
}

pub fn humanize_approvals<TMetadataExtension>(
    block: &BlockInfo,
    nft_info: &NftInfo<TMetadataExtension>,
    include_expired_approval: bool,
) -> Vec<Approval>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
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
