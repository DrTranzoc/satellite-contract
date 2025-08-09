use cosmwasm_std::{to_json_binary, CosmosMsg, Response, StdError, WasmMsg};
use cw721::Cw721ExecuteMsg;
use crate::ContractError;

pub(crate) fn standard_error(message : String) -> Result<Response, ContractError> {
    return Err(ContractError::Std(StdError::generic_err(format!("error : ||{}||", message.clone()))));
}

pub(crate) fn ensure_error(message : String) -> StdError {
    return StdError::generic_err(format!("error : ||{}||", message.clone()));
}

pub(crate) fn send_nft(collection : String, token_id : String, recipient : String) -> CosmosMsg {

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: collection.clone(),
        msg: to_json_binary(
            &(Cw721ExecuteMsg::TransferNft {
                token_id,
                recipient,
            }),
        ).unwrap(),
        funds: vec![],
    });
    return msg;
}