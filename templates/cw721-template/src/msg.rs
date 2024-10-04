use cosmwasm_schema::cw_serde;
use cosmwasm_std::{CustomMsg, Empty};
pub use cw721_base::msg::InstantiateMsg;

// Implements extended on-chain metadata, by default cw721 NFTs only store a
// token_uri, which is a URL to off-chain metadata (same as ERC721).
#[cw_serde]
#[derive(Default)]
pub struct MetadataExt {}

// This is the custom Execute message extension for this contract.
// Use it to implement custom functionality.
#[cw_serde]
pub enum ExecuteExt {}
impl CustomMsg for ExecuteExt {}

// This is the custom Query message type for this contract.
// Use it to implement custom query messages.
#[cw_serde]
pub enum QueryExt {}
impl CustomMsg for QueryExt {}

// This contrains default cw721 logic with extensions.
// If you don't need a particular extension, replace it with an
// `Empty` type.
pub type Cw721Contract<'a> =
    cw721_base::Cw721Contract<'a, MetadataExt, Empty, ExecuteExt, QueryExt>;

// The execute message type for this contract.
// If you don't need the Metadata and Execute extensions, you can use the
// `Empty` type.
pub type ExecuteMsg = cw721_base::ExecuteMsg<MetadataExt, ExecuteExt>;

// The query message type for this contract.
// If you don't need the QueryExt extension, you can use the
// `Empty` type.
pub type QueryMsg = cw721_base::QueryMsg<QueryExt>;
