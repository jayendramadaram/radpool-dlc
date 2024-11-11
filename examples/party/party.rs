use dlc::secp256k1_zkp::{rand::thread_rng, SecretKey};
use dlc_manager::contract::contract_input::ContractInput;
use dlc_messages::Message;
use lightning::{
    bitcoin::secp256k1::PublicKey,
    sign::{KeysManager, NodeSigner},
};
use p2pd_oracle_client::P2PDOracleClient;
use std::{
    fs,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use super::utils::{bitcoin_provider_with_defaults, dlc_dir_path, oracles};

pub type DlcManager = Arc<
    Mutex<
        dlc_manager::manager::Manager<
            Arc<bitcoin_rpc_provider::BitcoinCoreProvider>,
            Arc<
                dlc_manager::CachedContractSignerProvider<
                    Arc<bitcoin_rpc_provider::BitcoinCoreProvider>,
                    dlc_manager::SimpleSigner,
                >,
            >,
            Arc<bitcoin_rpc_provider::BitcoinCoreProvider>,
            Box<dlc_sled_storage_provider::SledStorageProvider>,
            Box<P2PDOracleClient>,
            Arc<dlc_manager::SystemTimeProvider>,
            Arc<bitcoin_rpc_provider::BitcoinCoreProvider>,
            dlc_manager::SimpleSigner,
        >,
    >,
>;

pub struct Party {
    pub dlc_manager: DlcManager,
    pub key_manager: Arc<KeysManager>,
    pub pubkey: PublicKey,
}

pub async fn setup(party_name: String) -> (DlcManager, Arc<KeysManager>) {
    let dlc_data_dir = dlc_dir_path(&party_name);
    (
        setup_dlc_manager(party_name).await,
        key_manager(dlc_data_dir.as_str()),
    )
}

impl Party {
    pub async fn new(party_name: String) -> Self {
        let (dlc_manager, key_manager) = setup(party_name).await;
        Self {
            dlc_manager,
            key_manager: key_manager.clone(),
            pubkey: key_manager
                .get_node_id(lightning::sign::Recipient::Node)
                .unwrap(),
        }
    }

    pub async fn create_order(&self, contract_input: ContractInput, alice_pubkey: PublicKey) -> Message {
        // Clone the DlcManager before moving it into the closure
        let dlc_manager_clone = self.dlc_manager.clone();

        // Create offer inside a block to ensure mutex is released
        tokio::task::spawn_blocking(move || {
            let bob_manager = dlc_manager_clone.lock().unwrap();
            Message::Offer(
                bob_manager
                    .send_offer(&contract_input, alice_pubkey)
                    .expect("Error sending offer"),
            )
        })
        .await
        .unwrap()
    }


}

pub async fn setup_dlc_manager(wallet: String) -> DlcManager {
    let bitcoind_provider = bitcoin_provider_with_defaults(&wallet);
    let oracles = oracles().await;

    let dlc_managerr = Arc::new(Mutex::new(
        dlc_manager::manager::Manager::new(
            bitcoind_provider.clone(),
            bitcoind_provider.clone(),
            bitcoind_provider.clone(),
            Box::new(
                dlc_sled_storage_provider::SledStorageProvider::new(&dlc_dir_path(&wallet))
                    .expect("Error creating storage."),
            ),
            oracles,
            Arc::new(dlc_manager::SystemTimeProvider {}),
            bitcoind_provider.clone(),
        )
        .expect("Could not create manager."),
    ));

    dlc_managerr
}

pub fn key_manager(dlc_data_dir: &str) -> Arc<KeysManager> {
    let sk_path = format!("{}/secret_key", dlc_data_dir);
    let sk = SecretKey::new(&mut thread_rng());
    let sk_str = sk.display_secret().to_string();
    fs::write(sk_path, sk_str).expect("Error writing secret key file.");

    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let km = Arc::new(KeysManager::new(
        &sk.secret_bytes(),
        time.as_secs(),
        time.as_nanos() as u32,
    ));

    km
}
