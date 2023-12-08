pub mod cw721_interface {
    use cosmwasm_std::{Binary, Empty, Response, StdResult};
    use cw_utils::Expiration;
    use sylvia::cw_std::StdError;
    use sylvia::interface;
    use sylvia::types::{ExecCtx, QueryCtx};

    use crate::{
        AllNftInfoResponse, ApprovalResponse, ApprovalsResponse, ContractInfoResponse,
        NftInfoResponse, NumTokensResponse, OperatorResponse, OperatorsResponse, OwnerOfResponse,
        TokensResponse,
    };

    #[interface]
    pub trait Cw721Interface {
        type Error: From<StdError>;

        #[msg(exec)]
        fn transfer_nft(
            &self,
            ctx: ExecCtx,
            recipient: String,
            token_id: String,
        ) -> Result<Response, Self::Error>;

        #[msg(exec)]
        fn send_nft(
            &self,
            ctx: ExecCtx,
            contract: String,
            token_id: String,
            msg: Binary,
        ) -> Result<Response, Self::Error>;

        #[msg(exec)]
        fn approve(
            &self,
            ctx: ExecCtx,
            spender: String,
            token_id: String,
            expires: Option<Expiration>,
        ) -> Result<Response, Self::Error>;

        #[msg(exec)]
        fn revoke(
            &self,
            ctx: ExecCtx,
            spender: String,
            token_id: String,
        ) -> Result<Response, Self::Error>;

        #[msg(exec)]
        fn approve_all(
            &self,
            ctx: ExecCtx,
            operator: String,
            expires: Option<Expiration>,
        ) -> Result<Response, Self::Error>;

        #[msg(exec)]
        fn revoke_all(&self, ctx: ExecCtx, operator: String) -> Result<Response, Self::Error>;

        #[msg(exec)]
        fn burn(&self, ctx: ExecCtx, token_id: String) -> Result<Response, Self::Error>;

        #[msg(query)]
        fn contract_info(&self, ctx: QueryCtx) -> StdResult<ContractInfoResponse>;

        #[msg(query)]
        fn num_tokens(&self, ctx: QueryCtx) -> StdResult<NumTokensResponse>;

        #[msg(query)]
        fn nft_info(&self, ctx: QueryCtx, token_id: String) -> StdResult<NftInfoResponse<Empty>>;

        #[msg(query)]
        fn owner_of(
            &self,
            ctx: QueryCtx,
            token_id: String,
            include_expired: bool,
        ) -> StdResult<OwnerOfResponse>;

        #[msg(query)]
        fn operator(
            &self,
            ctx: QueryCtx,
            owner: String,
            operator: String,
            include_expired: bool,
        ) -> StdResult<OperatorResponse>;

        #[msg(query)]
        fn operators(
            &self,
            ctx: QueryCtx,
            owner: String,
            include_expired: bool,
            start_after: Option<String>,
            limit: Option<u32>,
        ) -> StdResult<OperatorsResponse>;

        #[msg(query)]
        fn approval(
            &self,
            ctx: QueryCtx,
            token_id: String,
            spender: String,
            include_expired: bool,
        ) -> StdResult<ApprovalResponse>;

        #[msg(query)]
        fn approvals(
            &self,
            ctx: QueryCtx,
            token_id: String,
            include_expired: bool,
        ) -> StdResult<ApprovalsResponse>;

        #[msg(query)]
        fn tokens(
            &self,
            ctx: QueryCtx,
            owner: String,
            start_after: Option<String>,
            limit: Option<u32>,
        ) -> StdResult<TokensResponse>;

        #[msg(query)]
        fn all_tokens(
            &self,
            ctx: QueryCtx,
            start_after: Option<String>,
            limit: Option<u32>,
        ) -> StdResult<TokensResponse>;

        #[msg(query)]
        fn all_nft_info(
            &self,
            ctx: QueryCtx,
            token_id: String,
            include_expired: bool,
        ) -> StdResult<AllNftInfoResponse<Empty>>;
    }
}
