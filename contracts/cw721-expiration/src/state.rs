use cosmwasm_std::{CustomMsg, Timestamp};

// expose to all others using contract, so others dont need to import cw721
pub use cw721::state::*;

use cw721_base::Cw721Contract;
use cw_storage_plus::{Item, Map};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Cw721ExpirationContract<
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
    pub expiration_days: Item<'a, u16>, // max 65535 days
    pub mint_timestamps: Map<'a, &'a str, Timestamp>,
    pub base_contract:
        Cw721Contract<'a, TMetadataExtension,TCustomResponseMessage, TMetadataExtensionMsg>,
}

impl<TMetadataExtension, TCustomResponseMessage, TMetadataExtensionMsg> Default
    for Cw721ExpirationContract<
        'static,
        TMetadataExtension,
        TCustomResponseMessage,
        TMetadataExtensionMsg,
    >
where
    TMetadataExtension: Serialize + DeserializeOwned + Clone,
    TMetadataExtensionMsg: CustomMsg,
{
    fn default() -> Self {
        Self {
            expiration_days: Item::new("expiration_days"),
            mint_timestamps: Map::new("mint_timestamps"),
            base_contract: Cw721Contract::default(),
        }
    }
}
