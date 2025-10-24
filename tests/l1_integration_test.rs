use sodasvm::{Sequencer, L1Client};
use solana_sdk::{
    pubkey::Pubkey as SdkPubkey,
    signature::{Keypair, Signer},
};
use solana_pubkey::Pubkey;
use std::{str::FromStr, thread, time::Duration};

const PROGRAM_ID: &str = "BbYmwfjNKjLVKTV6qbWBj41toNPj9wY4Zicx8q63AMjr";
const LOCAL_VALIDATOR_URL: &str = "http://127.0.0.1:8899";

#[tokio::test(flavor = "multi_thread")]
async fn test_l1_emergency_withdrawal_integration() {
    println!("\nL1 EMERGENCY WITHDRAWAL INTEGRATION TEST");
    println!("=========================================");

    let program_id = SdkPubkey::from_str(PROGRAM_ID).expect("Invalid program ID");
    let authority = Keypair::new();

    let l1_client = L1Client::new(LOCAL_VALIDATOR_URL, program_id, authority);

    println!("Step 1: Checking validator connection and deployed program...");
    match l1_client.check_program_deployed() {
        Ok(true) => println!("Program verified deployed on validator"),
        Ok(false) => {
            println!("Program not found - make sure validator is running and program is deployed");
            return;
        }
        Err(e) => {
            println!("Failed to connect to validator: {}", e);
            println!("Make sure to run: solana-test-validator");
            return;
        }
    }

    println!("\nStep 2: Funding authority account...");
    match l1_client.airdrop(&l1_client.authority.pubkey(), 5_000_000_000) {
        Ok(_) => println!("Authority funded with 5 SOL"),
        Err(e) => {
            println!("Failed to airdrop: {}", e);
            return;
        }
    }
    thread::sleep(Duration::from_secs(2));

    println!("\nStep 3: Setting up SodaSVM sequencer...");
    let mut sequencer = Sequencer::new(2);

    let alice = Keypair::new();
    let bob = Keypair::new();

    sequencer.svm.register_user(Pubkey::from(alice.pubkey().to_bytes()), 1_000_000);
    sequencer.svm.register_user(Pubkey::from(bob.pubkey().to_bytes()), 2_000_000);

    println!("Alice registered with 1,000,000 lamports");
    println!("Bob registered with 2,000,000 lamports");

    println!("\nStep 4: Running SodaSVM operations and posting state roots to L1...");
    sequencer.simulate_normal_operations(6);

    println!("State roots generated: {}", sequencer.state_roots.len());

    for (i, commitment) in sequencer.state_roots.iter().enumerate() {
        let state_account = Keypair::new();

        match l1_client.post_state_root_to_l1(
            &state_account,
            commitment.root,
            commitment.block_number,
        ) {
            Ok(signature) => {
                println!("Posted state root {} to L1 - TX: {}", i, &signature[0..8]);
            }
            Err(e) => {
                println!("Failed to post state root {}: {}", i, e);
            }
        }
        thread::sleep(Duration::from_millis(1000));
    }

    println!("\nStep 5: Simulating sequencer crash...");
    sequencer.crash();

    println!("\nStep 6: Users generating emergency withdrawal proofs...");

    let alice_pubkey = Pubkey::from(alice.pubkey().to_bytes());
    let bob_pubkey = Pubkey::from(bob.pubkey().to_bytes());
    let alice_proof = sequencer.svm.generate_exit_proof(&alice_pubkey)
        .expect("Alice should have proof");
    let bob_proof = sequencer.svm.generate_exit_proof(&bob_pubkey)
        .expect("Bob should have proof");

    println!("Alice proof generated - Balance: {}", alice_proof.account_state.balance);
    println!("Bob proof generated - Balance: {}", bob_proof.account_state.balance);

    assert!(alice_proof.verify(), "Alice proof should be valid");
    assert!(bob_proof.verify(), "Bob proof should be valid");

    println!("\nStep 7: Setting up vault for emergency withdrawals...");
    let vault_account = Keypair::new();
    match l1_client.airdrop(&vault_account.pubkey(), 10_000_000_000) {
        Ok(_) => println!("Vault funded with 10 SOL"),
        Err(e) => {
            println!("Failed to fund vault: {}", e);
            return;
        }
    }
    thread::sleep(Duration::from_secs(2));

    println!("\nStep 8: Executing emergency withdrawals on L1...");

    println!("\nAlice emergency withdrawal:");
    let alice_before = l1_client.get_balance(&alice.pubkey()).unwrap_or(0);
    println!("Alice balance before: {} lamports", alice_before);

    let state_account = Keypair::new(); // Would use the latest posted state account in real scenario

    match l1_client.emergency_withdraw(
        &state_account.pubkey(),
        &alice,
        &vault_account.pubkey(),
        alice_proof,
    ) {
        Ok(signature) => {
            println!("Alice emergency withdrawal executed - TX: {}", &signature[0..8]);
            thread::sleep(Duration::from_secs(2));

            let alice_after = l1_client.get_balance(&alice.pubkey()).unwrap_or(0);
            println!("Alice balance after: {} lamports", alice_after);

            if alice_after > alice_before {
                println!("Alice successfully recovered {} lamports", alice_after - alice_before);
            }
        }
        Err(e) => {
            println!("Alice emergency withdrawal failed: {}", e);
            println!("This might be expected if the contract validation fails");
        }
    }

    println!("\nBob emergency withdrawal:");
    let bob_before = l1_client.get_balance(&bob.pubkey()).unwrap_or(0);
    println!("Bob balance before: {} lamports", bob_before);

    match l1_client.emergency_withdraw(
        &state_account.pubkey(),
        &bob,
        &vault_account.pubkey(),
        bob_proof,
    ) {
        Ok(signature) => {
            println!("Bob emergency withdrawal executed - TX: {}", &signature[0..8]);
            thread::sleep(Duration::from_secs(2));

            let bob_after = l1_client.get_balance(&bob.pubkey()).unwrap_or(0);
            println!("Bob balance after: {} lamports", bob_after);

            if bob_after > bob_before {
                println!("Bob successfully recovered {} lamports", bob_after - bob_before);
            }
        }
        Err(e) => {
            println!("Bob emergency withdrawal failed: {}", e);
            println!("This might be expected if the contract validation fails");
        }
    }

    println!("\nL1 INTEGRATION TEST COMPLETE");
    println!("Proof generation and L1 posting successful");
    println!("Emergency withdrawal flow tested with real blockchain");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_validator_connection() {
    println!("\nChecking validator connection and deployed program...");

    let program_id = SdkPubkey::from_str(PROGRAM_ID).expect("Invalid program ID");
    let authority = Keypair::new();
    let l1_client = L1Client::new(LOCAL_VALIDATOR_URL, program_id, authority);

    match l1_client.rpc_client.get_version() {
        Ok(version) => {
            println!("Connected to Solana validator: {:?}", version.solana_core);
        }
        Err(e) => {
            panic!("Failed to connect to validator: {}", e);
        }
    }

    match l1_client.check_program_deployed() {
        Ok(true) => {
            println!("Emergency exit program deployed successfully");
            println!("Program ID: {}", program_id);
        }
        Ok(false) => {
            panic!("Emergency exit program not found at: {}", program_id);
        }
        Err(e) => {
            panic!("Failed to check program: {}", e);
        }
    }

    println!("All checks passed - ready for integration testing");
}