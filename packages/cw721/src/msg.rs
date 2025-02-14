use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Coin, ContractInfoResponse, Decimal, Deps, Env, MessageInfo,
    Timestamp,
};
use cw_ownable::{is_owner, Action, Ownership};
use cw_utils::Expiration;
use serde::Serialize;
use url::Url;

use crate::error::Cw721ContractError;
use crate::execute::{assert_creator, assert_minter};
use crate::state::{
    Attribute, CollectionExtension, CollectionExtensionAttributes, CollectionInfo, NftInfo, Trait,
    ATTRIBUTE_DESCRIPTION, ATTRIBUTE_EXPLICIT_CONTENT, ATTRIBUTE_EXTERNAL_LINK, ATTRIBUTE_IMAGE,
    ATTRIBUTE_ROYALTY_INFO, ATTRIBUTE_START_TRADING_TIME, MAX_COLLECTION_DESCRIPTION_LENGTH,
    MAX_ROYALTY_SHARE_DELTA_PCT, MAX_ROYALTY_SHARE_PCT, MINTER,
};
use crate::traits::{Cw721CustomMsg, Cw721State, FromAttributesState, ToAttributesState};
use crate::NftExtension;
use crate::{traits::StateFactory, Approval, RoyaltyInfo};

#[cw_serde]
pub enum Cw721ExecuteMsg<
    // NftInfo extension msg for onchain metadata.
    TNftExtensionMsg,
    // CollectionInfo extension msg for onchain collection attributes.
    TCollectionExtensionMsg,
    // Custom extension msg for custom contract logic. Default implementation is a no-op.
    TExtensionMsg,
> {
    #[deprecated(since = "0.19.0", note = "Please use UpdateMinterOwnership instead")]
    /// Deprecated: use UpdateMinterOwnership instead! Will be removed in next release!
    UpdateOwnership(Action),
    UpdateMinterOwnership(Action),
    UpdateCreatorOwnership(Action),

    /// The creator is the only one eligible to update `CollectionInfo`.
    UpdateCollectionInfo {
        collection_info: CollectionInfoMsg<TCollectionExtensionMsg>,
    },
    /// Transfer is a base message to move a token to another account without triggering actions
    TransferNft {
        recipient: String,
        token_id: String,
    },
    /// Send is a base message to transfer a token to a contract and trigger an action
    /// on the receiving contract.
    SendNft {
        contract: String,
        token_id: String,
        msg: Binary,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke {
        spender: String,
        token_id: String,
    },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll {
        operator: String,
    },

    /// Mint a new NFT, can only be called by the contract minter
    Mint {
        /// Unique ID of the NFT
        token_id: String,
        /// The owner of the newly minter NFT
        owner: String,
        /// Universal resource identifier for this NFT
        /// Should point to a JSON file that conforms to the ERC721
        /// Metadata JSON Schema
        token_uri: Option<String>,
        /// Any custom extension used by this contract
        extension: TNftExtensionMsg,
    },

    /// Burn an NFT the sender has access to
    Burn {
        token_id: String,
    },

    /// Custom msg execution. This is a no-op in default implementation.
    UpdateExtension {
        msg: TExtensionMsg,
    },

    /// The creator is the only one eligible to update NFT's token uri and onchain metadata (`NftInfo.extension`).
    /// NOTE: approvals and owner are not affected by this call, since they belong to the NFT owner.
    UpdateNftInfo {
        token_id: String,
        /// NOTE: Empty string is handled as None
        token_uri: Option<String>,
        extension: TNftExtensionMsg,
    },

    /// Sets address to send withdrawn fees to. Only owner can call this.
    SetWithdrawAddress {
        address: String,
    },
    /// Removes the withdraw address, so fees are sent to the contract. Only owner can call this.
    RemoveWithdrawAddress {},
    /// Withdraw from the contract to the given address. Anyone can call this,
    /// which is okay since withdraw address has been set by owner.
    WithdrawFunds {
        amount: Coin,
    },
}

#[cw_serde]
pub struct Cw721InstantiateMsg<TCollectionExtensionMsg> {
    /// Name of the NFT contract
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,
    /// Optional extension of the collection metadata
    pub collection_info_extension: TCollectionExtensionMsg,

    /// The minter is the only one who can create new NFTs.
    /// This is designed for a base NFT that is controlled by an external program
    /// or contract. You will likely replace this with custom logic in custom NFTs
    pub minter: Option<String>,

    /// Sets the creator of collection. The creator is the only one eligible to update `CollectionInfo`.
    pub creator: Option<String>,

    pub withdraw_address: Option<String>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum Cw721QueryMsg<
    // Return type of NFT metadata defined in `NftInfo` and `AllNftInfo`.
    TNftExtension,
    // Return type of collection extension defined in `GetCollectionInfo`.
    TCollectionExtension,
    // Custom query msg for custom contract logic. Default implementation returns an empty binary.
    TExtensionQueryMsg,
> {
    /// Return the owner of the given token, error if token does not exist
    #[returns(OwnerOfResponse)]
    OwnerOf {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },
    /// Return operator that can access all of the owner's tokens.
    #[returns(ApprovalResponse)]
    Approval {
        token_id: String,
        spender: String,
        include_expired: Option<bool>,
    },
    /// Return approvals that a token has
    #[returns(ApprovalsResponse)]
    Approvals {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// Return approval of a given operator for all tokens of an owner, error if not set
    #[returns(OperatorResponse)]
    Operator {
        owner: String,
        operator: String,
        include_expired: Option<bool>,
    },
    /// List all operators that can access all of the owner's tokens
    #[returns(OperatorsResponse)]
    AllOperators {
        owner: String,
        /// unset or false will filter out expired items, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Total number of tokens issued
    #[returns(NumTokensResponse)]
    NumTokens {},

    #[deprecated(
        since = "0.19.0",
        note = "Please use GetCollectionInfoAndExtension instead"
    )]
    #[returns(CollectionInfoAndExtensionResponse<TCollectionExtension>)]
    /// Deprecated: use GetCollectionInfoAndExtension instead! Will be removed in next release!
    ContractInfo {},

    /// Returns `AllCollectionInfoResponse`
    #[returns(ConfigResponse<TCollectionExtension>)]
    GetConfig {},

    /// Returns `CollectionInfoAndExtensionResponse`
    #[returns(CollectionInfoAndExtensionResponse<TCollectionExtension>)]
    GetCollectionInfoAndExtension {},

    /// returns `AllInfoResponse` which contains contract, collection and nft details
    #[returns(AllInfoResponse)]
    GetAllInfo {},

    /// Returns `CollectionExtensionAttributes`
    #[returns(CollectionExtensionAttributes)]
    GetCollectionExtensionAttributes {},

    #[deprecated(since = "0.19.0", note = "Please use GetMinterOwnership instead")]
    #[returns(Ownership<Addr>)]
    /// Deprecated: use GetMinterOwnership instead! Will be removed in next release!
    Ownership {},

    /// Return the minter
    #[deprecated(since = "0.19.0", note = "Please use GetMinterOwnership instead")]
    #[returns(MinterResponse)]
    /// Deprecated: use GetMinterOwnership instead! Will be removed in next release!
    Minter {},

    #[returns(Ownership<Addr>)]
    GetMinterOwnership {},

    #[returns(Ownership<Addr>)]
    GetCreatorOwnership {},

    /// With MetaData Extension.
    /// Returns metadata about one particular token, based on *ERC721 Metadata JSON Schema*
    /// but directly from the contract
    #[returns(NftInfoResponse<TNftExtension>)]
    NftInfo { token_id: String },

    #[returns(Option<NftInfoResponse<TNftExtension>>)]
    GetNftByExtension {
        extension: TNftExtension,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// With MetaData Extension.
    /// Returns the result of both `NftInfo` and `OwnerOf` as one query as an optimization
    /// for clients
    #[returns(AllNftInfoResponse<TNftExtension>)]
    AllNftInfo {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },

    /// With Enumerable extension.
    /// Returns all tokens owned by the given address, [] if unset.
    #[returns(TokensResponse)]
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// With Enumerable extension.
    /// Requires pagination. Lists all token_ids controlled by the contract.
    #[returns(TokensResponse)]
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Custom msg query. Default implementation returns an empty binary.
    #[returns(())]
    Extension { msg: TExtensionQueryMsg },

    #[returns(())]
    GetCollectionExtension { msg: TCollectionExtension },

    #[returns(Option<String>)]
    GetWithdrawAddress {},
}

#[cw_serde]
pub enum Cw721MigrateMsg {
    WithUpdate {
        minter: Option<String>,
        creator: Option<String>,
    },
}

#[cw_serde]
pub struct CollectionInfoMsg<TCollectionExtensionMsg> {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub extension: TCollectionExtensionMsg,
}

#[cw_serde]
pub struct AttributeMsg {
    pub attr_type: AttributeType,
    pub key: String,
    pub value: String,
    pub data: Option<HashMap<String, String>>,
}

impl AttributeMsg {
    pub fn string_value(&self) -> Result<String, Cw721ContractError> {
        Ok(self.value.clone())
    }

    pub fn u64_value(&self) -> Result<u64, Cw721ContractError> {
        Ok(self.value.parse::<u64>()?)
    }

    pub fn bool_value(&self) -> Result<bool, Cw721ContractError> {
        Ok(self.value.parse::<bool>()?)
    }

    pub fn decimal_value(&self) -> Result<Decimal, Cw721ContractError> {
        Ok(self.value.parse::<Decimal>()?)
    }

    pub fn timestamp_value(&self) -> Result<Timestamp, Cw721ContractError> {
        let nanos = self.u64_value()?;
        Ok(Timestamp::from_nanos(nanos))
    }

    pub fn addr_value(&self) -> Result<Addr, Cw721ContractError> {
        Ok(Addr::unchecked(self.string_value()?))
    }
}

impl AttributeMsg {
    pub fn from(&self) -> Result<Attribute, Cw721ContractError> {
        let value = match self.attr_type {
            AttributeType::String => to_json_binary(&self.string_value()?)?,
            AttributeType::U64 => to_json_binary(&self.u64_value()?)?,
            AttributeType::Boolean => to_json_binary(&self.bool_value()?)?,
            AttributeType::Decimal => to_json_binary(&self.decimal_value()?)?,
            AttributeType::Timestamp => to_json_binary(&self.timestamp_value()?)?,
            AttributeType::Addr => to_json_binary(&self.addr_value()?)?,
            AttributeType::Custom => {
                return Err(Cw721ContractError::UnsupportedCustomAttributeType {
                    key: self.key.clone(),
                    value: self.value.clone(),
                });
            }
        };
        let attribute = Attribute {
            key: self.key.clone(),
            value,
        };
        Ok(attribute)
    }
}

#[cw_serde]
pub enum AttributeType {
    String,
    U64,
    Boolean,
    Timestamp,
    Addr,
    Decimal,
    Custom,
}

#[cw_serde]
/// NOTE: In case `info` is not provided in `create()` or `validate()` (like for migration), creator/minter assertion is skipped.
pub struct CollectionExtensionMsg<TRoyaltyInfoResponse> {
    pub description: Option<String>,
    pub image: Option<String>,
    pub external_link: Option<String>,
    pub explicit_content: Option<bool>,
    pub start_trading_time: Option<Timestamp>,
    pub royalty_info: Option<TRoyaltyInfoResponse>,
}

impl<TRoyaltyInfoResponse> Cw721CustomMsg for CollectionExtensionMsg<TRoyaltyInfoResponse> where
    TRoyaltyInfoResponse: Cw721CustomMsg
{
}

impl StateFactory<CollectionExtension<RoyaltyInfo>>
    for CollectionExtensionMsg<RoyaltyInfoResponse>
{
    /// NOTE: In case `info` is not provided (like for migration), creator/minter assertion is skipped.
    fn create(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&CollectionExtension<RoyaltyInfo>>,
    ) -> Result<CollectionExtension<RoyaltyInfo>, Cw721ContractError> {
        self.validate(deps, env, info, current)?;
        match current {
            // Some: update existing metadata
            Some(current) => {
                let mut updated = current.clone();
                if let Some(description) = &self.description {
                    updated.description.clone_from(description);
                }
                if let Some(image) = &self.image {
                    updated.image.clone_from(image)
                }
                if let Some(external_link) = &self.external_link {
                    updated.external_link = Some(external_link.clone());
                }
                if let Some(explicit_content) = self.explicit_content {
                    updated.explicit_content = Some(explicit_content);
                }
                if let Some(start_trading_time) = self.start_trading_time {
                    updated.start_trading_time = Some(start_trading_time);
                }
                if let Some(royalty_info_response) = &self.royalty_info {
                    match current.royalty_info.clone() {
                        // Some: existing royalty info for update
                        Some(current_royalty_info) => {
                            updated.royalty_info = Some(royalty_info_response.create(
                                deps,
                                env,
                                info,
                                Some(&current_royalty_info),
                            )?);
                        }
                        // None: no royalty info, so create new
                        None => {
                            updated.royalty_info =
                                Some(royalty_info_response.create(deps, env, info, None)?);
                        }
                    }
                }
                Ok(updated)
            }
            // None: create new metadata
            None => {
                let royalty_info = match &self.royalty_info {
                    // new royalty info
                    Some(royalty_info) => Some(royalty_info.create(deps, env, info, None)?),
                    // current royalty is none and new royalty is none
                    None => None,
                };
                let new = CollectionExtension {
                    description: self.description.clone().unwrap_or_default(),
                    image: self.image.clone().unwrap_or_default(),
                    external_link: self.external_link.clone(),
                    explicit_content: self.explicit_content,
                    start_trading_time: self.start_trading_time,
                    royalty_info,
                };
                Ok(new)
            }
        }
    }

    /// NOTE: In case `info` is not provided (like for migration), creator/minter assertion is skipped.
    fn validate(
        &self,
        deps: Deps,
        _env: &Env,
        info: Option<&MessageInfo>,
        _current: Option<&CollectionExtension<RoyaltyInfo>>,
    ) -> Result<(), Cw721ContractError> {
        let sender = info.map(|i| &i.sender);
        // start trading time can only be updated by minter
        let is_minter_or_uninitialized = match sender
            .map(|addr| MINTER.is_owner(deps.storage, addr))
            .transpose()
        {
            // if Some(bool), bool represents the `is_owner` check result.
            // if None, `info` was none and the check is skipped.
            Ok(is_minter) => is_minter.unwrap_or(true),
            // `is_owner` returns an error if the store is not initialized.
            // the check is skipped in this case as well.
            Err(_) => true,
        };
        if self.start_trading_time.is_some()
            && MINTER.item.exists(deps.storage)
            && !is_minter_or_uninitialized
        {
            return Err(Cw721ContractError::NotMinter {});
        }
        // all other props collection extension can only be updated by the creator
        let is_creator_or_uninitialized =
            match sender.map(|addr| is_owner(deps.storage, addr)).transpose() {
                // if Some(bool), bool represents the `is_owner` check result.
                // if None, `info` was none and the check is skipped.
                Ok(is_minter) => is_minter.unwrap_or(true),
                // `is_owner` returns an error if the store is not initialized.
                // the check is skipped in this case as well.
                Err(_) => true,
            };
        if (self.description.is_some()
            || self.image.is_some()
            || self.external_link.is_some()
            || self.explicit_content.is_some())
            && sender.is_some()
            && !is_creator_or_uninitialized
        {
            return Err(Cw721ContractError::NotCreator {});
        }
        // check description length, must not be empty and max 512 chars
        if let Some(description) = &self.description {
            if description.is_empty() {
                return Err(Cw721ContractError::CollectionDescriptionEmpty {});
            }
            if description.len() > MAX_COLLECTION_DESCRIPTION_LENGTH as usize {
                return Err(Cw721ContractError::CollectionDescriptionTooLong {
                    max_length: MAX_COLLECTION_DESCRIPTION_LENGTH,
                });
            }
        }

        // check images are URLs
        if let Some(image) = &self.image {
            Url::parse(image)?;
        }
        if let Some(external_link) = &self.external_link {
            Url::parse(external_link)?;
        }
        // no need to check royalty info, as it is checked during creation of RoyaltyInfo
        Ok(())
    }
}

#[cw_serde]
// This is both: a query response, and incoming message during instantiation and execution.
pub struct RoyaltyInfoResponse {
    pub payment_address: String,
    pub share: Decimal,
}

impl Cw721CustomMsg for RoyaltyInfoResponse {}

impl StateFactory<RoyaltyInfo> for RoyaltyInfoResponse {
    fn create(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&RoyaltyInfo>,
    ) -> Result<RoyaltyInfo, Cw721ContractError> {
        self.validate(deps, env, info, current)?;
        match current {
            // Some: update existing royalty info
            Some(current) => {
                let mut updated = current.clone();
                updated.payment_address = Addr::unchecked(self.payment_address.as_str()); // no check needed, since it is already done in validate
                updated.share = self.share;
                Ok(updated)
            }
            // None: create new royalty info
            None => {
                let new = RoyaltyInfo {
                    payment_address: Addr::unchecked(self.payment_address.as_str()), // no check needed, since it is already done in validate
                    share: self.share,
                };
                Ok(new)
            }
        }
    }

    fn validate(
        &self,
        deps: Deps,
        _env: &Env,
        _info: Option<&MessageInfo>,
        current: Option<&RoyaltyInfo>,
    ) -> Result<(), Cw721ContractError> {
        if let Some(current_royalty_info) = current {
            // check max share delta
            if current_royalty_info.share < self.share {
                let share_delta = self.share.abs_diff(current_royalty_info.share);

                if share_delta > Decimal::percent(MAX_ROYALTY_SHARE_DELTA_PCT) {
                    return Err(Cw721ContractError::InvalidRoyalties(format!(
                        "Share increase cannot be greater than {MAX_ROYALTY_SHARE_DELTA_PCT}%"
                    )));
                }
            }
        }
        // check max share
        if self.share > Decimal::percent(MAX_ROYALTY_SHARE_PCT) {
            return Err(Cw721ContractError::InvalidRoyalties(format!(
                "Share cannot be greater than {MAX_ROYALTY_SHARE_PCT}%"
            )));
        }
        // validate payment address
        deps.api.addr_validate(self.payment_address.as_str())?;
        Ok(())
    }
}

impl From<RoyaltyInfo> for RoyaltyInfoResponse {
    fn from(royalty_info: RoyaltyInfo) -> Self {
        Self {
            payment_address: royalty_info.payment_address.to_string(),
            share: royalty_info.share,
        }
    }
}

/// This is a wrapper around CollectionInfo that includes the extension.
#[cw_serde]
pub struct ConfigResponse<TCollectionExtension> {
    pub num_tokens: u64,
    pub minter_ownership: Ownership<Addr>,
    pub creator_ownership: Ownership<Addr>,
    pub withdraw_address: Option<String>,
    pub collection_info: CollectionInfo,
    pub collection_extension: TCollectionExtension,
    pub contract_info: ContractInfoResponse,
}

/// This is a wrapper around CollectionInfo that includes the extension.
#[cw_serde]
pub struct CollectionInfoAndExtensionResponse<TCollectionExtension> {
    pub name: String,
    pub symbol: String,
    pub extension: TCollectionExtension,
    pub updated_at: Timestamp,
}

/// This is a wrapper around CollectionInfo that includes the extension, contract info, and number of tokens (supply).
#[cw_serde]
pub struct AllInfoResponse {
    // contract details
    pub contract_info: ContractInfoResponse,
    // collection details
    pub collection_info: CollectionInfo,
    pub collection_extension: CollectionExtensionAttributes,
    // NFT details
    pub num_tokens: u64,
}

impl<T> From<CollectionInfoAndExtensionResponse<T>> for CollectionInfo {
    fn from(response: CollectionInfoAndExtensionResponse<T>) -> Self {
        CollectionInfo {
            name: response.name,
            symbol: response.symbol,
            updated_at: response.updated_at,
        }
    }
}

impl<TCollectionExtension, TCollectionExtensionMsg>
    StateFactory<CollectionInfoAndExtensionResponse<TCollectionExtension>>
    for CollectionInfoMsg<TCollectionExtensionMsg>
where
    TCollectionExtension: Cw721State,
    TCollectionExtensionMsg: Cw721CustomMsg + StateFactory<TCollectionExtension>,
{
    fn create(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&CollectionInfoAndExtensionResponse<TCollectionExtension>>,
    ) -> Result<CollectionInfoAndExtensionResponse<TCollectionExtension>, Cw721ContractError> {
        self.validate(deps, env, info, current)?;
        match current {
            // Some: update existing metadata
            Some(current) => {
                let mut updated = current.clone();
                if let Some(name) = &self.name {
                    updated.name.clone_from(name);
                }
                if let Some(symbol) = &self.symbol {
                    updated.symbol.clone_from(symbol);
                }
                let current_extension = current.extension.clone();
                let updated_extension =
                    self.extension
                        .create(deps, env, info, Some(&current_extension))?;
                updated.extension = updated_extension;
                Ok(updated)
            }
            // None: create new metadata
            None => {
                let extension = self.extension.create(deps, env, info, None)?;
                let new = CollectionInfoAndExtensionResponse {
                    name: self.name.clone().unwrap(),
                    symbol: self.symbol.clone().unwrap(),
                    extension,
                    updated_at: env.block.time,
                };
                Ok(new)
            }
        }
    }

    fn validate(
        &self,
        deps: Deps,
        _env: &Env,
        info: Option<&MessageInfo>,
        _current: Option<&CollectionInfoAndExtensionResponse<TCollectionExtension>>,
    ) -> Result<(), Cw721ContractError> {
        // make sure the name and symbol are not empty
        if self.name.is_some() && self.name.clone().unwrap().is_empty() {
            return Err(Cw721ContractError::CollectionNameEmpty {});
        }
        if self.symbol.is_some() && self.symbol.clone().unwrap().is_empty() {
            return Err(Cw721ContractError::CollectionSymbolEmpty {});
        }
        // collection metadata can only be updated by the creator. creator assertion is skipped for these cases:
        // - CREATOR store is empty/not initioized (like in instantiation)
        // - info is none (like in migration)
        let is_creator_or_uninitialized = match info
            .map(|inf| is_owner(deps.storage, &inf.sender))
            .transpose()
        {
            // if Some(bool), bool represents the `is_owner` check result.
            // if None, `info` was none and the check is skipped.
            Ok(is_minter) => is_minter.unwrap_or(true),
            // `is_owner` returns an error if the store is not initialized.
            // the check is skipped in this case as well.
            Err(_) => true,
        };
        if (self.name.is_some() || self.symbol.is_some()) && !is_creator_or_uninitialized {
            return Err(Cw721ContractError::NotCreator {});
        }
        Ok(())
    }
}

impl<TRoyaltyInfo> ToAttributesState for CollectionExtension<TRoyaltyInfo>
where
    TRoyaltyInfo: Serialize,
{
    fn to_attributes_state(&self) -> Result<Vec<Attribute>, Cw721ContractError> {
        let attributes = vec![
            Attribute {
                key: ATTRIBUTE_DESCRIPTION.to_string(),
                value: to_json_binary(&self.description)?,
            },
            Attribute {
                key: ATTRIBUTE_IMAGE.to_string(),
                value: to_json_binary(&self.image)?,
            },
            Attribute {
                key: ATTRIBUTE_EXTERNAL_LINK.to_string(),
                value: to_json_binary(&self.external_link.clone())?,
            },
            Attribute {
                key: ATTRIBUTE_EXPLICIT_CONTENT.to_string(),
                value: to_json_binary(&self.explicit_content)?,
            },
            Attribute {
                key: ATTRIBUTE_START_TRADING_TIME.to_string(),
                value: to_json_binary(&self.start_trading_time)?,
            },
            Attribute {
                key: ATTRIBUTE_ROYALTY_INFO.to_string(),
                value: to_json_binary(&self.royalty_info)?,
            },
        ];
        Ok(attributes)
    }
}

impl<TRoyaltyInfo> FromAttributesState for CollectionExtension<TRoyaltyInfo>
where
    TRoyaltyInfo: ToAttributesState + FromAttributesState,
{
    fn from_attributes_state(attributes: &[Attribute]) -> Result<Self, Cw721ContractError> {
        let description = attributes
            .iter()
            .find(|attr| attr.key == ATTRIBUTE_DESCRIPTION)
            .ok_or_else(|| Cw721ContractError::AttributeMissing("description".to_string()))?
            .value::<String>()?;
        let image = attributes
            .iter()
            .find(|attr| attr.key == ATTRIBUTE_IMAGE)
            .ok_or_else(|| Cw721ContractError::AttributeMissing("image".to_string()))?
            .value::<String>()?;
        let external_link = attributes
            .iter()
            .find(|attr| attr.key == ATTRIBUTE_EXTERNAL_LINK)
            .ok_or_else(|| Cw721ContractError::AttributeMissing("external link".to_string()))?
            .value::<Option<String>>()?;
        let explicit_content = attributes
            .iter()
            .find(|attr| attr.key == ATTRIBUTE_EXPLICIT_CONTENT)
            .ok_or_else(|| Cw721ContractError::AttributeMissing("explicit content".to_string()))?
            .value::<Option<bool>>()?;
        let start_trading_time = attributes
            .iter()
            .find(|attr| attr.key == ATTRIBUTE_START_TRADING_TIME)
            .ok_or_else(|| Cw721ContractError::AttributeMissing("start trading time".to_string()))?
            .value::<Option<Timestamp>>()?;

        let royalty_info = attributes
            .iter()
            .find(|attr| attr.key == ATTRIBUTE_ROYALTY_INFO)
            .ok_or_else(|| Cw721ContractError::AttributeMissing("royalty info".to_string()))?
            .value::<Option<RoyaltyInfo>>()?;

        let royalty_info = if royalty_info.is_some() {
            Some(FromAttributesState::from_attributes_state(attributes)?)
        } else {
            None
        };
        Ok(CollectionExtension {
            description,
            image,
            external_link,
            explicit_content,
            start_trading_time,
            royalty_info,
        })
    }
}

#[cw_serde]
pub struct OwnerOfResponse {
    /// Owner of the token
    pub owner: String,
    /// If set this address is approved to transfer/send the token as well
    pub approvals: Vec<Approval>,
}

#[cw_serde]
pub struct ApprovalResponse {
    pub approval: Approval,
}

#[cw_serde]
pub struct ApprovalsResponse {
    pub approvals: Vec<Approval>,
}

#[cw_serde]
pub struct OperatorResponse {
    pub approval: Approval,
}

#[cw_serde]
pub struct OperatorsResponse {
    pub operators: Vec<Approval>,
}

#[cw_serde]
pub struct NumTokensResponse {
    pub count: u64,
}

#[cw_serde]
pub struct NftInfoResponse<TNftExtension> {
    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
    /// You can add any custom metadata here when you extend cw721-base
    pub extension: TNftExtension,
}

#[cw_serde]
pub struct AllNftInfoResponse<TNftExtension> {
    /// Who can transfer the token
    pub access: OwnerOfResponse,
    /// Data on the token itself,
    pub info: NftInfoResponse<TNftExtension>,
}

#[cw_serde]
pub struct TokensResponse {
    /// Contains all token_ids in lexicographical ordering
    /// If there are more than `limit`, use `start_after` in future queries
    /// to achieve pagination.
    pub tokens: Vec<String>,
}

/// Deprecated: use Cw721QueryMsg::GetMinterOwnership instead!
/// Shows who can mint these tokens.
#[cw_serde]
pub struct MinterResponse {
    pub minter: Option<String>,
}

#[cw_serde]
pub struct NftInfoMsg<TNftExtensionMsg> {
    /// The owner of the newly minted NFT
    pub owner: String,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    /// NOTE: Empty string is handled as None
    pub token_uri: Option<String>,

    /// You can add any custom metadata here when you extend cw721-base
    pub extension: TNftExtensionMsg,
}

impl<TNftExtension, TNftExtensionMsg> StateFactory<NftInfo<TNftExtension>>
    for NftInfoMsg<TNftExtensionMsg>
where
    TNftExtension: Cw721State,
    TNftExtensionMsg: Cw721CustomMsg + StateFactory<TNftExtension>,
{
    fn create(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        optional_current: Option<&NftInfo<TNftExtension>>,
    ) -> Result<NftInfo<TNftExtension>, Cw721ContractError> {
        self.validate(deps, env, info, optional_current)?;
        match optional_current {
            // Some: update only token uri and extension in existing NFT (but not owner and approvals)
            Some(current) => {
                let mut updated = current.clone();
                if self.token_uri.is_some() {
                    updated.token_uri = empty_as_none(self.token_uri.clone());
                }
                // update extension
                // current extension is a nested option in option, so we need to flatten it
                let current_extension = optional_current.map(|c| &c.extension);
                updated.extension = self.extension.create(deps, env, info, current_extension)?;
                Ok(updated)
            }
            // None: create new NFT, note: msg is of same type, so we can clone it
            None => {
                let extension = self.extension.create(deps, env, info, None)?;
                let token_uri = empty_as_none(self.token_uri.clone());
                Ok(NftInfo {
                    owner: Addr::unchecked(&self.owner), // only for creation we use owner, but not for update!
                    approvals: vec![],
                    token_uri,
                    extension,
                })
            }
        }
    }

    fn validate(
        &self,
        deps: Deps,
        _env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&NftInfo<TNftExtension>>,
    ) -> Result<(), Cw721ContractError> {
        let info = info.ok_or(Cw721ContractError::NoInfo)?;
        if current.is_none() {
            // current is none: only minter can create new NFT
            assert_minter(deps.storage, &info.sender)?;
        } else {
            // current is some: only creator can update NFT
            assert_creator(deps.storage, &info.sender)?;
        }
        // validate token_uri is a URL
        let token_uri = empty_as_none(self.token_uri.clone());
        if let Some(token_uri) = token_uri {
            Url::parse(token_uri.as_str())?;
        }
        // validate owner
        deps.api.addr_validate(&self.owner)?;
        Ok(())
    }
}

#[cw_serde]
#[derive(Default)]
pub struct NftExtensionMsg {
    /// NOTE: Empty string is handled as None
    pub image: Option<String>,
    pub image_data: Option<String>,
    /// NOTE: Empty string is handled as None
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    /// NOTE: Empty string is handled as None
    pub animation_url: Option<String>,
    /// NOTE: Empty string is handled as None
    pub youtube_url: Option<String>,
}

impl Cw721CustomMsg for NftExtensionMsg {}

impl From<NftExtension> for NftExtensionMsg {
    fn from(extension: NftExtension) -> Self {
        NftExtensionMsg {
            image: extension.image,
            image_data: extension.image_data,
            external_url: extension.external_url,
            description: extension.description,
            name: extension.name,
            attributes: extension.attributes,
            background_color: extension.background_color,
            animation_url: extension.animation_url,
            youtube_url: extension.youtube_url,
        }
    }
}

impl StateFactory<NftExtension> for NftExtensionMsg {
    fn create(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&NftExtension>,
    ) -> Result<NftExtension, Cw721ContractError> {
        self.validate(deps, env, info, current)?;
        match current {
            // Some: update existing metadata
            Some(current) => {
                let mut updated = current.clone();
                if self.image.is_some() {
                    updated.image = empty_as_none(self.image.clone());
                }
                if self.image_data.is_some() {
                    updated.image_data = empty_as_none(self.image_data.clone());
                }
                if self.external_url.is_some() {
                    updated.external_url = empty_as_none(self.external_url.clone());
                }
                if self.description.is_some() {
                    updated.description = empty_as_none(self.description.clone());
                }
                if self.name.is_some() {
                    updated.name = empty_as_none(self.name.clone());
                }
                if self.attributes.is_some() {
                    updated.attributes = match self.attributes.clone() {
                        Some(attributes) => Some(attributes.create(deps, env, info, None)?),
                        None => None,
                    };
                }
                if self.background_color.is_some() {
                    updated.background_color = empty_as_none(self.background_color.clone())
                }
                if self.animation_url.is_some() {
                    updated.animation_url = empty_as_none(self.animation_url.clone());
                }
                if self.youtube_url.is_some() {
                    updated.youtube_url = empty_as_none(self.youtube_url.clone());
                }
                Ok(updated)
            }
            // None: create new metadata, note: msg is of same type as metadata, so we can clone it
            None => {
                let mut new_metadata: NftExtension = self.clone().into();
                if self.attributes.is_some() {
                    new_metadata.attributes = match self.attributes.clone() {
                        Some(attributes) => Some(attributes.create(deps, env, info, None)?),
                        None => None,
                    };
                }
                Ok(new_metadata)
            }
        }
    }

    fn validate(
        &self,
        deps: Deps,
        _env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&NftExtension>,
    ) -> Result<(), Cw721ContractError> {
        // assert here is different to NFT Info:
        // - creator and minter can create NFT metadata
        // - only creator can update NFT metadata
        if current.is_none() {
            let info = info.ok_or(Cw721ContractError::NoInfo)?;
            // current is none: minter and creator can create new NFT metadata
            let minter_check = assert_minter(deps.storage, &info.sender);
            let creator_check = assert_creator(deps.storage, &info.sender);
            if minter_check.is_err() && creator_check.is_err() {
                return Err(Cw721ContractError::NotMinterOrCreator {});
            }
        } else {
            let info = info.ok_or(Cw721ContractError::NoInfo)?;
            // current is some: only creator can update NFT metadata
            assert_creator(deps.storage, &info.sender)?;
        }
        // check URLs
        let image = empty_as_none(self.image.clone());
        if let Some(image) = &image {
            Url::parse(image)?;
        }
        let external_url = empty_as_none(self.external_url.clone());
        if let Some(url) = &external_url {
            Url::parse(url)?;
        }
        let animation_url = empty_as_none(self.animation_url.clone());
        if let Some(animation_url) = &animation_url {
            Url::parse(animation_url)?;
        }
        let youtube_url = empty_as_none(self.youtube_url.clone());
        if let Some(youtube_url) = &youtube_url {
            Url::parse(youtube_url)?;
        }
        // no need to validate simple strings: image_data, description, name, and background_color
        Ok(())
    }
}

pub fn empty_as_none(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.is_empty())
}

impl<TMsg, TState> StateFactory<Option<TState>> for Option<TMsg>
where
    TState: Cw721State,
    TMsg: Cw721CustomMsg + StateFactory<TState>,
{
    fn create(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&Option<TState>>,
    ) -> Result<Option<TState>, Cw721ContractError> {
        // no msg, so no validation needed
        if self.is_none() {
            return Ok(None);
        }
        let msg = self.clone().unwrap();
        // current is a nested option in option, so we need to flatten it
        let current = current.and_then(|c| c.as_ref());
        let created_or_updated = msg.create(deps, env, info, current)?;
        Ok(Some(created_or_updated))
    }

    fn validate(
        &self,
        deps: Deps,
        env: &Env,
        info: Option<&MessageInfo>,
        current: Option<&Option<TState>>,
    ) -> Result<(), Cw721ContractError> {
        // no msg, so no validation needed
        if self.is_none() {
            return Ok(());
        }
        let msg = self.clone().unwrap();
        // current is a nested option in option, so we need to flatten it
        let current = current.and_then(|c| c.as_ref());
        msg.validate(deps, env, info, current)
    }
}
