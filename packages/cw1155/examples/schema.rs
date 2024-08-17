use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};
use cosmwasm_std::Empty;

use cw1155::{
    AllBalancesResponse, ApprovedForAllResponse, BalanceResponse, BatchBalanceResponse,
    Cw1155ExecuteMsg, Cw1155QueryMsg, Cw1155ReceiveMsg, IsApprovedForAllResponse, MinterResponse,
    NumTokensResponse, TokenInfoResponse, TokensResponse,
};

type Extension = Empty;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();
    export_schema_with_title(&schema_for!(Cw1155ExecuteMsg), &out_dir, "ExecuteMsg");
    export_schema(&schema_for!(Cw1155QueryMsg), &out_dir);
    export_schema(&schema_for!(Cw1155ReceiveMsg), &out_dir);
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
