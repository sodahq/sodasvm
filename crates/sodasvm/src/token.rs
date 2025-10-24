use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

pub struct UsdcToken {
    pub mint: Pubkey,
    pub mint_authority: Pubkey,
    pub decimals: u8,
    pub vault_account: Pubkey,
    pub vault_authority: Pubkey,
    pub user_accounts: HashMap<Pubkey, Pubkey>, // user -> token_account
}

impl UsdcToken {
    pub fn new() -> Self {
        let mint_authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let vault_account = Pubkey::new_unique();
        let vault_authority = Pubkey::new_unique();

        Self {
            mint,
            mint_authority,
            decimals: 6, // Standard USDC decimals
            vault_account,
            vault_authority,
            user_accounts: HashMap::new(),
        }
    }

    pub fn create_user_account(&mut self, user: &Pubkey) -> Pubkey {
        let token_account = Pubkey::new_unique();
        self.user_accounts.insert(*user, token_account);
        token_account
    }

    pub fn get_user_account(&self, user: &Pubkey) -> Option<&Pubkey> {
        self.user_accounts.get(user)
    }

    pub fn deposit_to_l2(&self, user: &Pubkey, amount: u64) -> Result<(), String> {
        let user_token_account = self.get_user_account(user)
            .ok_or("User token account not found")?;

        println!("Depositing {} USDC from user {} to L2",
                 amount as f64 / 10_u64.pow(self.decimals as u32) as f64,
                 user);

        println!("Transfer: {} -> Vault {}", user_token_account, self.vault_account);

        Ok(())
    }

    pub fn emergency_withdraw(&self, user: &Pubkey, amount: u64) -> Result<(), String> {
        let user_token_account = self.get_user_account(user)
            .ok_or("User token account not found")?;

        println!("Emergency withdrawing {} USDC from vault to user {}",
                 amount as f64 / 10_u64.pow(self.decimals as u32) as f64,
                 user);

        println!("Transfer: Vault {} -> {}", self.vault_account, user_token_account);

        Ok(())
    }

    pub fn format_amount(&self, amount: u64) -> f64 {
        amount as f64 / 10_u64.pow(self.decimals as u32) as f64
    }
}