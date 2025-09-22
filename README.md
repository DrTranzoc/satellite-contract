# GameFi Satellite Contract

A CosmWasm-based smart contract for GameFi applications, designed to support local game logic and cross-chain (IBC) interactions. This contract includes CW721 features, extended storage, and an IBC module for interoperability between Cosmos-based chains.

Built for every IBC-powered dAPPS that want to implement custom logic on NFTs without make them leave the home-chain.

---

## ✨ Features

- ✅ CW721 token integration
- ✅ NFT GameFi-generic remote storage and logic
- ✅ IBC (Inter-Blockchain Communication) support
- ✅ Custom execution and query messages
- ✅ Built with `cosmwasm-std 2.2`, `cw-multi-test`, and the latest Cosmos SDK integration
- ✅ Safety measure to emergency unlock NFTs if the IBC relayer appear to be offline
- ✅ Credit system for locks

---

## 📦 Contracts Structure

### `/contract.rs`

Contains the main contract logic:
- `instantiate`: Initializes contract state.
- `execute`: Handles custom logic (e.g. player actions, lock unlock).
- `query`: Returns contract state info.

### `/ibc.rs`

Handles IBC messages:
- `ibc_channel_open`
- `ibc_packet_receive`
- `ibc_packet_ack`
- `ibc_packet_timeout`

Enables safe and verified cross-chain communication.

---

## VERSIONS HASHES

- 0.1 -> 5fa8140e42cd21a011fc6a73e140d066e7a0ecf9b4df6e7fdf87945257bd39ad

## ⚙️ Compile & Optimize

We use the official CosmWasm optimizer Docker image:

```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.0
```



