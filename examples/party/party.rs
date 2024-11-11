use dlc::secp256k1_zkp::{rand::thread_rng, SecretKey};
use dlc_manager::contract::{contract_input::ContractInput, Contract};
use dlc_manager::Storage;
use dlc_messages::{AcceptDlc, Message};
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
use crate::party::utils::{hex_str, offers_path};

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
    pub pubkey: PublicKey,
    pub user: String,
}

/// Sets up the necessary objects for a Party to function.
///
/// Provides a DlcManager and a KeysManager for a Party.
pub async fn setup(party_name: &str) -> (DlcManager, Arc<KeysManager>) {
    let dlc_data_dir = dlc_dir_path(&party_name);
    (
        setup_dlc_manager(party_name).await,
        key_manager(dlc_data_dir.as_str()),
    )
}

impl Party {
    /// Creates a new instance of `Party`.
    ///
    /// This asynchronous function initializes a `Party` by setting up
    pub async fn new(party_name: String) -> Self {
        let (dlc_manager, key_manager) = setup(&party_name).await;

        print!("Party {} created\n", party_name);
        Self {
            dlc_manager,
            pubkey: key_manager
                .get_node_id(lightning::sign::Recipient::Node)
                .unwrap(),
            user: party_name,
        }
    }

    /// Creates an offer for counter_party.
    ///
    /// This function is blocking, creates contract, and gets announcements from the oracle.
    /// todo: dlc_manager lock can be moved into a helped function
    pub async fn create_order(
        &self,
        contract_input: ContractInput,
        counter_party: PublicKey,
    ) -> Message {
        let dlc_mngr_clone = self.dlc_manager.clone();

        // Create offer and get announcements from oracle
        let msg = tokio::task::spawn_blocking(move || {
            let dlc_mngr_locked = dlc_mngr_clone.lock().unwrap();
            Message::Offer(
                dlc_mngr_locked
                    .send_offer(&contract_input, counter_party)
                    .expect("Error sending offer"),
            )
        })
        .await
        .unwrap();

        println!("Offer created for {}! by {}", counter_party , self.user);

        msg
    }

    /// Processes and stores a received offer to sledstorage.
    ///  
    ///  returns an `AcceptDlc` message.
    pub fn accept_and_store_offer(&self, offer: Message, counter_party: PublicKey) -> AcceptDlc {
        let dlc_mngr_clone = self.dlc_manager.clone();
        let dlc_mngr_locked = dlc_mngr_clone.lock().unwrap();

        // store offer
        dlc_mngr_locked
            .on_dlc_message(&offer, counter_party)
            .expect("Error processing offer");

        let offers = dlc_mngr_locked.get_store().get_contract_offers().unwrap();

        // Only one offer should exist
        assert!(offers.len() == 1);

        // load offer and write offer to file
        let offer = offers.get(0).unwrap();
        let offer_id = hex_str(&offer.id);
        let offer_json_path = format!("{}/{}.json", offers_path(self.user.as_str()), offer_id);

        if fs::metadata(&offer_json_path).is_err() {
            let offer_str = serde_json::to_string_pretty(&offer).unwrap();
            fs::write(&offer_json_path, offer_str).unwrap();
        }

        println!("Offer Stored for {}! by {}", counter_party , self.user);

        // accept offer
        let (_, _, msg) = {
            dlc_mngr_locked
                .accept_contract_offer(&offer.id)
                .expect("Error accepting offer")
        };

        println!("Offer Accepted {:?} from {} by {}", offer_id, offer.counter_party , self.user);

        msg
    }

    /// Sign Funding and CETS for an accepted DLC offer and returns a Message.
    ///
    /// todo: uses unwrap for now.
    pub fn sign_accepted_dlc(&self, msg: AcceptDlc, counter_party: PublicKey) -> Message {
        let dlc_mngr_clone = self.dlc_manager.clone();
        let dlc_mngr_locked = dlc_mngr_clone.lock().unwrap();

        let resp: Option<Message> = {
            dlc_mngr_locked
                .on_dlc_message(&Message::Accept(msg), counter_party)
                .expect("Error processing offer")
        };

        println!("Contract  Signed by {}", self.user);
        resp.unwrap()
    }

    /// Broadcast the funding transaction for an accepted DLC offer.
    ///
    /// Note: This function should be called after calling `sign_accepted_dlc` and
    /// should be called by the party that accepted the DLC offer.
    ///
    /// verifies signed contract and broadcast funding tx
    pub fn broadcast_funding_tx(&self, msg: Message, counter_party: PublicKey) {
        let dlc_mngr_clone = self.dlc_manager.clone();
        let dlc_mngr_locked = dlc_mngr_clone.lock().unwrap();

        dlc_mngr_locked
            .on_dlc_message(&msg, counter_party)
            .expect("Error processing offer");

        println!("Funding Tx Broadcasted by {}", self.user);
    }

    /// Derives the maturity epoch for a signed contract.
    pub fn derive_maturity(&self) -> u64 {
        let dlc_mngr_clone = self.dlc_manager.clone();
        let dlc_mngr_locked = dlc_mngr_clone.lock().unwrap();

        // there would only be single signed contract accourding to our execution flow
        for contract in dlc_mngr_locked
            .get_store()
            .get_confirmed_contracts()
            .unwrap()
        {
            let contract_infos = &contract.accepted_contract.offered_contract.contract_info;
            for info in contract_infos {
                for announcement in &info.oracle_announcements {
                    return announcement.oracle_event.event_maturity_epoch as u64;
                }
            }
        }

        0
    }

    /// checks for the state changes happened in existing contract and tries to move to next state.
    /// prints the contract state.
    pub async fn process_contracts(&self) {
        let mngr = self.dlc_manager.clone();

        tokio::task::spawn_blocking(move || {
            mngr.lock()
                .unwrap()
                .periodic_check(true)
                .expect("Error doing periodic check.");
            let contracts = mngr
                .lock()
                .unwrap()
                .get_store()
                .get_contracts()
                .expect("Error retrieving contract list.");
            for contract in contracts {
                let id = hex_str(&contract.get_id());
                match contract {
                    Contract::Offered(_) => {
                        println!("Offered contract: {}", id);
                    }
                    Contract::Accepted(_) => {
                        println!("Accepted contract: {}", id);
                    }
                    Contract::Confirmed(_) => {
                        println!("Confirmed contract: {}", id);
                    }
                    Contract::Signed(_) => {
                        println!("Signed contract: {}", id);
                    }
                    Contract::Closed(closed) => {
                        println!("Closed contract: {}", id);
                        if let Some(attestations) = closed.attestations {
                            println!(
                                "Outcomes: {:?}",
                                attestations
                                    .iter()
                                    .map(|x| x.outcomes.clone())
                                    .collect::<Vec<_>>()
                            );
                        }
                        println!("PnL: {} sats", closed.pnl)
                    }
                    Contract::Refunded(_) => {
                        println!("Refunded contract: {}", id);
                    }
                    Contract::FailedAccept(_) | Contract::FailedSign(_) => {
                        println!("Failed contract: {}", id);
                    }
                    Contract::Rejected(_) => println!("Rejected contract: {}", id),
                    Contract::PreClosed(_) => println!("Pre-closed contract: {}", id),
                }
            }
        })
        .await
        .expect("Error listing contract info");
    }
}

/// set up dlc manager
pub async fn setup_dlc_manager(wallet: &str) -> DlcManager {
    let bitcoind_provider = bitcoin_provider_with_defaults(wallet);
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

/// set up key manager
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
