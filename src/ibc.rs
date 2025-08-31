use std::collections::HashMap;

use cosmwasm_std::{ensure, from_json, Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcOrder, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, StdError};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, DepsMut, Env, StdResult};

use crate::{datatypes::{AckMessage, ChannelInfo, IbcPacketOutgoing, PacketType, UserData}, helpers::send_nft, state::{CHANNEL, PENDING_PACKETS_REQUESTS, STATE, TIMED_OUT_UNLOCK_REQUESTS, USERS_DATA}};

const IBC_APP_VERSION: &str = "gamefi-satellite-protocol-v1";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_open(
    deps: DepsMut,
    env: Env,
    msg: IbcChannelOpenMsg
) -> StdResult<IbcChannelOpenResponse> {
    let channel = msg.channel();

    ensure!(!CHANNEL.exists(deps.storage), StdError::generic_err("channel already exists"));
 
    if channel.order != IbcOrder::Unordered {
        return Err(StdError::generic_err("only un-ordered channels are supported"));
    }
 
    if let Some(counter_version) = msg.counterparty_version() {
        if counter_version != IBC_APP_VERSION {
            return Err(StdError::generic_err(format!(
                "Counterparty version must be `{IBC_APP_VERSION}`"
            )));
        }
    }
 
    CHANNEL.save(deps.storage, &ChannelInfo {
        channel_id: channel.endpoint.channel_id.clone(),
        finalized: false,
        opened_at : env.block.time.seconds()
    })?;
 
    Ok(Some(Ibc3ChannelOpenResponse {
        version: IBC_APP_VERSION.to_string(),
    }))
}
 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    
    let mut channel_info = CHANNEL.load(deps.storage)?;
    ensure!(!channel_info.finalized, StdError::generic_err("channel already finalized"));
    debug_assert_eq!(channel_info.channel_id, channel.endpoint.channel_id, "channel ID mismatch");
 
    // at this point, we are finished setting up the channel and can mark it as finalized
    channel_info.finalized = true;
    CHANNEL.save(deps.storage, &channel_info)?;
 
    Ok(IbcBasicResponse::new())
}
 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    deps: DepsMut,
    _env: Env,
    _msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    CHANNEL.remove(deps.storage);
    Ok(IbcBasicResponse::new())
}

//MAIN IBC LOGIC HERE
 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {

    let ack_packet : AckMessage = from_json(&msg.acknowledgement.data)?;
    let original_packet : IbcPacketOutgoing = from_json(&msg.original_packet.data)?;

    match ack_packet {
        AckMessage::Success { } => {
            match original_packet.packet_type {
                
                //If the LockRequest was successful, concretize the user if it doesn't exist, or 'officially' consider the NFT locked
                PacketType::LockRequest { user, token_id , collection, native_address : _} => {

                    let mut user_data = match USERS_DATA.may_load(deps.storage, user.clone()) {
                        Ok(Some(data)) => data,
                        Ok(None) => UserData { 
                            address : user.clone(),
                            locked_tokens : HashMap::new(),
                            last_lock : 0
                        },
                        Err(err) => { 
                            return Ok(
                                IbcBasicResponse::new()
                                .add_attribute("response", format!("failed to lock token {}, error {}", token_id, err.to_string()))
                                .add_message(send_nft(collection, token_id, user.clone().to_string()))
                            )
                        }
                    };

                    //Concretize lock
                    if let Some(locked_tokens) = user_data.locked_tokens.get_mut(&collection) {
                        locked_tokens.push(token_id.clone());
                    } else {
                        user_data.locked_tokens.insert(collection.clone(), vec![token_id.clone()]);
                    }

                    user_data.last_lock = env.block.time.seconds();
                    
                    //Delete pending request
                    PENDING_PACKETS_REQUESTS.remove(deps.storage, (user.clone(), original_packet.request_id));

                    //Save user data
                    USERS_DATA.save(deps.storage, user.clone(), &user_data)?;

                    return Ok(
                        IbcBasicResponse::new()
                        .add_attribute("response" , format!("lock_token"))
                        .add_attribute("user", user.clone().to_string())
                        .add_attribute("token_id", token_id)
                    )

                },
                PacketType::UnlockRequest { user, token_id , collection, native_address : _} => {

                    //Concretize unlock
                    let mut user_data = USERS_DATA.load(deps.storage, user.clone())?;

                    if let Some(tokens) = user_data.locked_tokens.get_mut(&collection) {
                        tokens.retain(|x| *x != token_id);
                    }

                    PENDING_PACKETS_REQUESTS.remove(deps.storage, (user.clone(), original_packet.request_id));

                    //Save user data and send back the NFT
                    USERS_DATA.save(deps.storage, user.clone(), &user_data)?;

                    //Remove any pending timeout
                    TIMED_OUT_UNLOCK_REQUESTS.remove(deps.storage, (token_id.clone(), user.clone()));

                    return Ok(
                        IbcBasicResponse::new()
                        .add_attribute("response" , "unlock_token")
                        .add_attribute("user", user.clone().to_string())
                        .add_attribute("token_id", token_id.clone())
                        .add_message(send_nft(collection, token_id, user.clone().to_string()))
                    )
                }
            }
        },
        //Handle the controller error flow from the main contract
        AckMessage::Error { error } => {
            match original_packet.packet_type {

                //Restore the ownership of the token, sending it back the the owner.
                PacketType::LockRequest { user, token_id, collection , native_address : _} => {

                    PENDING_PACKETS_REQUESTS.remove(deps.storage, (user.clone(), original_packet.request_id));

                    return Ok(
                        IbcBasicResponse::new()
                        .add_attribute("response", "lock_token_fail")
                        .add_attribute("user", user.clone().to_string())
                        .add_attribute("token_id", token_id.clone())
                        .add_attribute("reason", error)
                        .add_message(send_nft(collection, token_id, user.clone().to_string()))
                    )

                },
                PacketType::UnlockRequest { user, token_id, collection: _ , native_address : _} => {

                    PENDING_PACKETS_REQUESTS.remove(deps.storage, (user.clone(), original_packet.request_id));

                    return Ok(
                        IbcBasicResponse::new()
                        .add_attribute("response", "unlock_token_fail")
                        .add_attribute("user", user.clone().to_string())
                        .add_attribute("token_id", token_id)
                        .add_attribute("reason", error)
                    )
                }
            }
        }
    }
}
 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {

    let original_data : IbcPacketOutgoing = from_json(&msg.packet.data)?;

    match original_data.packet_type {
        PacketType::LockRequest { user, token_id , collection, native_address : _} => {
            
            PENDING_PACKETS_REQUESTS.remove(deps.storage, (user.clone(), original_data.request_id));
            
            Ok(
                IbcBasicResponse::new()
                .add_attribute("reason", "IBC package timeout")
                .add_message(send_nft(collection, token_id, user.clone().to_string()))
            )
        },
        PacketType::UnlockRequest { user, token_id, collection , native_address : _} => {
            let unlock_request_key = (token_id.clone(), user.clone());
            
            let state = STATE.load(deps.storage)?;

            let mut consecutive_timeouts = match TIMED_OUT_UNLOCK_REQUESTS.load(deps.storage, unlock_request_key.clone()) {
                Ok(e) => e,
                Err(_) => 0,
            };

            //After 3 consecutive timeout, the NFT can be unlocked, a relayer issue is a team problem, shouldn't mine the ownership of NFTs
            if consecutive_timeouts == state.ibc_settings.max_timeouts {

                TIMED_OUT_UNLOCK_REQUESTS.remove(deps.storage, unlock_request_key.clone());

                return Ok(
                    IbcBasicResponse::new()
                    .add_attribute("response" , "unlock_token_force")
                    .add_attribute("reason", "max_timeout_reached")
                    .add_attribute("user", user.clone().to_string())
                    .add_attribute("token_id", token_id.clone())
                    .add_message(send_nft(collection, token_id, user.clone().to_string()))
                )

            }

            consecutive_timeouts += 1;

            TIMED_OUT_UNLOCK_REQUESTS.save(deps.storage, unlock_request_key.clone(), &consecutive_timeouts)?;
            PENDING_PACKETS_REQUESTS.remove(deps.storage, (user.clone(), original_data.request_id));
            
            Ok(
                IbcBasicResponse::new()
                .add_attribute("response", format!("failed to unlock token {}", token_id))
                .add_attribute("reason", "IBC package timeout")
            )
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    Ok(IbcReceiveResponse::new(b""))
}