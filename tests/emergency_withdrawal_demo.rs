use sodasvm::{Sequencer};
use solana_pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::collections::HashMap;

#[tokio::test(flavor = "multi_thread")]
async fn test_emergency_withdrawal_demo() {
    println!("\nEmergency Withdrawal System Test");
    println!("================================");

    let alice = Keypair::new();
    let bob = Keypair::new();
    let alice_pubkey = Pubkey::from(alice.pubkey().to_bytes());
    let bob_pubkey = Pubkey::from(bob.pubkey().to_bytes());

    println!("\nStep 1: Users created");
    println!("Alice: {}", alice_pubkey);
    println!("Bob:   {}", bob_pubkey);

    let mut sequencer = Sequencer::new(2);
    let alice_balance = 1_500_000u64;
    let bob_balance = 2_500_000u64;

    sequencer.svm.register_user(alice_pubkey, alice_balance);
    sequencer.svm.register_user(bob_pubkey, bob_balance);

    println!("\nStep 2: Users registered on Layer 2");
    println!("Alice balance: {} lamports", alice_balance);
    println!("Bob balance:   {} lamports", bob_balance);

    println!("\nStep 3: Running normal Layer 2 operations");
    sequencer.simulate_normal_operations(6);
    println!("Processed {} transactions", 6);
    println!("Generated {} state commitments", sequencer.state_roots.len());

    let final_state_root = sequencer.svm.commit_state_root();
    println!("\nStep 4: Final state committed");
    println!("State root: {}", hex::encode(final_state_root));

    println!("\nStep 5: Sequencer crash detected");
    sequencer.crash();
    println!("Sequencer status: OFFLINE");
    println!("Emergency withdrawal mode activated");

    println!("\nStep 6: Generating emergency exit proofs");

    let alice_proof = sequencer.svm.generate_exit_proof(&alice_pubkey)
        .expect("Alice proof generation failed");
    let bob_proof = sequencer.svm.generate_exit_proof(&bob_pubkey)
        .expect("Bob proof generation failed");

    println!("Alice proof:");
    println!("  Balance: {} lamports", alice_proof.account_state.balance);
    println!("  Valid: {}", alice_proof.verify());

    println!("Bob proof:");
    println!("  Balance: {} lamports", bob_proof.account_state.balance);
    println!("  Valid: {}", bob_proof.verify());

    println!("\nStep 7: Emergency vault setup");
    let vault_balance = 10_000_000_000u64;
    println!("Vault balance: {} lamports", vault_balance);

    println!("\nStep 8: Executing emergency withdrawals");

    let mut l1_balances = HashMap::new();
    l1_balances.insert(alice.pubkey(), 1_000_000u64);
    l1_balances.insert(bob.pubkey(), 1_000_000u64);

    println!("Initial L1 balances:");
    println!("  Alice: {} lamports", l1_balances.get(&alice.pubkey()).unwrap());
    println!("  Bob:   {} lamports", l1_balances.get(&bob.pubkey()).unwrap());

    if alice_proof.verify() {
        let alice_l1_balance = l1_balances.get_mut(&alice.pubkey()).unwrap();
        *alice_l1_balance += alice_proof.account_state.balance;
        println!("\nAlice emergency withdrawal: SUCCESS");
        println!("  Recovered: {} lamports", alice_proof.account_state.balance);
        println!("  New L1 balance: {} lamports", alice_l1_balance);
    }

    if bob_proof.verify() {
        let bob_l1_balance = l1_balances.get_mut(&bob.pubkey()).unwrap();
        *bob_l1_balance += bob_proof.account_state.balance;
        println!("\nBob emergency withdrawal: SUCCESS");
        println!("  Recovered: {} lamports", bob_proof.account_state.balance);
        println!("  New L1 balance: {} lamports", bob_l1_balance);
    }

    println!("\nStep 9: Recovery summary");
    let alice_final = l1_balances.get(&alice.pubkey()).unwrap();
    let bob_final = l1_balances.get(&bob.pubkey()).unwrap();
    let total_recovered = (alice_final - 1_000_000) + (bob_final - 1_000_000);

    println!("Alice recovered: {} lamports", alice_final - 1_000_000);
    println!("Bob recovered:   {} lamports", bob_final - 1_000_000);
    println!("Total recovered: {} lamports", total_recovered);
    println!("Original L2 total: {} lamports", alice_balance + bob_balance);

    assert_eq!(total_recovered, alice_balance + bob_balance);
    println!("\nEmergency withdrawal test: PASSED");
    println!("All user funds successfully recovered");
}