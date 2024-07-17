use std::marker::PhantomData;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, CustomMsg, StdResult, Storage};
use cw_ownable::{OwnershipStore, OWNERSHIP_KEY};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use cw_utils::Expiration;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// - minter is stored in the contract storage using cw_ownable::OwnershipStore (same as for OWNERSHIP but with different key)
pub const MINTER: OwnershipStore = OwnershipStore::new(OWNERSHIP_KEY);

/// Default CollectionInfoExtension with RoyaltyInfo
pub type DefaultOptionMetadataExtension = Option<Metadata>;

pub struct Cw721Config<
    'a,
    // Metadata defined in NftInfo (used for mint).
    TMetadataExtension,
    // Defines for `CosmosMsg::Custom<T>` in response. Barely used, so `Empty` can be used.
    TCustomResponseMessage,
    // Message passed for updating metadata.
    TMetadataExtensionMsg,
> where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
    TMetadataExtensionMsg: CustomMsg,
{
    /// Note: replaces deprecated/legacy key "nft_info"!
    pub collection_info: Item<CollectionInfo>,
    pub token_count: Item<u64>,
    /// Stored as (granter, operator) giving operator full control over granter's account.
    /// NOTE: granter is the owner, so operator has only control for NFTs owned by granter!
    pub operators: Map<(&'a Addr, &'a Addr), Expiration>,
    pub nft_info:
        IndexedMap< &'a str, NftInfo<TMetadataExtension>, TokenIndexes<'a, TMetadataExtension>>,
    pub withdraw_address: Item<String>,

    pub(crate) _custom_response: PhantomData<TCustomResponseMessage>,
    pub(crate) _custom_execute: PhantomData<TMetadataExtensionMsg>,
}

impl<TMetadataExtension, TCustomResponseMessage, TMetadataExtensionMsg> Default
    for Cw721Config<'static, TMetadataExtension, TCustomResponseMessage, TMetadataExtensionMsg>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
    TMetadataExtensionMsg: CustomMsg,
{
    fn default() -> Self {
        Self::new(
            "collection_info", // Note: replaces deprecated/legacy key "nft_info"
            "num_tokens",
            "operators",
            "tokens",
            "tokens__owner",
            "withdraw_address",
        )
    }
}

impl<'a, TMetadataExtension, TCustomResponseMessage, TMetadataExtensionMsg>
    Cw721Config<'a, TMetadataExtension, TCustomResponseMessage, TMetadataExtensionMsg>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
    TMetadataExtensionMsg: CustomMsg,
{
    fn new(
        collection_info_key: &'static str,
        token_count_key: &'static str,
        operator_key: &'static str,
        nft_info_key: &'static str,
        nft_info_owner_key: &'static str,
        withdraw_address_key: &'static str,
    ) -> Self {
        let indexes = TokenIndexes {
            owner: MultiIndex::new(token_owner_idx, nft_info_key.clone(), nft_info_owner_key.clone()),
        };
        Self {
            collection_info: Item::new(collection_info_key.clone()),
            token_count: Item::new(token_count_key.clone()),
            operators: Map::new(operator_key.clone()),
            nft_info: IndexedMap::new(nft_info_key.clone(), indexes),
            withdraw_address: Item::new(withdraw_address_key.clone()),
            _custom_response: PhantomData,
            _custom_execute: PhantomData,
        }
    }

    pub fn token_count(&self, storage: &dyn Storage) -> StdResult<u64> {
        Ok(self.token_count.may_load(storage)?.unwrap_or_default())
    }

    pub fn increment_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? + 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }

    pub fn decrement_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? - 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }
}

pub fn token_owner_idx<TMetadataExtension>(_pk: &[u8], d: &NftInfo<TMetadataExtension>) -> Addr {
    d.owner.clone()
}

#[cw_serde]
pub struct NftInfo<TMetadataExtension> {
    /// The owner of the newly minted NFT
    pub owner: Addr,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,

    /// You can add any custom metadata here when you extend cw721-base
    pub extension: TMetadataExtension,
}

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

pub struct TokenIndexes<'a, TMetadataExtension>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
{
    pub owner: MultiIndex<'a, Addr, NftInfo<TMetadataExtension>, String>,
}

impl<'a, TMetadataExtension> IndexList<NftInfo<TMetadataExtension>>
    for TokenIndexes<'a, TMetadataExtension>
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
{
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn Index<NftInfo<TMetadataExtension>>> + '_> {
        let v: Vec<&dyn Index<NftInfo<TMetadataExtension>>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

#[cw_serde]
pub struct CollectionInfo {
    pub name: String,
    pub symbol: String,
}

// see: https://docs.opensea.io/docs/metadata-standards
#[cw_serde]
#[derive(Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}
