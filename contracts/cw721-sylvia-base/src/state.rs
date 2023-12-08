use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Empty};
use cw_ownable::Expiration;
use cw_storage_plus::{Index, IndexList, MultiIndex};

#[cw_serde]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

impl Approval {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

#[cw_serde]
pub struct TokenInfo {
    /// The owner of the newly minted NFT
    pub owner: Addr,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,

    pub extension: Empty,
}

pub fn token_owner_idx(_pk: &[u8], d: &TokenInfo) -> Addr {
    d.owner.clone()
}

/// Indexed map for NFT tokens by owner
pub struct TokenIndexes<'a> {
    pub owner: MultiIndex<'a, Addr, TokenInfo, String>,
}
impl<'a> IndexList<TokenInfo> for TokenIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ (dyn Index<TokenInfo> + '_)> + '_> {
        let v: Vec<&dyn Index<TokenInfo>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}
