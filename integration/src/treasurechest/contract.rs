use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, Empty, StdResult, WasmMsg, WasmQuery};
use cw_multi_test::{App, Contract, ContractWrapper};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use treasurechest::chest::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse};

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TreasureChestContract(pub Addr);

impl TreasureChestContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        self.call_with_funds(msg, vec![])
    }

    pub fn call_with_funds<T: Into<ExecuteMsg>>(
        &self,
        msg: T,
        funds: Vec<Coin>,
    ) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds,
        }
        .into())
    }

    pub fn withdraw(&self, funds: Vec<Coin>) -> StdResult<CosmosMsg> {
        self.call_with_funds(ExecuteMsg::Withdraw {}, funds)
    }

    pub fn new_wallet(&self, token_factory_type: String) -> StdResult<CosmosMsg> {
        self.call(ExecuteMsg::ChangeTokenFactory {
            token_factory_type,
        })
    }

    pub fn return_dust(&self, limit:Option<u32>) -> StdResult<CosmosMsg> {
        self.call(ExecuteMsg::ReturnDust {limit})
    }

    pub fn config(&self, app: &App) -> StdResult<ConfigResponse> {
        let msg = QueryMsg::Config {};
        self.query(app, &msg)
    }

    pub fn query<T>(&self, app: &App, msg: &QueryMsg) -> StdResult<T>
    where
        T: DeserializeOwned,
    {
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_json_binary(msg)?,
        }
        .into();
        app.wrap().query::<T>(&query)
    }

    pub fn state(&self, app: &App) -> StdResult<StateResponse> {
        let msg = QueryMsg::State {};
        self.query(app, &msg)
    }

    pub fn simple_instantiate(owner: &str, ticket: &str, burn: bool) -> InstantiateMsg {
        InstantiateMsg {
            denom: ticket.to_string(),
            owner: owner.into(),
            notes: "just a test".to_string(),
            token_factory: "CosmWasm".to_string(),
            burn_it: Some(burn),
        }
    }

    pub fn template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            treasurechest_contract::contract::execute,
            treasurechest_contract::contract::instantiate,
            treasurechest_contract::contract::query,
        );

        Box::new(contract)
    }
}
