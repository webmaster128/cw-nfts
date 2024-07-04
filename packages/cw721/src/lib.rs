pub mod error;
pub mod execute;
pub mod helpers;
pub mod msg;
pub mod query;
pub mod receiver;
pub mod state;
pub mod traits;

use cosmwasm_std::Empty;
pub use cw_utils::Expiration;
use msg::{
    CollectionExtensionMsg, CollectionInfoAndExtensionResponse, NftExtensionMsg,
    RoyaltyInfoResponse,
};
pub use state::{Approval, Attribute, CollectionExtension, NftExtension, RoyaltyInfo};

/// Type for `Option<CollectionExtension<RoyaltyInfo>>`
pub type DefaultOptionalCollectionExtension = Option<CollectionExtension<RoyaltyInfo>>;
/// Type for `Option<Empty>`
pub type EmptyOptionalCollectionExtension = Option<Empty>;

/// Type for `Option<CollectionExtensionMsg<RoyaltyInfoResponse>>`
pub type DefaultOptionalCollectionExtensionMsg =
    Option<CollectionExtensionMsg<RoyaltyInfoResponse>>;
/// Type for `Option<Empty>`
pub type EmptyOptionalCollectionExtensionMsg = Option<Empty>;

/// Type for `Option<NftExtension>`.
pub type DefaultOptionalNftExtension = Option<NftExtension>;
/// Type for `Option<Empty>`
pub type EmptyOptionalNftExtension = Option<Empty>;

/// Type for `Option<NftExtensionMsg>`.
pub type DefaultOptionalNftExtensionMsg = Option<NftExtensionMsg>;
/// Type for `Option<Empty>`
pub type EmptyOptionalNftExtensionMsg = Option<Empty>;

// explicit type for better distinction.
#[deprecated(since = "0.19.0", note = "Please use `NftExtension` instead")]
pub type MetaData = NftExtension;
#[deprecated(
    since = "0.19.0",
    note = "Please use `CollectionInfoAndExtensionResponse<DefaultOptionalCollectionExtension>` instead"
)]
pub type ContractInfoResponse =
    CollectionInfoAndExtensionResponse<DefaultOptionalCollectionExtension>;
#[cfg(test)]
pub mod testing;
