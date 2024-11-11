use std::fs;

use dlc_manager::contract::contract_input::ContractInput;
use dlc_manager::contract::{self, Contract};
use dlc_manager::Storage;
use dlc_messages::Message;
use lightning::sign::NodeSigner;
use party::party::{setup, DlcManager};
use party::utils::{cleanup, hex_str, offers_path, };

mod party;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // clean previous run temporaries
    cleanup();

    // Setup parties
    let (alice_dlc_mngr, alice_key_mngr) = setup("alice".to_string()).await;
    let (bob_dlc_mngr, bob_key_mngr) = setup("bob".to_string()).await;

    let (alice_pubkey, bob_pubkey) = (
        alice_key_mngr
            .get_node_id(lightning::sign::Recipient::Node)
            .unwrap(),
        bob_key_mngr
            .get_node_id(lightning::sign::Recipient::Node)
            .unwrap(),
    );

    // Read contract input
    let contract_path = "./examples/contracts/numeric_outcome.json";
    let contract_input_str = fs::read_to_string(contract_path)?;
    let contract_input: ContractInput = serde_json::from_str(&contract_input_str)?;

    // Clone bob_dlc_mngr before moving into closure
    let bob_dlc_mngr_clone = bob_dlc_mngr.clone();

    let offer = tokio::task::spawn_blocking(move || {
        let binding = bob_dlc_mngr_clone; // Use the clone instead
        let bob_manager = binding.lock().unwrap();
        Message::Offer(
            bob_manager
                .send_offer(&contract_input, alice_pubkey)
                .expect("Error sending offer"),
        )
    })
    .await
    .unwrap();

    println!("Offer Sent To: Alice");

    // Process offer in a separate block
    {
        let alice_dlc_mngr = alice_dlc_mngr.clone();
        let alice_manager = alice_dlc_mngr.lock().unwrap();
        alice_manager
            .on_dlc_message(&offer, bob_pubkey)
            .expect("Error processing offer");
    }

    // let maturity_time:

    // List offers in a separate block
    {
        // Acquire and release locks in smaller scopes
        let offers = {
            let alice_manager = alice_dlc_mngr.lock().unwrap();
            alice_manager.get_store().get_contract_offers().unwrap()
        };

        for offer in offers.iter().filter(|x| !x.is_offer_party) {
            let offer_id = hex_str(&offer.id);

            let offer_json_path = format!("{}/{}.json", offers_path("alice"), offer_id);

            if fs::metadata(&offer_json_path).is_err() {
                let offer_str = serde_json::to_string_pretty(&offer)?;
                fs::write(&offer_json_path, offer_str)?;
            }

            println!("Offer {:?} from {}", offer_id, offer.counter_party);

            // Acquire alice lock for accepting offer
            let (_, node_id, msg) = {
                let alice_manager = alice_dlc_mngr.lock().unwrap();
                alice_manager
                    .accept_contract_offer(&offer.id)
                    .expect("Error accepting offer")
            };

            println!("Offer {} Accepted", offer_id);

            // Process Bob's response in a separate scope
            let resp = {
                let bob_manager = bob_dlc_mngr.lock().unwrap();
                bob_manager
                    .on_dlc_message(&Message::Accept(msg), alice_pubkey)
                    .expect("Error processing offer")
            };

            print!("Bob Signs Contract: ");

            // Process Bob's response with Alice in a separate scope
            if let Some(msg) = resp {
                let alice_manager = alice_dlc_mngr.lock().unwrap();
                alice_manager
                    .on_dlc_message(&msg, bob_pubkey)
                    .expect("Error processing offer");
            }

            println!("Contract Signed, Time for Alice to broadcast funding sig");
        }
    }

    loop {
        // tokio::select! {
        //     _ =  => {},
        //     _ = process_contracts(alice_dlc_mngr.clone()) => {},
        //     _ =  => {},
        // }
        
        check_maturity(alice_dlc_mngr.clone()).await;
        process_contracts(bob_dlc_mngr.clone()).await;
        process_contracts(alice_dlc_mngr.clone()).await;
        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;

        // let current_time = std::time::SystemTime::now()
        //     .duration_since(std::time::UNIX_EPOCH)
        //     .unwrap()
        //     .as_secs();
        // println!("Current Unix time:{} secs\n\n", current_time,);

        // if maturity_time > current_time {
        //     println!("Time Left: {} Secs", maturity_time - current_time);
        // } else {
        //     println!("Time Passed By: {} Secs", current_time - maturity_time);
        // }
    }

    // Ok(())
}


pub async fn check_maturity(mngr: DlcManager) {
    let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    for contract in mngr.lock().unwrap().get_store().get_confirmed_contracts().unwrap() {
        
        let contract_infos = &contract.accepted_contract.offered_contract.contract_info;
        for info in contract_infos {
            for announcement in &info.oracle_announcements {
            println!("maturity time {} and current time {} \n\n", announcement.oracle_event.event_maturity_epoch as u64, current_time);
                if (announcement.oracle_event.event_maturity_epoch as u64) <=  current_time{
                    println!("Time Passed By: {} Secs", current_time - announcement.oracle_event.event_maturity_epoch as u64);
                } else {
                    println!("Time Left: {} Secs", announcement.oracle_event.event_maturity_epoch as u64 - current_time);
                }
            }
        }
        
    }
}

pub async fn process_contracts(mngr: DlcManager) {
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
