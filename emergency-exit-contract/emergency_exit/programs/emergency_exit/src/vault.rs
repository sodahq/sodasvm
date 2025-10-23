use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

declare_id!("BbYmwfjNKjLVKTV6qbWBj41toNPj9wY4Zicx8q63AMjr");

#[program]
pub mod emergency_exit {
    use super::*;

    pub fn initialize_contract(ctx: Context<InitializeContract>, usdc_mint: Pubkey) -> Result<()> {
        let contract = &mut ctx.accounts.contract;
        contract.authority = ctx.accounts.authority.key();
        contract.vault_balance = 0;
        contract.total_withdrawals = 0;
        contract.is_active = true;
        contract.usdc_mint = usdc_mint;
        contract.time_lock_duration_slots = 151200; // ~7 days at 400ms/slot
        contract.last_state_root_slot = 0;

        msg!("Emergency Exit Contract initialized with USDC mint: {}", usdc_mint);
        Ok(())
    }

    pub fn deposit_usdc(
        ctx: Context<DepositUsdc>,
        amount: u64,
    ) -> Result<()> {
        let contract = &mut ctx.accounts.contract;

        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_usdc_account.to_account_info(),
                to: ctx.accounts.vault_usdc_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );

        token::transfer(transfer_ctx, amount)?;
        contract.vault_balance += amount;

        msg!("USDC deposited: {} tokens, Vault balance: {}", amount, contract.vault_balance);
        Ok(())
    }

    pub fn post_state_root(
        ctx: Context<PostStateRoot>,
        merkle_root: [u8; 32],
        block_number: u64,
    ) -> Result<()> {
        let state_account = &mut ctx.accounts.state_account;
        let contract = &mut ctx.accounts.contract;
        let clock = Clock::get()?;

        state_account.merkle_root = merkle_root;
        state_account.block_number = block_number;
        state_account.timestamp = clock.unix_timestamp;
        state_account.is_finalized = true;

        contract.last_state_root_slot = clock.slot;

        msg!("State root posted - Block: {}, Slot: {}", block_number, clock.slot);
        Ok(())
    }

    pub fn emergency_withdraw(
        ctx: Context<EmergencyWithdraw>,
        proof: MerkleProofData,
    ) -> Result<()> {
        let state_account = &ctx.accounts.state_account;
        let user = &ctx.accounts.user;
        let withdrawal_record = &mut ctx.accounts.withdrawal_record;
        let contract = &ctx.accounts.contract;
        let clock = Clock::get()?;

        require!(proof.account_pubkey == user.key(), ErrorCode::InvalidUser);
        require!(!withdrawal_record.withdrawn, ErrorCode::AlreadyWithdrawn);

        // Time lock check: Emergency withdrawals only after time lock period
        let slots_since_last_root = clock.slot.saturating_sub(contract.last_state_root_slot);
        require!(
            slots_since_last_root >= contract.time_lock_duration_slots,
            ErrorCode::TimeLockNotExpired
        );

        require!(
            verify_merkle_proof(&proof, &state_account.merkle_root),
            ErrorCode::InvalidProof
        );

        withdrawal_record.user = user.key();
        withdrawal_record.amount = proof.balance;
        withdrawal_record.timestamp = clock.unix_timestamp;
        withdrawal_record.withdrawn = true;
        withdrawal_record.slot_withdrawn = clock.slot;

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_usdc_account.to_account_info(),
                to: ctx.accounts.user_usdc_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            &[&[b"vault_authority", &[ctx.bumps.vault_authority]]],
        );

        token::transfer(transfer_ctx, proof.balance)?;

        msg!("Emergency withdrawal successful: {} USDC tokens at slot {}", proof.balance, clock.slot);
        Ok(())
    }

    pub fn simulate_time_passage(
        ctx: Context<SimulateTime>,
        slots_to_advance: u64,
    ) -> Result<()> {
        let contract = &mut ctx.accounts.contract;
        contract.last_state_root_slot = contract.last_state_root_slot.saturating_sub(slots_to_advance);

        msg!("Simulated {} slots passage for testing", slots_to_advance);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeContract<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 8 + 8 + 1 + 32 + 8 + 8
    )]
    pub contract: Account<'info, EmergencyContract>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositUsdc<'info> {
    #[account(mut)]
    pub contract: Account<'info, EmergencyContract>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        constraint = user_usdc_account.mint == contract.usdc_mint
    )]
    pub user_usdc_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_usdc_account.mint == contract.usdc_mint
    )]
    pub vault_usdc_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
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
    pub contract: Account<'info, EmergencyContract>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct EmergencyWithdraw<'info> {
    pub state_account: Account<'info, StateCommitment>,
    pub contract: Account<'info, EmergencyContract>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        constraint = user_usdc_account.mint == contract.usdc_mint
    )]
    pub user_usdc_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_usdc_account.mint == contract.usdc_mint
    )]
    pub vault_usdc_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"vault_authority"],
        bump
    )]
    /// CHECK: PDA for vault authority
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8 + 8 + 1 + 8,
        seeds = [b"withdrawal", user.key().as_ref()],
        bump
    )]
    pub withdrawal_record: Account<'info, WithdrawalRecord>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SimulateTime<'info> {
    #[account(
        mut,
        constraint = contract.authority == authority.key()
    )]
    pub contract: Account<'info, EmergencyContract>,
    pub authority: Signer<'info>,
}

#[account]
pub struct EmergencyContract {
    pub authority: Pubkey,
    pub vault_balance: u64,
    pub total_withdrawals: u64,
    pub is_active: bool,
    pub usdc_mint: Pubkey,
    pub time_lock_duration_slots: u64,
    pub last_state_root_slot: u64,
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
    pub slot_withdrawn: u64,
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
    #[msg("Time lock not expired - emergency withdrawals not yet available")]
    TimeLockNotExpired,
}