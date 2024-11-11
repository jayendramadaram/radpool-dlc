use dlc_manager::contract::contract_input::ContractInput;
use party::party::Party;
use party::utils::{cleanup, must_load_contract};

mod party;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // clean previous run temporaries
    cleanup();

    // Setup parties
    let (alice, bob) = (
        Party::new("alice".to_string()).await,
        Party::new("bob".to_string()).await,
    );

    // Read contract input
    let contract_path = "./examples/contracts/numeric_outcome.json";
    let contract_input: ContractInput = must_load_contract(contract_path);

    // bob creates offer
    let offer = bob.create_order(contract_input, alice.pubkey).await;

    // Alice accepts and stores offer processed by bob
    let accepted_offer = alice.accept_and_store_offer(offer, bob.pubkey);

    // Bob signs accepted contract
    let signed_msg = bob.sign_accepted_dlc(accepted_offer, alice.pubkey);

    // alice validates contract sigs and cet sigs and broadcasts funding tx
    alice.broadcast_funding_tx(signed_msg, bob.pubkey);

    print!("Funding Tx Broadcasted, please mine 10 blocks to get it confirmed\n");
    let mut required_maturity: u64 = 0;

    loop {
        if required_maturity == 0 {
            let maturity = alice.derive_maturity();
            if maturity == 0 {
                print!("Funding Tx Not Confirmed yet\n");
            } else {
                required_maturity = maturity;
            }
        } else {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            println!(
                "Contract matures at {}, Current time is {}",
                required_maturity, current_time
            );
        }

        bob.process_contracts().await;
        alice.process_contracts().await;
        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
    }

    // Ok(())
}
