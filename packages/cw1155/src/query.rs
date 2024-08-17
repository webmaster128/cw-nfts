use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_utils::Expiration;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw1155QueryMsg {
    /// Query Minter.
    /// Return type: MinterResponse.
    Minter {},
    /// Returns the current balance of the given address, 0 if unset.
    /// Return type: BalanceResponse.
    Balance { owner: String, token_id: String },
    /// Returns all current balances of the given token id. Supports pagination
    /// Return type: AllBalancesResponse.
    AllBalances {
        token_id: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the current balance of the given address for a batch of tokens, 0 if unset.
    /// Return type: BatchBalanceResponse.
    BatchBalance {
        owner: String,
        token_ids: Vec<String>,
    },
    /// Total number of tokens issued for the token id
    NumTokens { token_id: String },
    /// List all operators that can access all of the owner's tokens.
    /// Return type: ApprovedForAllResponse.
    ApprovedForAll {
        owner: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Query approved status `owner` granted to `operator`.
    /// Return type: IsApprovedForAllResponse
    IsApprovedForAll { owner: String, operator: String },

    /// With MetaData Extension.
    /// Query metadata of token
    /// Return type: TokenInfoResponse.
    TokenInfo { token_id: String },

    /// With Enumerable extension.
    /// Returns all tokens owned by the given address, [] if unset.
    /// Return type: TokensResponse.
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// With Enumerable extension.
    /// Requires pagination. Lists all token_ids controlled by the contract.
    /// Return type: TokensResponse.
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AllBalancesResponse {
    pub balances: Vec<Balance>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Balance {
    pub token_id: String,
    pub owner: Addr,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BatchBalanceResponse {
    pub balances: Vec<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct NumTokensResponse {
    pub count: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: String,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ApprovedForAllResponse {
    pub operators: Vec<Approval>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct IsApprovedForAllResponse {
    pub approved: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenInfoResponse<T> {
    /// Should be a url point to a json file
    pub token_uri: Option<String>,
    /// You can add any custom metadata here when you extend cw1155-base
    pub extension: Option<T>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokensResponse {
    /// Contains all token_ids in lexicographical ordering
    /// If there are more than `limit`, use `start_from` in future queries
    /// to achieve pagination.
    pub tokens: Vec<String>,
}

/// Shows who can mint these tokens
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MinterResponse {
    pub minter: String,
}
