use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use cw1155::{
    AllBalancesResponse, ApprovedForAllResponse, BalanceResponse, BatchBalanceResponse,
    Cw1155QueryMsg, IsApprovedForAllResponse, MinterResponse, NumTokensResponse, TokenInfoResponse,
    TokensResponse,
};
use cw1155_base::{ExecuteMsg, Extension, InstantiateMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema_with_title(&schema_for!(ExecuteMsg<Extension>), &out_dir, "ExecuteMsg");
    export_schema(&schema_for!(Cw1155QueryMsg), &out_dir);
    export_schema(&schema_for!(BalanceResponse), &out_dir);
    export_schema(&schema_for!(AllBalancesResponse), &out_dir);
    export_schema(&schema_for!(BatchBalanceResponse), &out_dir);
    export_schema(&schema_for!(NumTokensResponse), &out_dir);
    export_schema(&schema_for!(ApprovedForAllResponse), &out_dir);
    export_schema(&schema_for!(IsApprovedForAllResponse), &out_dir);
    export_schema(&schema_for!(TokensResponse), &out_dir);
    export_schema(&schema_for!(MinterResponse), &out_dir);
    export_schema_with_title(
        &schema_for!(TokenInfoResponse<Extension>),
        &out_dir,
        "TokenInfoResponse",
    );
}
