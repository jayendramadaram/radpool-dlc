use std::str::FromStr;
use tokio::time::{interval, Duration};
use dlc::secp256k1_zkp::XOnlyPublicKey;
use dlc_manager::contract::contract_input::ContractInput;
use party::party::Party;
use party::utils::cleanup;
use radpool_dlc::contract::contract_builder::ContractBuilder;
use radpool_dlc::contract::descriptor_builder::NumericalDescriptorBuilder;

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

    // Read contract input, Built contract should match contract from ./examples/contracts/numeric_outcome.json
    let contract_input: ContractInput = build_contract_with_defaults();

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

    let mut interval = interval(Duration::from_secs(5));

    loop {
        // Synchronize with the interval
        interval.tick().await;

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

        // Process contracts in parallel
        tokio::join!(
            bob.process_contracts(),
            alice.process_contracts()
        );
    }

    // Ok(())
}

fn build_contract_with_defaults() -> ContractInput {
    let pubkey = XOnlyPublicKey::from_str(
        "0d829c1cc556aa59060df5a9543c5357199ace5db9bcd5a8ddd6ee2fc7b6d174",
    )
    .unwrap();
    let current_unix_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + 120;

    let descriptor_with_payouts = NumericalDescriptorBuilder::new()
        .add_payout_point(1, 0, 0, 0)
        .add_payout_point(1, 50000, 0, 0)
        .add_payout_point(2, 50000, 0, 0)
        .add_payout_point(2, 60000, 200000000, 0)
        .add_payout_point(3, 60000, 200000000, 0)
        .add_payout_point(3, 1048575, 200000000, 0);

    let descriptor = descriptor_with_payouts
        .add_rounding_interval(0, 1)
        .set_oracle_numeric_info(2, vec![20])
        .build()
        .unwrap();

    let contract_info = ContractBuilder::create_contract_info(
        dlc_manager::contract::ContractDescriptor::Numerical(descriptor),
        vec![pubkey],
        format!("btcusd{}", current_unix_time),
        1,
    );

    let contract = ContractBuilder::new()
        .fee_rate(2)
        .offer_collateral(100000000)
        .accept_collateral(100000000)
        .with_contract_info(contract_info.unwrap())
        .build();

    contract.unwrap()
}
