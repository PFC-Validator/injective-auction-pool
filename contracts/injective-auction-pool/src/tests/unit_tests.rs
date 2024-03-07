use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Decimal,
};
use injective_auction::auction_pool::InstantiateMsg;
use treasurechest::tf::tokenfactory::TokenFactoryType;

use crate::contract::instantiate;

#[test]
fn instantiate_contract() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        native_denom: "inj".to_string(),
        token_factory_type: TokenFactoryType::Injective,
        rewards_fee: Decimal::percent(10),
        rewards_fee_addr: "nyjah".to_string(),
        whitelisted_addresses: vec!["robinho".to_string()],
        min_next_bid_increment_rate: Decimal::from_ratio(25u128, 10_000u128),
        treasury_chest_code_id: 1,
        min_return: Decimal::percent(5),
    };

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.messages, vec![]);
}
