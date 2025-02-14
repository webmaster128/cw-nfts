use cosmwasm_std::Timestamp;

use cw721::extension::Cw721OnchainExtensions;
use cw_storage_plus::{Item, Map};

/// Opionated version of generic `Cw721ExpirationContract` with default onchain nft and collection extensions using:
/// - `DefaultOptionalNftExtension` for NftInfo extension (onchain metadata).
/// - `DefaultOptionalNftExtensionMsg` for NftInfo extension msg for onchain metadata.
/// - `DefaultOptionalCollectionExtension` for CollectionInfo extension (onchain attributes).
/// - `DefaultOptionalCollectionExtensionMsg` for CollectionInfo extension msg for onchain collection attributes.
/// - `Empty` for custom extension msg for custom contract logic.
/// - `Empty` for custom query msg for custom contract logic.
/// - `Empty` for custom response msg for custom contract logic.
pub struct DefaultCw721ExpirationContract<'a> {
    pub expiration_days: Item<u16>, // max 65535 days
    pub mint_timestamps: Map<&'a str, Timestamp>,
    pub base_contract: Cw721OnchainExtensions<'a>,
}

impl Default for DefaultCw721ExpirationContract<'static> {
    fn default() -> Self {
        Self {
            expiration_days: Item::new("expiration_days"),
            mint_timestamps: Map::new("mint_timestamps"),
            base_contract: Cw721OnchainExtensions::default(),
        }
    }
}
