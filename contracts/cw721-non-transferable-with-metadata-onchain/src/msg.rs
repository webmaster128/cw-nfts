use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub name: String,
    pub symbol: String,
    pub minter: Option<String>,
    pub withdraw_address: Option<String>,
}
