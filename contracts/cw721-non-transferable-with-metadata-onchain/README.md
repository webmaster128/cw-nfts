# CosmWasm Non-Transferable NFT with Onchain Metadata
A Non-Transferable NFT implementation by extending [`cw721-base`](https://github.com/CosmWasm/cw-nfts/tree/main/contracts/cw721-base) and using [`cw_ownable`](https://github.com/larry0x/cw-plus-plus/tree/main/packages/ownable) functions:

```rust
initialize_owner(deps.storage, deps.api, msg.admin.as_deref())?;
```

and 

```rust
get_ownership(deps.storage)?.owner;
```
to check only contract owner can execute with the additional feature of onchain metadata:

```rust
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
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

pub type Extension = Option<Metadata>;
```

This simplifies the json schema as it can be written now:

```rust
use cosmwasm_schema::write_api;

use nft::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
```
which was more complex in the [`cw721-non-transferable`](https://github.com/CosmWasm/cw-nfts/tree/main/contracts/cw721-non-transferable).