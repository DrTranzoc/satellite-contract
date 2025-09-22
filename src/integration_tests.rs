#[cfg(test)]
mod test
{
    use cosmwasm_std::{to_json_binary, Addr, Coin, Empty, Timestamp};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};

    use crate::{contract::{execute, instantiate, migrate, query}, datatypes::{ChannelInfo, CollectionInfo, Cw721ReceiveMsg, IbcSettings, NftReceiveMsg, PacketType}, msg::{ExecuteMsg, InstantiateMsg, QueryMsg}};

    #[test]
    fn test_instantiate_contract() {
        let mut app = mock_app();
        let admin = Addr::unchecked("admin");

        let code_id = app.store_code(contract_satellite());
        let msg = default_instantiate_msg();

        let contract_addr = app.instantiate_contract(
            code_id,
            admin.clone(),
            &msg,
            &[],
            "gamefi_satellite",
            None,
        ).unwrap();

        // You can query state here to confirm successful instantiation
        let state: serde_json::Value = app.wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetState {})
            .unwrap();

        assert_eq!(state["admin"], admin.to_string());
    }

    //IBC messages cannot be relayed with cw_multi_tests, we will only check that a message has been prepared and marked as pending
    #[test]
    fn test_receive_nft_lock_success() {
        let mut app = mock_app();
        let admin = Addr::unchecked("osmo1cw2ap3sxk6yn7j4sgj0zj4qlr30f4pm23enp3v");
        let user = Addr::unchecked("osmo1pw2ap3sxk6yn7j4sgj0zj4qlr30f4pm23enp4v");
        let cw721_contract = Addr::unchecked("osmo1xqw2sl9zk8a6pch0csaw78n4swg5ws8t62wc5qta4gnjxfqg6v2qcs777k");

        let code_id = app.store_code(contract_satellite());
        let msg = default_instantiate_msg();

        let contract_addr = app.instantiate_contract(code_id, admin.clone(), &msg, &[], "test_instantiate", Some(admin.to_string())).unwrap();

        app.update_block(|block| block.time = Timestamp::from_seconds(1_000_000));

        app.contract_storage_mut(&contract_addr).set("channel".as_bytes(), to_json_binary(&ChannelInfo {
            channel_id: "channel-0".to_string(),
            finalized: true,
            opened_at: 1_000_000,
        }).unwrap().as_slice());

        // Send NFT
        let nft_msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: user.to_string(),
            token_id: "1".to_string(),
            msg: to_json_binary(
                    &(NftReceiveMsg::LockNft {  
                        remote_recipient: None
                    })
                ).unwrap(),
        });

        let _ = app.execute_contract(cw721_contract, contract_addr.clone(), &nft_msg, &[]);
        
        let packets: Vec<PacketType> = app.wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetPendingPackets {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        assert!(packets.len() == 1);
    }

    #[test]
    fn test_unlock_token_fail_user_not_found() {
        let mut app = mock_app();
        let admin = Addr::unchecked("osmo1cw2ap3sxk6yn7j4sgj0zj4qlr30f4pm23enp3v");

        let code_id = app.store_code(contract_satellite());
        let msg = default_instantiate_msg();
        let contract_addr = app.instantiate_contract(code_id, admin.clone(), &msg, &[], "test_instantiate", Some(admin.to_string())).unwrap();
        
        // This user hasn't locked anything yet
        let unlock_msg = ExecuteMsg::UnlockToken {
            collection: "osmo1xqw2sl9zk8a6pch0csaw78n4swg5ws8t62wc5qta4gnjxfqg6v2qcs777k".to_string(),
            token_id: "1".to_string(),
            native_address : None
        };

        let result = app.execute_contract(Addr::unchecked("osmo1pw2ap3sxk6yn7j4sgj0zj4qlr30f4pm23enp0v"), contract_addr.clone(), &unlock_msg, &[]);
        assert!(result.is_err());
    }

    fn mock_app() -> App {
        App::default()
    }

    fn contract_satellite() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
        Box::new(contract)
    }

    fn default_instantiate_msg() -> InstantiateMsg {
        let mut collections = Vec::new();
        collections.push(CollectionInfo {
            address: "osmo1xqw2sl9zk8a6pch0csaw78n4swg5ws8t62wc5qta4gnjxfqg6v2qcs777k".to_string(),
        });

        InstantiateMsg {
            collections_info: collections,
            ibc_settings: IbcSettings {
                timeout: 300u64,
                max_timeouts : 3
            },
            host_chain_prefix: "osmo".to_string(),
            lock_credit_settings: crate::datatypes::LockCreditSettings {
                token: Some(Coin {
                    denom: "uosmo".to_string(),
                    amount: 100_000u128.into()
                }),
                credit_per_lock: 1u16
            }
        }
    }
}