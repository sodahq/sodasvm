use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use std::str::FromStr;
use crate::MerkleProof;
use borsh::{BorshSerialize, BorshDeserialize, to_vec};
use solana_sha256_hasher::Hasher;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum EmergencyInstruction {
    InitializeContract,
    PostStateRoot {
        merkle_root: [u8; 32],
        block_number: u64,
    },
    EmergencyWithdraw {
        proof: L1MerkleProof,
    },
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PostStateRootArgs {
    pub merkle_root: [u8; 32],
    pub block_number: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct EmergencyWithdrawArgs {
    pub proof: L1MerkleProof,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct L1MerkleProof {
    pub account_index: u32,
    pub account_pubkey: Pubkey,
    pub balance: u64,
    pub nonce: u64,
    pub last_update: i64,
    pub proof: Vec<[u8; 32]>,
}

impl From<MerkleProof> for L1MerkleProof {
    fn from(proof: MerkleProof) -> Self {
        Self {
            account_index: proof.account_index,
            account_pubkey: Pubkey::from_str(&proof.account_state.pubkey.to_string()).unwrap(),
            balance: proof.account_state.balance,
            nonce: proof.account_state.nonce,
            last_update: proof.account_state.last_update,
            proof: proof.proof,
        }
    }
}

pub struct L1Client {
    pub rpc_client: RpcClient,
    pub program_id: Pubkey,
    pub authority: Keypair,
}

fn get_discriminator(instruction: &str) -> [u8; 8] {
    match instruction {
        "emergency_withdraw" => [239, 45, 203, 64, 150, 73, 218, 92],
        "post_state_root" => [219, 218, 56, 232, 23, 15, 104, 16],
        "initialize_contract" => [181, 192, 35, 141, 212, 113, 138, 94],
        "deposit_usdc" => [242, 35, 198, 137, 82, 225, 242, 182],
        _ => panic!("Unknown instruction: {}", instruction),
    }
}

// spl token (busdc ) => deposits => 
// tree analysis => depth => user id =>  shard the tree => 
// simulate binary tree 
// time lock in the contract 
// use slot time ot simualte => 7days => 
// 


impl L1Client {
    pub fn find_withdrawal_record_pda(
        &self,
        user_pubkey: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                b"withdrawal",
                user_pubkey.as_ref(),
            ],
            &self.program_id,
        )
    }

    pub fn find_vault_pda(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"emergency_vault"],
            &self.program_id,
        )
    }

    pub fn new(rpc_url: &str, program_id: Pubkey, authority: Keypair) -> Self {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );

        Self {
            rpc_client,
            program_id,
            authority,
        }
    }

    pub fn initialize_contract(&self, contract_account: &Keypair, usdc_mint: &Pubkey) -> Result<String, Box<dyn std::error::Error>> {
        let discriminator = get_discriminator("initialize_contract");
        let mut instruction_data = discriminator.to_vec();
        instruction_data.extend_from_slice(&usdc_mint.to_bytes());

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(contract_account.pubkey(), true),
                AccountMeta::new(self.authority.pubkey(), true),
                AccountMeta::new_readonly(Pubkey::from_str("11111111111111111111111111111111").unwrap(), false),
            ],
            data: instruction_data,
        };

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.authority.pubkey()),
            &[&self.authority, contract_account],
            recent_blockhash,
        );

        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        println!("L1: Contract initialized - {}", signature);
        Ok(signature.to_string())
    }

    pub fn deposit_usdc(
        &self,
        user_keypair: &Keypair,
        user_usdc_account: &Pubkey,
        vault_usdc_account: &Pubkey,
        contract_account: &Pubkey,
        amount: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let discriminator = get_discriminator("deposit_usdc");
        let mut instruction_data = discriminator.to_vec();
        instruction_data.extend_from_slice(&amount.to_le_bytes());

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*contract_account, false),
                AccountMeta::new(user_keypair.pubkey(), true),
                AccountMeta::new(*user_usdc_account, false),
                AccountMeta::new(*vault_usdc_account, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data: instruction_data,
        };

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&user_keypair.pubkey()),
            &[user_keypair],
            recent_blockhash,
        );

        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        println!("L1: USDC deposit completed - {} tokens - {}", amount, signature);
        Ok(signature.to_string())
    }

    pub fn post_state_root_to_l1(
        &self,
        state_account: &Keypair,
        contract_account: &Pubkey,
        merkle_root: [u8; 32],
        block_number: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let discriminator = get_discriminator("post_state_root");
        let instruction_args = to_vec(&PostStateRootArgs {
            merkle_root,
            block_number,
        })?;

        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&instruction_args);

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(state_account.pubkey(), true),
                AccountMeta::new(*contract_account, false),
                AccountMeta::new(self.authority.pubkey(), true),
                AccountMeta::new_readonly(Pubkey::from_str("11111111111111111111111111111111").unwrap(), false),
            ],
            data: instruction_data,
        };

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.authority.pubkey()),
            &[&self.authority, state_account],
            recent_blockhash,
        );

        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        println!("L1: State root posted on-chain - Block {} - {}", block_number, signature);
        Ok(signature.to_string())
    }
    
    pub fn emergency_withdraw(
        &self,
        state_account: &Pubkey,
        contract_account: &Pubkey,
        user_keypair: &Keypair,
        user_usdc_account: &Pubkey,
        vault_usdc_account: &Pubkey,
        proof: MerkleProof,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let l1_proof = L1MerkleProof::from(proof);

        let discriminator = get_discriminator("emergency_withdraw");
        let instruction_args = to_vec(&EmergencyWithdrawArgs {
            proof: l1_proof,
        })?;

        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&instruction_args);

        let (withdrawal_record_pda, _bump) = self.find_withdrawal_record_pda(
            &user_keypair.pubkey(),
        );

        let (vault_authority, _) = Pubkey::find_program_address(&[b"vault_authority"], &self.program_id);

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(*state_account, false),
                AccountMeta::new_readonly(*contract_account, false),
                AccountMeta::new(user_keypair.pubkey(), true),
                AccountMeta::new(*user_usdc_account, false),
                AccountMeta::new(*vault_usdc_account, false),
                AccountMeta::new_readonly(vault_authority, false),
                AccountMeta::new(withdrawal_record_pda, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(Pubkey::from_str("11111111111111111111111111111111").unwrap(), false),
            ],
            data: instruction_data,
        };

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&user_keypair.pubkey()),
            &[user_keypair],
            recent_blockhash,
        );

        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        println!("L1: Emergency withdrawal executed on-chain - {}", signature);
        Ok(signature.to_string())
    }

    pub fn airdrop(&self, pubkey: &Pubkey, lamports: u64) -> Result<String, Box<dyn std::error::Error>> {
        let signature = self.rpc_client.request_airdrop(pubkey, lamports)?;
        self.rpc_client.confirm_transaction(&signature)?;
        println!("L1: Airdrop completed - {} lamports to {}", lamports, pubkey);
        Ok(signature.to_string())
    }

    pub fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self.rpc_client.get_balance(pubkey)?)
    }

    pub fn check_program_deployed(&self) -> Result<bool, Box<dyn std::error::Error>> {
        match self.rpc_client.get_account(&self.program_id) {
            Ok(account) if account.data.len() > 0 => {
                println!("L1: Emergency program verified deployed at {}", self.program_id);
                Ok(true)
            }
            Ok(_) => {
                println!("L1: Program not found at {}", self.program_id);
                Ok(false)
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l1_client_creation() {
        let authority = Keypair::new();
        let program_id = Pubkey::new_unique();

        let client = L1Client::new("http://127.0.0.1:8899", program_id, authority);
        assert_eq!(client.program_id, program_id);
    }
}