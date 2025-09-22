
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::datatypes::{LockCreditSettings, CollectionInfo, Cw721ReceiveMsg, IbcSettings, PacketType, State, UserData};

#[cw_serde]
pub struct InstantiateMsg {
    pub collections_info : Vec<CollectionInfo>,
    pub ibc_settings : IbcSettings,
    pub host_chain_prefix : String, //e.g orai, osmo, juno, inj etc...
    pub lock_credit_settings : LockCreditSettings
}

#[cw_serde]
pub struct UpdateStatePayload {
    pub collections_info : Option<Vec<CollectionInfo>>,
    pub ibc_settings : Option<IbcSettings>,
    pub admin : Option<Addr>,
    pub host_chain_prefix : Option<String> //e.g orai, osmo, juno, inj etc...
}

#[cw_serde]
pub struct MigrateMsg {

}

#[cw_serde]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    GetCredits {
        amount : u16
    },
    UnlockToken {
        collection : String,
        token_id : String,
        native_address : Option<String>
    },
    UpdateStatePayload {
        state_changes : UpdateStatePayload
    }
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(UserData)]
    GetUserData { 
        address : String
    },
    #[returns(Vec<String>)]
    GetAllUsersData {
        start_after : Option<Addr>,
        limit : Option<u16>
    },
    #[returns(Vec<PacketType>)]
    GetPendingPackets { 
        start_after : Option<(Addr , u128)>,
        limit : Option<u16>
    },
    #[returns(Vec<PacketType>)]
    GetUserPendingPackets { 
        start_after : Option<u128>,
        limit : Option<u16>,
        user : Addr,
    },
    #[returns(State)]
    GetState { },
    #[returns(String)]
    GetTokenStatus {
        user : Addr,
        collection : String,
        token_id : String
    }
}