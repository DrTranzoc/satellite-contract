use cosmwasm_std::{ensure, from_json, to_json_binary, Addr, IbcMsg, IbcTimeout, Timestamp};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::{datatypes::{Cw721ReceiveMsg, IbcPacketOutgoing, NftReceiveMsg, PacketType, State}, helpers::{ensure_error, standard_error}, msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UpdateStatePayload}, queries::{get_all_users_data, get_pending_packets, get_state, get_token_status, get_user_data}, state::{CHANNEL, PENDING_PACKETS_REQUESTS, STATE, UNIQUE_PACKETS_REQUEST_ID, USERS_DATA}, ContractError};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:gamefi_satellite";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MIGRATE_VERSION: &str = "0.1.0";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, MIGRATE_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(deps: DepsMut, _env: Env, info: MessageInfo, msg: InstantiateMsg) -> Result<Response, ContractError> {
    
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let state = State {
        collections_info : msg.collections_info,
        admin : info.sender.clone(),
        ibc_settings : msg.ibc_settings,
        host_chain_prefix : msg.host_chain_prefix
    };

    STATE.save(deps.storage, &state)?;
    UNIQUE_PACKETS_REQUEST_ID.save(deps.storage, &0u128)?;
    
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, _env: Env, info: MessageInfo, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNft(message) => init_lock_procedure(deps, _env, info, message),
        ExecuteMsg::UnlockToken { collection, token_id } => init_unlock_procedure(deps, _env, info, collection, token_id),
        ExecuteMsg::UpdateStatePayload { state_changes } => update_state(deps, info, state_changes)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetUserData{address}=> to_json_binary(&get_user_data(deps,address)?),
        QueryMsg::GetAllUsersData{start_after,limit}=>to_json_binary(&get_all_users_data(deps,start_after,limit)?),
        QueryMsg::GetPendingPackets{start_after,limit}=>to_json_binary(&get_pending_packets(deps,start_after,limit)?),
        QueryMsg::GetState{}=>to_json_binary(&get_state(deps)?),
        QueryMsg::GetTokenStatus { user, collection, token_id } => to_json_binary(&get_token_status(deps, user, collection, token_id)?)
    }
}

pub fn update_state(
    deps: DepsMut,
    info: MessageInfo,
    state_changes: UpdateStatePayload,
) -> Result<Response, ContractError> {
    // Load the current state
    let mut state: State = STATE.load(deps.storage)?;

    //Check if admin
    if state.admin != info.sender.clone() {
        return standard_error("Only admin can modify the contract settings".to_string())
    }

    //Check each field, update the state with the new version
    if let Some(admin) = state_changes.admin {
        state.admin = admin;
    }
    if let Some(collections_info) = state_changes.collections_info {
        state.collections_info = collections_info;
    }
    if let Some(ibc_settings) = state_changes.ibc_settings {
        state.ibc_settings = ibc_settings;
    }
    if let Some(host_chain_prefix) = state_changes.host_chain_prefix {
        state.host_chain_prefix = host_chain_prefix;
    }

    Ok(
        Response::default()
        .add_attribute("action", "state changed")
    )
}

/**
 * Init the lock procedure, 
 * 0) Basic checks
 * 1) Create and save the pending request in the state
 * 2) Send the IBC lock request to the main contract
 */
fn init_lock_procedure(deps: DepsMut, env: Env, info: MessageInfo, message: Cw721ReceiveMsg) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let channel_info = CHANNEL.load(deps.storage)?;

    let is_collection_supported = state.collections_info
            .iter()
            .find(|collection| collection.address == info.sender.to_string())
            .is_some();

    ensure!(is_collection_supported , ensure_error("The collection is not supported.".into()));
    ensure!(channel_info.finalized, ensure_error("can't lock, IBC channel is closed.".into()));

    let msg: NftReceiveMsg = from_json(&message.msg)?;
    let user = Addr::unchecked(message.sender);

    match msg {
        NftReceiveMsg::LockNft { remote_recipient } => {
            let request_id = UNIQUE_PACKETS_REQUEST_ID.load(deps.storage)? + 1;

            //Set timeout and prepare the IBC package
            let current_time = env.block.time.seconds();
            let timeout = current_time + state.ibc_settings.timeout;

            let lock_request = IbcPacketOutgoing {
                packet_type : PacketType::LockRequest {
                    user : user.clone(), //Local user address
                    token_id : message.token_id.clone(),
                    collection : info.sender.to_string(),
                    native_address : remote_recipient
                },
                chain_prefix : state.host_chain_prefix,
                timestamp : current_time,
                request_id : request_id,
            };

            //Prepare the IBC message
            let ibc_message : IbcMsg = IbcMsg::SendPacket {
                channel_id : channel_info.channel_id,
                data: to_json_binary(&lock_request)?,
                timeout: IbcTimeout::with_timestamp(
                    Timestamp::from_seconds(timeout.clone())
                )
            };

            //Save the pending request and send the packet through IBC
            PENDING_PACKETS_REQUESTS.save(deps.storage, (user, request_id), &lock_request)?;
            UNIQUE_PACKETS_REQUEST_ID.save(deps.storage, &request_id)?;

            let mut response = Response::new()
                .add_attribute("lock status", "pending")
                .add_attribute("lock timeout", format!("{}s", timeout))
                .add_attribute("locked token id", message.token_id.clone());

            #[cfg(not(test))]
            {
                // Only in non-test builds, add the IBC message, enables IBC testing
                response = response.add_message(ibc_message);
            }

            Ok(response)
        }
    }
}

/**
 * Init the lock procedure, 
 * 0) Check if the user has an account, and checks if the requested token has been locked by him
 * 1) Create and save the pending request in the state
 * 2) Send the IBC unlock request to the main contract
 */
fn init_unlock_procedure(deps: DepsMut, env: Env, info: MessageInfo, collection: String, token_id: String) -> Result<Response, ContractError> {

    //Ensure the user exists and owns the token
    let user_data = match USERS_DATA.may_load(deps.storage, info.sender.clone()) {
        Ok(Some(data)) => data,
        Ok(None) => return standard_error("User not found".to_string()),
        Err(_e) => return standard_error("User not found".to_string())
    };  

    let user_collection_tokens = user_data.locked_tokens.get(&collection);
    ensure!(user_collection_tokens.is_some_and(|locked_tokens| locked_tokens.contains(&token_id)), ensure_error("The token is not locked in the contract, or not owned by the usre".to_string()));

    //Load state for IBC operation
    let channel_info = CHANNEL.load(deps.storage)?;
    let request_id = UNIQUE_PACKETS_REQUEST_ID.load(deps.storage)? + 1;
    let state = STATE.load(deps.storage)?;

    //Set timeout and prepare the IBC package
    let current_time = env.block.time.seconds();
    let timeout = current_time + state.ibc_settings.timeout;

    let lock_request = IbcPacketOutgoing {
        packet_type : PacketType::UnlockRequest {
            user : user_data.address.clone(),
            token_id : token_id.clone(),
            collection
        },
        chain_prefix : state.host_chain_prefix,
        timestamp : current_time,
        request_id : request_id,
    };

    //Prepare the IBC message
    let ibc_message : IbcMsg = IbcMsg::SendPacket {
        channel_id : channel_info.channel_id,
        data: to_json_binary(&lock_request)?,
        timeout: IbcTimeout::with_timestamp(
            Timestamp::from_seconds(timeout.clone())
        )
    };

    //Save the pending request and send the packet through IBC
    PENDING_PACKETS_REQUESTS.save(deps.storage, (user_data.address.clone(), request_id), &lock_request)?;
    UNIQUE_PACKETS_REQUEST_ID.save(deps.storage, &request_id)?;

    let mut response = Response::new()
            .add_attribute("unlock status", "pending")
            .add_attribute("unlock timeout", format!("{}s", timeout))
            .add_attribute("unlocked token id", token_id.clone());

    #[cfg(not(test))]
    {
        // Only in non-test builds, add the IBC message, enables IBC testing
        response = response.add_message(ibc_message);
    }

    Ok(response)
}