use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Uint128};
use cw1155::Expiration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The minter is the only one who can create new tokens.
    /// This is designed for a base token platform that is controlled by an external program or
    /// contract.
    pub minter: String,
}

/// This is like Cw1155ExecuteMsg but we add a Mint command for a minter
/// to make this stand-alone. You will likely want to remove mint and
/// use other control logic in any contract that inherits this.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg<T> {
    /// SendFrom is a base message to move tokens,
    /// if `env.sender` is the owner or has sufficient pre-approval.
    SendFrom {
        from: String,
        /// If `to` is not contract, `msg` should be `None`
        to: String,
        token_id: String,
        value: Uint128,
        /// `None` means don't call the receiver interface
        msg: Option<Binary>,
    },
    /// BatchSendFrom is a base message to move multiple types of tokens in batch,
    /// if `env.sender` is the owner or has sufficient pre-approval.
    BatchSendFrom {
        from: String,
        /// if `to` is not contract, `msg` should be `None`
        to: String,
        batch: Vec<(String, Uint128)>,
        /// `None` means don't call the receiver interface
        msg: Option<Binary>,
    },
    /// Burn is a base message to burn tokens.
    Burn {
        from: String,
        token_id: String,
        value: Uint128,
    },
    /// BatchBurn is a base message to burn multiple types of tokens in batch.
    BatchBurn {
        from: String,
        batch: Vec<(String, Uint128)>,
    },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll { operator: String },

    /// Mint a new NFT, can only be called by the contract minter
    Mint(MintMsg<T>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintMsg<T> {
    pub token_id: String,
    /// The owner of the newly minted tokens
    pub to: String,
    /// The amount of the newly minted tokens
    pub value: Uint128,

    /// Only first mint can set `token_uri` and `extension`
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
    /// Any custom extension used by this contract
    pub extension: Option<T>,
}
