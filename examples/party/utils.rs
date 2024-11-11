use std::{collections::HashMap, fs, path::Path, sync::Arc};
use dlc::secp256k1_zkp::XOnlyPublicKey;
use p2pd_oracle_client::P2PDOracleClient;
use dlc_manager::{contract::contract_input::ContractInput, Oracle};
use std::fmt::Write;



pub fn bitcoin_provider_with_defaults(wallet: &String) -> Arc<bitcoin_rpc_provider::BitcoinCoreProvider> {
    Arc::new(
        bitcoin_rpc_provider::BitcoinCoreProvider::new(
            "localhost".to_string(),
            18443,
            Some(wallet.to_string()),
            "testuser".to_string(),
            "lq6zequb-gYTdF2_ZEUtr8ywTXzLYtknzWU4nV8uVoo=".to_string(),
        )
        .expect("Error creating BitcoinCoreProvider"))
}

pub async fn oracles() -> HashMap<XOnlyPublicKey, Box<P2PDOracleClient>>{
    let oracle_host = "http://localhost:8080/";
    let oracle = tokio::task::spawn_blocking(move || {
        P2PDOracleClient::new(&oracle_host).expect("Error creating oracle client")
    })
    .await
    .unwrap();

    let mut oracles = HashMap::new();
    oracles.insert(oracle.get_public_key(), Box::new(oracle));

    oracles
}



pub fn hex_str(value: &[u8]) -> String {
    let mut res = String::with_capacity(64);
    for v in value {
        write!(res, "{:02x}", v).unwrap();
    }
    res
}

pub fn offers_path(wallet : &str) -> String {
    let offers_path = format!("{}/{}", dlc_dir_path(wallet), "offers");
    
    if !Path::new(&offers_path).exists() {
        fs::create_dir_all(&offers_path).expect("Error creating offers directory.");
    }

    offers_path
}

pub fn dlc_dir_path(wallet : &str) -> String {
    let dlc_storage_dir_path = format!("./temp/{}_{}", &wallet, "dlc_storage");
    if !Path::new(&dlc_storage_dir_path).exists() {
        fs::create_dir_all(&dlc_storage_dir_path).expect("Error creating dlc storage directory.");
    }
    dlc_storage_dir_path
} 


pub fn cleanup() {
    let _ = fs::remove_dir_all("./temp");
}

pub fn must_load_contract(contract_path : &str) -> ContractInput {
    let contract_input_str = fs::read_to_string(contract_path).unwrap();
    serde_json::from_str(&contract_input_str).unwrap()
}