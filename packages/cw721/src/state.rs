use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, BlockInfo, Decimal, Deps, Empty, Env, MessageInfo,
    StdResult, Storage, Timestamp,
};
use cw_ownable::OwnershipStore;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use cw_utils::Expiration;
use serde::de::DeserializeOwned;

use crate::error::Cw721ContractError;
use crate::traits::{Contains, Cw721CustomMsg, Cw721State, FromAttributesState, ToAttributesState};
use crate::{traits::StateFactory, NftExtensionMsg};

/// Creator owns this contract and can update collection info!
/// !!! Important note here: !!!
/// - creator is stored using using cw-ownable's OWNERSHIP singleton, so it is not stored here
/// - in release v0.18.0 it was used for minter (which is confusing), but now it is used for creator
/// - minter is stored in the contract storage using cw_ownable::OwnershipStore (same as for OWNERSHIP but with different key)
pub const MINTER: OwnershipStore = OwnershipStore::new("collection_minter");

// ----------------------
// NOTE: below are max restrictions for default collection extension (CollectionExtensionResponse)
// This may be quite restrictive and may be increased in the future.
// Custom contracts may also provide different collection extension.
// Please also note, each element in the collection extension is stored as a separate `Attribute`.

/// Maximum length of the description field in the collection info.
pub const MAX_COLLECTION_DESCRIPTION_LENGTH: u32 = 512;
/// Max increase/decrease of of royalty share percentage.
pub const MAX_ROYALTY_SHARE_DELTA_PCT: u64 = 2;
/// Max royalty share percentage.
pub const MAX_ROYALTY_SHARE_PCT: u64 = 10;
// ----------------------
pub const ATTRIBUTE_DESCRIPTION: &str = "description";
pub const ATTRIBUTE_IMAGE: &str = "image";
pub const ATTRIBUTE_EXTERNAL_LINK: &str = "external_link";
pub const ATTRIBUTE_EXPLICIT_CONTENT: &str = "explicit_content";
pub const ATTRIBUTE_START_TRADING_TIME: &str = "start_trading_time";
pub const ATTRIBUTE_ROYALTY_INFO: &str = "royalty_info";
// ----------------------

pub struct Cw721Config<
    'a,
    // NftInfo extension (onchain metadata).
    TNftExtension,
> where
    TNftExtension: Cw721State,
{
    /// Note: replaces deprecated/legacy key "nft_info"!
    pub collection_info: Item<CollectionInfo>,
    pub collection_extension: Map<String, Attribute>,
    pub token_count: Item<u64>,
    /// Stored as (granter, operator) giving operator full control over granter's account.
    /// NOTE: granter is the owner, so operator has only control for NFTs owned by granter!
    pub operators: Map<(&'a Addr, &'a Addr), Expiration>,
    pub nft_info: IndexedMap<&'a str, NftInfo<TNftExtension>, TokenIndexes<'a, TNftExtension>>,
    pub withdraw_address: Item<String>,
}

impl<TNftExtension> Default for Cw721Config<'static, TNftExtension>
where
    TNftExtension: Cw721State,
{
    fn default() -> Self {
        Self::new(
            // `cw721_` prefix is added for avoiding conflicts with other contracts.
            "cw721_collection_info", // replaces deprecated/legacy key "nft_info"
            "cw721_collection_info_extension",
            "num_tokens",
            "operators",
            "tokens",
            "tokens__owner",
            "withdraw_address",
        )
    }
}

impl<'a, TNftExtension> Cw721Config<'a, TNftExtension>
where
    TNftExtension: Cw721State,
{
    fn new(
        collection_info_key: &'static str,
        collection_info_extension_key: &'static str,
        token_count_key: &'static str,
        operator_key: &'static str,
        nft_info_key: &'static str,
        nft_info_owner_key: &'static str,
        withdraw_address_key: &'static str,
    ) -> Self {
        let indexes = TokenIndexes {
            owner: MultiIndex::new(token_owner_idx, nft_info_key, nft_info_owner_key),
        };
        Self {
            collection_info: Item::new(collection_info_key),
            token_count: Item::new(token_count_key),
            operators: Map::new(operator_key),
            nft_info: IndexedMap::new(nft_info_key, indexes),
            withdraw_address: Item::new(withdraw_address_key),
            collection_extension: Map::new(collection_info_extension_key),
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

pub fn token_owner_idx<TNftExtension>(_pk: &[u8], d: &NftInfo<TNftExtension>) -> Addr {
    d.owner.clone()
}

#[cw_serde]
pub struct NftInfo<TNftExtension> {
    /// The owner of the newly minted NFT
    pub owner: Addr,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,

    /// You can add any custom metadata here when you extend cw721-base
    pub extension: TNftExtension,
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

pub struct TokenIndexes<'a, TNftExtension>
where
    TNftExtension: Cw721State,
{
    pub owner: MultiIndex<'a, Addr, NftInfo<TNftExtension>, String>,
}

impl<'a, TNftExtension> IndexList<NftInfo<TNftExtension>> for TokenIndexes<'a, TNftExtension>
where
    TNftExtension: Cw721State,
{
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn Index<NftInfo<TNftExtension>>> + '_> {
        let v: Vec<&dyn Index<NftInfo<TNftExtension>>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

#[cw_serde]
pub struct CollectionInfo {
    pub name: String,
    pub symbol: String,
    pub updated_at: Timestamp,
}

/// Explicit type equivalent to `Vec<Attribute>`, for better distinction.
pub type CollectionExtensionAttributes = Vec<Attribute>;

#[cw_serde]
pub struct Attribute {
    pub key: String,
    pub value: Binary,
}

impl Attribute {
    pub fn value<T>(&self) -> Result<T, Cw721ContractError>
    where
        T: DeserializeOwned,
    {
        Ok(from_json(&self.value)?)
    }
}

#[cw_serde]
pub struct RoyaltyInfo {
    pub payment_address: Addr,
    pub share: Decimal,
}

impl Cw721State for RoyaltyInfo {}
impl Cw721CustomMsg for RoyaltyInfo {}

impl ToAttributesState for Empty {
    fn to_attributes_state(&self) -> Result<Vec<Attribute>, Cw721ContractError> {
        Ok(vec![])
    }
}

impl ToAttributesState for RoyaltyInfo {
    fn to_attributes_state(&self) -> Result<Vec<Attribute>, Cw721ContractError> {
        Ok(vec![Attribute {
            key: ATTRIBUTE_ROYALTY_INFO.to_string(),
            value: to_json_binary(&self.clone()).unwrap(),
        }])
    }
}

impl FromAttributesState for RoyaltyInfo {
    fn from_attributes_state(attributes: &[Attribute]) -> Result<Self, Cw721ContractError> {
        let royalty_info = attributes
            .iter()
            .find(|attr| attr.key == ATTRIBUTE_ROYALTY_INFO)
            .ok_or_else(|| {
                Cw721ContractError::AttributeMissing("royalty payment address".to_string())
            })?
            .value::<RoyaltyInfo>()?;
        Ok(royalty_info)
    }
}

impl FromAttributesState for Empty {
    fn from_attributes_state(_attributes: &[Attribute]) -> Result<Self, Cw721ContractError> {
        Ok(Empty {})
    }
}

#[cw_serde]
pub struct CollectionExtension<TRoyaltyInfo> {
    pub description: String,
    pub image: String,
    pub external_link: Option<String>,
    pub explicit_content: Option<bool>,
    pub start_trading_time: Option<Timestamp>,
    pub royalty_info: Option<TRoyaltyInfo>,
}

impl Cw721State for CollectionExtension<RoyaltyInfo> {}

// see: https://docs.opensea.io/docs/metadata-standards
#[cw_serde]
#[derive(Default)]
pub struct NftExtension {
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

impl Cw721State for NftExtension {}

impl From<NftExtensionMsg> for NftExtension {
    fn from(msg: NftExtensionMsg) -> Self {
        NftExtension {
            image: msg.image,
            image_data: msg.image_data,
            external_url: msg.external_url,
            description: msg.description,
            name: msg.name,
            attributes: msg.attributes,
            background_color: msg.background_color,
            animation_url: msg.animation_url,
            youtube_url: msg.youtube_url,
        }
    }
}

impl Contains for Empty {
    fn contains(&self, _other: &Empty) -> bool {
        true
    }
}

impl Contains for NftExtension {
    fn contains(&self, other: &NftExtension) -> bool {
        fn is_equal(a: &Option<String>, b: &Option<String>) -> bool {
            match (a, b) {
                (Some(a), Some(b)) => a.contains(b),
                (Some(_), None) => true,
                (None, None) => true,
                _ => false,
            }
        }
        if !is_equal(&self.image, &other.image) {
            return false;
        }
        if !is_equal(&self.image_data, &other.image_data) {
            return false;
        }
        if !is_equal(&self.external_url, &other.external_url) {
            return false;
        }
        if !is_equal(&self.description, &other.description) {
            return false;
        }
        if !is_equal(&self.name, &other.name) {
            return false;
        }
        if !is_equal(&self.background_color, &other.background_color) {
            return false;
        }
        if !is_equal(&self.animation_url, &other.animation_url) {
            return false;
        }
        if !is_equal(&self.youtube_url, &other.youtube_url) {
            return false;
        }
        if let (Some(a), Some(b)) = (&self.attributes, &other.attributes) {
            for (i, b) in b.iter().enumerate() {
                if !a[i].eq(b) {
                    return false;
                }
            }
        }
        true
    }
}

impl<T> Contains for Option<T>
where
    T: Contains,
{
    fn contains(&self, other: &Option<T>) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.contains(b),
            (None, None) => true,
            _ => false,
        }
    }
}

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

impl StateFactory<Trait> for Trait {
    fn create(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&Trait>,
    ) -> Result<Trait, Cw721ContractError> {
        self.validate(deps, env, info, current)?;
        Ok(self.clone())
    }

    fn validate(
        &self,
        _deps: Deps,
        _env: &Env,
        _info: Option<&MessageInfo>,
        _current: Option<&Trait>,
    ) -> Result<(), Cw721ContractError> {
        if self.trait_type.is_empty() {
            return Err(Cw721ContractError::TraitTypeEmpty {});
        }
        if self.value.is_empty() {
            return Err(Cw721ContractError::TraitValueEmpty {});
        }
        if let Some(display_type) = &self.display_type {
            if display_type.is_empty() {
                return Err(Cw721ContractError::TraitDisplayTypeEmpty {});
            }
        }
        Ok(())
    }
}

impl StateFactory<Vec<Trait>> for Vec<Trait> {
    fn create(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&Vec<Trait>>,
    ) -> Result<Vec<Trait>, Cw721ContractError> {
        self.validate(deps, env, info, current)?;
        Ok(self.clone())
    }

    fn validate(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        _current: Option<&Vec<Trait>>,
    ) -> Result<(), Cw721ContractError> {
        for attribute in self {
            attribute.validate(deps, env, info, None)?;
        }
        Ok(())
    }
}
