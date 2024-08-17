use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdResult, Storage, Uint128};

use cw1155::{Balance, Expiration};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

pub struct Cw1155Contract<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    pub minter: Item<'a, Addr>,
    pub token_count: Map<'a, &'a str, Uint128>,
    pub balances: IndexedMap<'a, (Addr, String), Balance, BalanceIndexes<'a>>,
    pub approves: Map<'a, (&'a Addr, &'a Addr), Expiration>,
    pub tokens: Map<'a, &'a str, TokenInfo<T>>,
}

impl<'a, T> Default for Cw1155Contract<'static, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn default() -> Self {
        Self::new(
            "minter",
            "token_count",
            "balances",
            "approves",
            "tokens",
            "balances__token_id",
        )
    }
}

impl<'a, T> Cw1155Contract<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn new(
        minter_key: &'a str,
        token_count_key: &'a str,
        balances_key: &'a str,
        approves_key: &'a str,
        tokens_key: &'a str,
        balances_token_id_key: &'a str,
    ) -> Self {
        let indexes = BalanceIndexes {
            token_id: MultiIndex::new(balances_token_id_idx, balances_key, balances_token_id_key),
        };
        Self {
            minter: Item::new(minter_key),
            token_count: Map::new(token_count_key),
            balances: IndexedMap::new(balances_key, indexes),
            approves: Map::new(approves_key),
            tokens: Map::new(tokens_key),
        }
    }

    pub fn token_count(&self, storage: &dyn Storage, token_id: &'a str) -> StdResult<Uint128> {
        Ok(self
            .token_count
            .may_load(storage, token_id)?
            .unwrap_or_default())
    }

    pub fn increment_tokens(
        &self,
        storage: &mut dyn Storage,
        token_id: &'a str,
        amount: &Uint128,
    ) -> StdResult<Uint128> {
        let val = self.token_count(storage, token_id)? + amount;
        self.token_count.save(storage, token_id, &val)?;
        Ok(val)
    }

    pub fn decrement_tokens(
        &self,
        storage: &mut dyn Storage,
        token_id: &'a str,
        amount: &Uint128,
    ) -> StdResult<Uint128> {
        let val = self.token_count(storage, token_id)?.checked_sub(*amount)?;
        self.token_count.save(storage, token_id, &val)?;
        Ok(val)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo<T> {
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
    /// You can add any custom metadata here when you extend cw1155-base
    pub extension: Option<T>,
}

pub struct BalanceIndexes<'a> {
    pub token_id: MultiIndex<'a, String, Balance, (Addr, String)>,
}

impl<'a> IndexList<Balance> for BalanceIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Balance>> + '_> {
        let v: Vec<&dyn Index<Balance>> = vec![&self.token_id];
        Box::new(v.into_iter())
    }
}

pub fn balances_token_id_idx(d: &Balance) -> String {
    d.token_id.clone()
}
