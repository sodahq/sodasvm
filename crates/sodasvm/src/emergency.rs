use crate::{MerkleProof, StateCommitment};
use solana_pubkey::Pubkey;
use std::collections::HashMap;

pub struct MockL1Contract {
    pub stored_roots: Vec<StateCommitment>,
    pub user_balances: HashMap<Pubkey, u64>,
    pub withdrawn_users: Vec<Pubkey>,
}

impl MockL1Contract {
    pub fn new() -> Self {
        Self {
            stored_roots: Vec::new(),
            user_balances: HashMap::new(),
            withdrawn_users: Vec::new(),
        }
    }

    pub fn post_state_root(&mut self, commitment: StateCommitment) {
        println!(" L1: State root posted - Block {}", commitment.block_number);
        self.stored_roots.push(commitment);
    }

    pub fn emergency_withdraw(&mut self, proof: MerkleProof, user: Pubkey) -> Result<u64, String> {
        if self.withdrawn_users.contains(&user) {
            return Err("User already withdrew".to_string());
        }

        let latest_root = self.stored_roots.last()
            .ok_or("No state root available")?;

        if proof.root != latest_root.root {
            return Err("Proof root doesn't match stored root".to_string());
        }

        if !proof.verify() {
            return Err("Invalid merkle proof".to_string());
        }

        if proof.account_state.pubkey != user {
            return Err("Proof doesn't match user pubkey".to_string());
        }

        let withdraw_amount = proof.account_state.balance;
        self.withdrawn_users.push(user);

        println!(" L1: Emergency withdrawal successful - {} lamports to {}",
                withdraw_amount, user);

        Ok(withdraw_amount)
    }

    pub fn fund_contract(&mut self, user: Pubkey, amount: u64) {
        self.user_balances.insert(user, amount);
    }
}

pub struct EmergencyWithdrawalSimulation {
    pub l1_contract: MockL1Contract,
}

impl EmergencyWithdrawalSimulation {
    pub fn new() -> Self {
        Self {
            l1_contract: MockL1Contract::new(),
        }
    }

    pub fn simulate_user_emergency_exit(&mut self, proof: MerkleProof, user: Pubkey) {
        println!("\n EMERGENCY WITHDRAWAL SIMULATION");
        println!("User {} attempting emergency exit", user);

        match self.l1_contract.emergency_withdraw(proof, user) {
            Ok(amount) => {
                println!(" Emergency withdrawal successful!");
                println!(" User recovered {} lamports", amount);
            }
            Err(e) => {
                println!(" Emergency withdrawal failed: {}", e);
            }
        }
    }

    pub fn simulate_full_emergency_scenario(
        &mut self,
        users: Vec<(Pubkey, u64)>,
        state_commitments: Vec<StateCommitment>,
        proofs: Vec<MerkleProof>,
    ) {
        println!("\n FULL EMERGENCY SCENARIO SIMULATION");
        println!("========================================");

        for commitment in state_commitments {
            self.l1_contract.post_state_root(commitment);
        }

        println!("\n SodaSVM SEQUENCER CRASHED!");
        println!("Users must use emergency withdrawal...");

        for (i, (user, _)) in users.iter().enumerate() {
            if let Some(proof) = proofs.get(i) {
                self.simulate_user_emergency_exit(proof.clone(), *user);
            }
        }

        println!("\n Emergency Withdrawal Summary:");
        println!("Total users recovered: {}", self.l1_contract.withdrawn_users.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Sequencer, SodaAccountState};

    #[test]
    fn test_emergency_withdrawal_flow() {
        let mut simulation = EmergencyWithdrawalSimulation::new();
        let mut sequencer = Sequencer::new(2);

        let user1 = Pubkey::new_unique();
        let user2 = Pubkey::new_unique();

        sequencer.svm.register_user(user1, 1000);
        sequencer.svm.register_user(user2, 2000);

        sequencer.simulate_normal_operations(4);

        if let Some(commitment) = sequencer.get_latest_root() {
            simulation.l1_contract.post_state_root(commitment.clone());
        }

        sequencer.crash();

        let proof1 = sequencer.svm.generate_exit_proof(&user1).unwrap();
        let proof2 = sequencer.svm.generate_exit_proof(&user2).unwrap();

        let result1 = simulation.l1_contract.emergency_withdraw(proof1, user1);
        let result2 = simulation.l1_contract.emergency_withdraw(proof2, user2);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap(), 1000);
        assert_eq!(result2.unwrap(), 2000);
    }

    #[test]
    fn test_invalid_proof_rejection() {
        let mut simulation = EmergencyWithdrawalSimulation::new();
        let user = Pubkey::new_unique();

        let fake_proof = MerkleProof {
            account_index: 0,
            account_state: SodaAccountState::new(user, 1000, 0, 0),
            proof: vec![[0; 32]],
            root: [1; 32],
        };

        let result = simulation.l1_contract.emergency_withdraw(fake_proof, user);
        assert!(result.is_err());
    }
}