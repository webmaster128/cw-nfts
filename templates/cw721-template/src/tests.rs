use crate::contract::{entry, CONTRACT_NAME};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cw721_base::InstantiateMsg;

/// Make sure cw2 version info is properly initialized during instantiation,
/// and NOT overwritten by the base contract.
#[test]
fn proper_cw2_initialization() {
    let mut deps = mock_dependencies();

    entry::instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("larry", &[]),
        InstantiateMsg {
            name: "".into(),
            symbol: "".into(),
            minter: "larry".into(),
        },
    )
    .unwrap();

    let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
    assert_eq!(version.contract, CONTRACT_NAME);
    assert_ne!(version.contract, cw721_base::CONTRACT_NAME);
}
