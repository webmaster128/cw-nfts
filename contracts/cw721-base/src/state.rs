use cosmwasm_std::CustomMsg;

// expose to all others using contract, so others dont need to import cw721
pub use cw721::state::*;

use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Cw721Contract<
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
    pub config: Cw721Config<'a, TMetadataExtension, TCustomResponseMessage, TMetadataExtensionMsg>,
}

impl<TMetadataExtension, TCustomResponseMessage, TMetadataExtensionMsg> Default
    for Cw721Contract<'static, TMetadataExtension, TCustomResponseMessage, TMetadataExtensionMsg>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
    E: CustomMsg,
    Q: CustomMsg,
{
}

impl<T, C, E, Q> Default for Cw721Contract<'static, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    E: CustomMsg,
    Q: CustomMsg,
{
    fn default() -> Self {
        Self {
            config: Cw721Config::default(),
        }
    }
}