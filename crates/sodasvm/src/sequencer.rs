use crate::{SodaSVM, SodaMerkleTree};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Sequencer {
    pub svm: SodaSVM,
    pub commit_interval: u64,
    pub transaction_count: u64,
    pub state_roots: Vec<StateCommitment>,
    pub is_online: bool,
}

#[derive(Debug, Clone)]
pub struct StateCommitment {
    pub root: [u8; 32],
    pub block_number: u64,
    pub timestamp: i64,
    pub transaction_count: u64,
}

impl Sequencer {
    pub fn new(commit_interval: u64) -> Self {
        Self {
            svm: SodaSVM::new(),
            commit_interval,
            transaction_count: 0,
            state_roots: Vec::new(),
            is_online: true,
        }
    }

    pub fn process_transaction(&mut self) -> Result<(), String> {
        if !self.is_online {
            return Err("Sequencer is offline".to_string());
        }

        self.transaction_count += 1;

        if self.transaction_count % self.commit_interval == 0 {
            self.commit_state_root();
        }

        Ok(())
    }

    pub fn commit_state_root(&mut self) {
        let root = self.svm.commit_state_root();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let commitment = StateCommitment {
            root,
            block_number: self.state_roots.len() as u64,
            timestamp,
            transaction_count: self.transaction_count,
        };

        self.state_roots.push(commitment);
        println!("State root committed: {}", hex::encode(root));
    }

    pub fn crash(&mut self) {
        self.is_online = false;
    }

    pub fn get_latest_root(&self) -> Option<&StateCommitment> {
        self.state_roots.last()
    }

    pub fn simulate_normal_operations(&mut self, num_transactions: u64) {
        for i in 1..=num_transactions {
            match self.process_transaction() {
                Ok(_) => {
                    if i % 10 == 0 {
                        println!("📦 Processed {} transactions", i);
                    }
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_pubkey::Pubkey;

    #[test]
    fn test_sequencer_commits() {
        let mut sequencer = Sequencer::new(5);

        sequencer.svm.register_user(Pubkey::new_unique(), 1000);
        sequencer.svm.register_user(Pubkey::new_unique(), 2000);

        sequencer.simulate_normal_operations(10);

        assert_eq!(sequencer.state_roots.len(), 2);
        assert_eq!(sequencer.transaction_count, 10);
    }

    #[test]
    fn test_sequencer_crash() {
        let mut sequencer = Sequencer::new(3);

        sequencer.simulate_normal_operations(5);
        sequencer.crash();

        assert!(!sequencer.is_online);
        assert!(sequencer.process_transaction().is_err());
    }
}