use crate::datatypes::{ChannelInfo, IbcPacketOutgoing, State, UserData};
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const STATE_KEY: &str = "state";

pub const STATE: Item<State> = Item::new(STATE_KEY);
pub const UNIQUE_PACKETS_REQUEST_ID : Item<u128> = Item::new(&"packets_request_id");
pub const CHANNEL: Item<ChannelInfo> = Item::new(&"channel");

pub const PENDING_PACKETS_REQUESTS : Map<(Addr, u128), IbcPacketOutgoing> = Map::new("packet_requests");
pub const USERS_DATA : Map<Addr, UserData> = Map::new("users_data");

pub const TIMED_OUT_UNLOCK_REQUESTS : Map<(String, Addr), u8> = Map::new("timed_out_unlock_requests");