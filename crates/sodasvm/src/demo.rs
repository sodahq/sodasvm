use crate::{Sequencer, EmergencyWithdrawalSimulation};
use solana_pubkey::Pubkey;

pub fn run_complete_emergency_demo() {
    println!("\n🎯 SODASVM EMERGENCY WITHDRAWAL DEMO");
    println!("====================================");

    let mut sequencer = Sequencer::new(3);
    let mut emergency_sim = EmergencyWithdrawalSimulation::new();

    println!("\n👥 Setting up users...");
    let alice = Pubkey::new_unique();
    let bob = Pubkey::new_unique();
    let charlie = Pubkey::new_unique();

    sequencer.svm.register_user(alice, 1500);
    sequencer.svm.register_user(bob, 2500);
    sequencer.svm.register_user(charlie, 800);

    println!("✅ Alice: 1500 lamports");
    println!("✅ Bob: 2500 lamports");
    println!("✅ Charlie: 800 lamports");

    println!("\n🔄 Running normal SodaSVM operations...");
    sequencer.simulate_normal_operations(10);

    println!("\n📋 Posting state roots to L1...");
    for commitment in &sequencer.state_roots {
        emergency_sim.l1_contract.post_state_root(commitment.clone());
    }

    println!("\n💥 DISASTER: Sequencer crashes!");
    sequencer.crash();
    println!("❌ SodaSVM is now offline - no new transactions possible");

    println!("\n🆘 Users initiate emergency withdrawals...");

    let alice_proof = sequencer.svm.generate_exit_proof(&alice)
        .expect("Alice should have proof");
    let bob_proof = sequencer.svm.generate_exit_proof(&bob)
        .expect("Bob should have proof");
    let charlie_proof = sequencer.svm.generate_exit_proof(&charlie)
        .expect("Charlie should have proof");

    println!("\n🔍 Verifying proofs and executing withdrawals...");

    emergency_sim.simulate_user_emergency_exit(alice_proof, alice);
    emergency_sim.simulate_user_emergency_exit(bob_proof, bob);
    emergency_sim.simulate_user_emergency_exit(charlie_proof, charlie);

    println!("\n🎉 DEMO COMPLETE!");
    println!("All users successfully recovered their funds using cryptographic proofs!");
    println!("Total recovered: {} users", emergency_sim.l1_contract.withdrawn_users.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_demo() {
        run_complete_emergency_demo();
    }
}