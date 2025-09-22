use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_storage_plus::Bound;

use crate::{datatypes::{PacketType, State, UserData}, helpers::ensure_error, state::{PENDING_PACKETS_REQUESTS, STATE, USERS_DATA}};

pub(crate) fn get_user_data(deps : Deps, address : String) -> StdResult<UserData> {
    let valid_address = match deps.api.addr_validate(&address) {
        Ok(address) => address,
        Err(_) => return Err(ensure_error("Address not valid".to_string())),
    };

    return Ok(USERS_DATA.load(deps.storage, valid_address)?);
}

pub(crate) fn get_state(deps : Deps) -> StdResult<State> {
    Ok(STATE.load(deps.storage)?)
}

pub(crate) fn get_all_users_data(deps : Deps, start_after : Option<Addr>, limit : Option<u16>) -> StdResult<Vec<UserData>> {

    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(10) as usize;

    Ok(
        USERS_DATA.range(
          deps.storage, 
            start,
            None, 
                Order::Ascending
            )
            .take(limit)
            .map(|x| x.map(|x| x.1))
            .collect::<StdResult<Vec<_>>>()?
    )
}

pub(crate) fn get_token_status(
    deps: Deps,
    user: Addr,
    collection: String,
    token_id: String,
) -> StdResult<String> {
    let user_data = USERS_DATA.load(deps.storage, user.clone())?;

    let pending_user_packets = PENDING_PACKETS_REQUESTS
        .prefix(user.clone())
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| res.map(|(_, packet)| packet.packet_type))
        .collect::<StdResult<Vec<_>>>()?;

    if pending_user_packets.iter().any(|res| res.to_string() == "lock_request") {
        return Ok("lock_in_progres".to_string());
    }

    if pending_user_packets.iter().any(|res| res.to_string() == "unlock_request") {
        return Ok("unlock_in_progres".to_string());
    }

    let token_status = if user_data
        .locked_tokens
        .get(&collection)
        .map_or(false, |tokens| tokens.contains(&token_id))
    {
        "locked"
    } else {
        "unlocked"
    };

    Ok(token_status.to_string())
}

pub(crate) fn get_all_pending_packets(deps : Deps, start_after : Option<(Addr , u128)>, limit : Option<u16>) -> StdResult<Vec<PacketType>> {

    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(10) as usize;

    Ok(
        PENDING_PACKETS_REQUESTS.range(
            deps.storage, 
            start,
            None, 
                Order::Descending
            )
            .take(limit)
            .map(|res| res.map(|(_, packet)| packet.packet_type))
            .collect::<StdResult<Vec<_>>>()?
    )
}

pub(crate) fn get_user_pending_packets(deps : Deps, start_after : Option<u128>, limit : Option<u16>, user: Addr) -> StdResult<Vec<PacketType>> {

    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(10) as usize;

    Ok(
        PENDING_PACKETS_REQUESTS.prefix(user).range(
            deps.storage, 
            start,
            None, 
                Order::Descending
            )
            .take(limit)
            .map(|res| res.map(|(_, packet)| packet.packet_type))
            .collect::<StdResult<Vec<_>>>()?
    )
}