use cosmwasm_schema::cw_serde;

/// Shows who can mint these tokens
#[cw_serde]
pub struct MinterResponse {
    pub minter: Option<String>,
}
