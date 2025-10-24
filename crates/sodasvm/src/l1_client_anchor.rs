use solana_client::rpc_client::RpcClient;
use serde_json;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use crate::MerkleProof;
use borsh::{BorshSerialize, BorshDeserialize};
use std::str::FromStr;

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

pub struct L1ClientAnchor {
    pub rpc_client: RpcClient,
    pub program_id: Pubkey,
    pub authority: Keypair,
}

impl L1ClientAnchor {
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

    fn get_anchor_discriminator(namespace: &str, name: &str) -> [u8; 8] {
        let preimage = format!("{}:{}", namespace, name);
        let mut hasher = solana_sdk::hash::Hasher::default();
        hasher.hash(preimage.as_bytes());
        let hash = hasher.result();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&hash.to_bytes()[..8]);
        discriminator
    }

    pub async fn initialize_contract(
        &self,
        contract_account: &Keypair,
        usdc_mint: &Pubkey,
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let discriminator = Self::get_anchor_discriminator("global", "initialize_contract");
        let mut instruction_data = discriminator.to_vec();
        instruction_data.extend_from_slice(&usdc_mint.to_bytes());

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(contract_account.pubkey(), true),
                AccountMeta::new(self.authority.pubkey(), true),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
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

    pub async fn deposit_usdc(
        &self,
        user_keypair: &Keypair,
        user_usdc_account: &Pubkey,
        vault_usdc_account: &Pubkey,
        contract_account: &Pubkey,
        amount: u64,
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let discriminator = Self::get_anchor_discriminator("global", "deposit_usdc");
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
        println!("L1: USDC deposit - {} tokens - {}", amount, signature);
        Ok(signature.to_string())
    }

    pub async fn post_state_root_to_l1(
        &self,
        state_account: &Keypair,
        contract_account: &Pubkey,
        merkle_root: [u8; 32],
        block_number: u64,
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let discriminator = Self::get_anchor_discriminator("global", "post_state_root");
        let mut instruction_data = discriminator.to_vec();
        instruction_data.extend_from_slice(&merkle_root);
        instruction_data.extend_from_slice(&block_number.to_le_bytes());

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(state_account.pubkey(), true),
                AccountMeta::new(*contract_account, false),
                AccountMeta::new(self.authority.pubkey(), true),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
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
        println!("L1: State root posted - Block {} - {}", block_number, signature);
        Ok(signature.to_string())
    }

    pub async fn emergency_withdraw(
        &self,
        state_account: &Pubkey,
        contract_account: &Pubkey,
        user_keypair: &Keypair,
        user_usdc_account: &Pubkey,
        vault_usdc_account: &Pubkey,
        proof: MerkleProof,
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let l1_proof = L1MerkleProof::from(proof);
        let discriminator = Self::get_anchor_discriminator("global", "emergency_withdraw");
        let proof_data = borsh::to_vec(&l1_proof)?;

        let mut instruction_data = discriminator.to_vec();
        instruction_data.extend_from_slice(&proof_data);

        let (withdrawal_record_pda, _) = Pubkey::find_program_address(
            &[b"withdrawal", user_keypair.pubkey().as_ref()],
            &self.program_id,
        );

        let (vault_authority, _) = Pubkey::find_program_address(
            &[b"vault_authority"],
            &self.program_id,
        );

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
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
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
        println!("L1: Emergency withdrawal - {}", signature);
        Ok(signature.to_string())
    }

    pub async fn advance_time_slots(&self, slots: u64) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Get current slot
        let current_slot = self.rpc_client.get_slot()?;
        let target_slot = current_slot + slots;

        println!("L1: Advancing time from slot {} to slot {} (+{} slots)", current_slot, target_slot, slots);

        // Use Solana test validator's warp capability
        // This works only on localnet/test validator
        let params = serde_json::json!([target_slot]);
        let _result: serde_json::Value = self.rpc_client.send(
            solana_client::rpc_request::RpcRequest::Custom { method: "warpToSlot" },
            params,
        )?;

        println!("L1: Time advanced successfully to slot {}", target_slot);
        Ok(())
    }

    pub fn check_program_deployed(&self) -> std::result::Result<bool, Box<dyn std::error::Error>> {
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