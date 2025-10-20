use anchor_lang::prelude::*;

declare_id!("BbYmwfjNKjLVKTV6qbWBj41toNPj9wY4Zicx8q63AMjr");

#[program]
pub mod emergency_exit {
    use super::*;

    pub fn initialize_contract(ctx: Context<InitializeContract>) -> Result<()> {
        let contract = &mut ctx.accounts.contract;
        contract.authority = ctx.accounts.authority.key();
        contract.vault_balance = 0;
        contract.total_withdrawals = 0;
        contract.is_active = true;

        msg!("Emergency Exit Contract initialized");
        Ok(())
    }

    pub fn post_state_root(
        ctx: Context<PostStateRoot>,
        merkle_root: [u8; 32],
        block_number: u64,
    ) -> Result<()> {
        let state_account = &mut ctx.accounts.state_account;
        let clock = Clock::get()?;

        state_account.merkle_root = merkle_root;
        state_account.block_number = block_number;
        state_account.timestamp = clock.unix_timestamp;
        state_account.is_finalized = true;

        msg!("State root posted - Block: {}", block_number);
        Ok(())
    }

    pub fn emergency_withdraw(
        ctx: Context<EmergencyWithdraw>,
        proof: MerkleProofData,
    ) -> Result<()> {
        let state_account = &ctx.accounts.state_account;
        let user = &ctx.accounts.user;
        let withdrawal_record = &mut ctx.accounts.withdrawal_record;

        require!(proof.account_pubkey == user.key(), ErrorCode::InvalidUser);
        require!(!withdrawal_record.withdrawn, ErrorCode::AlreadyWithdrawn);

        require!(
            verify_merkle_proof(&proof, &state_account.merkle_root),
            ErrorCode::InvalidProof
        );

        withdrawal_record.user = user.key();
        withdrawal_record.amount = proof.balance;
        withdrawal_record.timestamp = Clock::get()?.unix_timestamp;
        withdrawal_record.withdrawn = true;

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= proof.balance;
        **user.to_account_info().try_borrow_mut_lamports()? += proof.balance;

        msg!("Emergency withdrawal successful: {} lamports", proof.balance);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeContract<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 8 + 8 + 1
    )]
    pub contract: Account<'info, EmergencyContract>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PostStateRoot<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 8 + 8 + 1
    )]
    pub state_account: Account<'info, StateCommitment>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct EmergencyWithdraw<'info> {
    pub state_account: Account<'info, StateCommitment>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: Vault account that holds emergency funds
    #[account(mut)]
    pub vault: UncheckedAccount<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8 + 8 + 1,
        seeds = [b"withdrawal", user.key().as_ref()],
        bump
    )]
    pub withdrawal_record: Account<'info, WithdrawalRecord>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct EmergencyContract {
    pub authority: Pubkey,
    pub vault_balance: u64,
    pub total_withdrawals: u64,
    pub is_active: bool,
}

#[account]
pub struct StateCommitment {
    pub merkle_root: [u8; 32],
    pub block_number: u64,
    pub timestamp: i64,
    pub is_finalized: bool,
}

#[account]
pub struct WithdrawalRecord {
    pub user: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
    pub withdrawn: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MerkleProofData {
    pub account_index: u32,
    pub account_pubkey: Pubkey,
    pub balance: u64,
    pub nonce: u64,
    pub last_update: i64,
    pub proof: Vec<[u8; 32]>,
}

fn verify_merkle_proof(proof: &MerkleProofData, expected_root: &[u8; 32]) -> bool {
    use sha3::{Digest, Keccak256};

    let leaf_hash = {
        let mut hasher = Keccak256::new();
        hasher.update(&[0x00]);
        hasher.update(proof.account_pubkey.to_bytes());
        hasher.update(proof.balance.to_le_bytes());
        hasher.update(proof.nonce.to_le_bytes());
        hasher.update(proof.last_update.to_le_bytes());
        let result: [u8; 32] = hasher.finalize().into();
        result
    };

    let mut current_hash = leaf_hash;
    let mut current_index = proof.account_index;

    for sibling_hash in &proof.proof {
        let mut hasher = Keccak256::new();
        hasher.update(&[0x01]);

        if current_index % 2 == 0 {
            hasher.update(&current_hash);
            hasher.update(sibling_hash);
        } else {
            hasher.update(sibling_hash);
            hasher.update(&current_hash);
        }

        current_hash = hasher.finalize().into();
        current_index >>= 1;
    }

    &current_hash == expected_root
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid user")]
    InvalidUser,
    #[msg("User already withdrew")]
    AlreadyWithdrawn,
    #[msg("Invalid merkle proof")]
    InvalidProof,
}