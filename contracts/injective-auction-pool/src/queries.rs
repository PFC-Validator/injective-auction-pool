use cosmwasm_std::{to_json_binary, Binary, Deps, StdResult};
use injective_auction::auction_pool::ConfigResponse;

use crate::state::CONFIG;

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    to_json_binary(&ConfigResponse {
        config: CONFIG.load(deps.storage)?,
    })
}
