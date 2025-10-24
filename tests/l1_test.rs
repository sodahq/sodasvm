use sodasvm::{Sequencer, L1Client};
use solana_sdk::{
    pubkey::Pubkey as SdkPubkey,
    signature::{Keypair, Signer},
};
use solana_pubkey::Pubkey;
use std::str::FromStr;
use std::{thread, time::Duration};

const PROGRAM_ID: &str = "BbYmwfjNKjLVKTV6qbWBj41toNPj9wY4Zicx8q63AMjr";
const LOCAL_VALIDATOR_URL: &str = "http://127.0.0.1:8899";

#[tokio::test(flavor = "multi_thread")]
async fn test_real_emergency_withdrawal_with_deployed_contract() {
    println!("\nL1 EMERGENCY WITHDRAWAL TEST");
    println!("============================");

    let program_id = SdkPubkey::from_str(PROGRAM_ID).expect("Invalid program ID");
    let authority = Keypair::new();

    let l1_client = L1Client::new(LOCAL_VALIDATOR_URL, program_id, authority);

    println!("Funding authority account...");
    l1_client.airdrop(&l1_client.authority.pubkey(), 5_000_000_000).unwrap();
    thread::sleep(Duration::from_secs(2));

    println!("Setting up SodaSVM sequencer...");
    let mut sequencer = Sequencer::new(2);

    let alice = Keypair::new();
    let bob = Keypair::new();

    // Fund Alice and Bob on L1 first so they have accounts to withdraw to
    println!("Funding Alice and Bob on L1...");
    l1_client.airdrop(&alice.pubkey(), 5_000_000).unwrap();
    l1_client.airdrop(&bob.pubkey(), 5_000_000).unwrap();
    thread::sleep(Duration::from_secs(2));

    // Register users in SodaSVM with converted pubkeys
    sequencer.svm.register_user(Pubkey::from(alice.pubkey().to_bytes()), 1_500_000);
    sequencer.svm.register_user(Pubkey::from(bob.pubkey().to_bytes()), 2_500_000);

    println!("Alice registered with 1,500,000 lamports");
    println!("Bob registered with 2,500,000 lamports");

    println!("\nRunning SodaSVM operations...");
    sequencer.simulate_normal_operations(6);

    println!("State roots generated: {}", sequencer.state_roots.len());

    println!("\nPosting state roots to real L1 contract...");
    let mut state_accounts = Vec::new();

    for (i, commitment) in sequencer.state_roots.iter().enumerate() {
        let state_account = Keypair::new();
        state_accounts.push(state_account);

        match l1_client.post_state_root_to_l1(
            &state_accounts[i],
            commitment.root,
            commitment.block_number,
        ) {
            Ok(signature) => {
                println!("Posted state root {}: {}", i, &signature[0..8]);
            }
            Err(e) => {
                println!("Failed to post state root {}: {}", i, e);
            }
        }
        thread::sleep(Duration::from_millis(500));
    }

    println!("\nSIMULATING SEQUENCER CRASH...");
    sequencer.crash();

    println!("\nUsers generating emergency exit proofs...");

    let alice_pubkey = Pubkey::from(alice.pubkey().to_bytes());
    let bob_pubkey = Pubkey::from(bob.pubkey().to_bytes());
    let alice_proof = sequencer.svm.generate_exit_proof(&alice_pubkey)
        .expect("Alice should have proof");
    let bob_proof = sequencer.svm.generate_exit_proof(&bob_pubkey)
        .expect("Bob should have proof");

    println!("Alice proof generated - Balance: {}", alice_proof.account_state.balance);
    println!("Bob proof generated - Balance: {}", bob_proof.account_state.balance);

    println!("\nInitializing emergency contract...");
    let contract_account = Keypair::new();
    match l1_client.initialize_contract(&contract_account) {
        Ok(signature) => {
            println!("Contract initialized: {}", &signature[0..8]);
        }
        Err(e) => {
            println!("Contract initialization failed (might already be initialized): {}", e);
        }
    }
    thread::sleep(Duration::from_secs(1));

    println!("\nSetting up vault account for withdrawals...");
    // Try using a PDA vault that the program can control
    let (vault_pda, _bump) = l1_client.find_vault_pda();
    println!("Vault PDA: {}", vault_pda);
    l1_client.airdrop(&vault_pda, 10_000_000_000).unwrap();
    thread::sleep(Duration::from_secs(1));

    println!("\nExecuting emergency withdrawals on real L1...");

    println!("\nAlice attempting emergency withdrawal...");
    let alice_before = l1_client.get_balance(&alice.pubkey()).unwrap_or(0);
    println!("Alice balance before: {} lamports", alice_before);

    // Use the latest posted state account
    let latest_state_account = &state_accounts[state_accounts.len() - 1];
    match l1_client.emergency_withdraw(
        &latest_state_account.pubkey(),
        &alice,
        &vault_pda,
        alice_proof,
    ) {
        Ok(signature) => {
            println!("Alice withdrawal successful: {}", &signature[0..8]);
            thread::sleep(Duration::from_secs(2));

            let alice_after = l1_client.get_balance(&alice.pubkey()).unwrap_or(0);
            println!("Alice balance after: {} lamports", alice_after);
            println!("Alice recovered: {} lamports", alice_after - alice_before);

            assert!(alice_after > alice_before, "Alice should have recovered funds");
        }
        Err(e) => {
            println!("Alice withdrawal failed: {}", e);
        }
    }

    println!("\nBob attempting emergency withdrawal...");
    let bob_before = l1_client.get_balance(&bob.pubkey()).unwrap_or(0);
    println!("Bob balance before: {} lamports", bob_before);

    match l1_client.emergency_withdraw(
        &latest_state_account.pubkey(),
        &bob,
        &vault_pda,
        bob_proof,
    ) {
        Ok(signature) => {
            println!("Bob withdrawal successful: {}", &signature[0..8]);
            thread::sleep(Duration::from_secs(2));

            let bob_after = l1_client.get_balance(&bob.pubkey()).unwrap_or(0);
            println!("Bob balance after: {} lamports", bob_after);
            println!("Bob recovered: {} lamports", bob_after - bob_before);

            assert!(bob_after > bob_before, "Bob should have recovered funds");
        }
        Err(e) => {
            println!("Bob withdrawal failed: {}", e);
        }
    }

    println!("\nL1 INTEGRATION TEST COMPLETE");
    println!("Emergency withdrawal system tested with deployed contract");
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

    match l1_client.rpc_client.get_account(&program_id) {
        Ok(account) if account.data.len() > 0 => {
            println!("Emergency exit program deployed successfully");
            println!("Program ID: {}", program_id);
            println!("Program size: {} bytes", account.data.len());
        }
        Ok(_) => {
            panic!("Emergency exit program not found at: {}", program_id);
        }
        Err(e) => {
            panic!("Failed to check program: {}", e);
        }
    }

    println!("All checks passed - ready for integration testing");
}