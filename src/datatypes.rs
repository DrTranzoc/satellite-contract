use std::{collections::HashMap, fmt};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub collections_info : Vec<CollectionInfo>,
    pub admin : Addr,
    pub ibc_settings : IbcSettings,
    pub host_chain_prefix : String //e.g orai, osmo, juno, inj etc...
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct IbcSettings {
    pub timeout : u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CollectionInfo {
    pub address : String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserData {
    pub address : Addr,
    pub locked_tokens : HashMap<String, Vec<String>>,
    pub last_lock : u64
}

#[cw_serde]
pub struct IbcPacketOutgoing {
    pub request_id : u128,
    pub timestamp : u64,
    pub chain_prefix : String,
    pub packet_type : PacketType
}

#[cw_serde]
pub struct IbcPacketIncoming {
    pub request_id : u128,
    pub timestamp : u64,
    pub chain_prefix : String,
    pub message : AckMessage
}

#[cw_serde]
pub enum PacketType {
    LockRequest { 
        user : Addr,
        token_id : String,
        collection : String,
        native_address : Option<String> // Optional native address for the token, if applicable, mainly used for Injective as it doesn't have bech32 addresses
    },
    UnlockRequest { 
        user : Addr,
        token_id : String,
        collection : String
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Cw721ReceiveMsg {
    pub sender: String,
    pub token_id: String,
    pub msg: Binary,
}

impl fmt::Display for PacketType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match self {
           PacketType::LockRequest { .. }  => write!(f, "lock_request"),
           PacketType::UnlockRequest { .. } => write!(f, "unlock_request"),
       }
    }
}

#[cw_serde]
pub enum AckMessage {
    Error {
        error : String
    },
    Success {

    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum NftReceiveMsg {
    LockNft {
        remote_recipient: Option<String>
    }
}

#[cw_serde]
pub struct ChannelInfo {
    pub channel_id: String,
    pub finalized: bool,
    pub opened_at : u64
}