use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("GuedoNc8MuqmSowhvL3MRYw21n5VGBWHo7PNoSnkaEPP");

const VAULT_SEED: &[u8] = b"vault";

#[program]
pub mod solana_vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.payer.key();
        vault.vault_bump = ctx.bumps.vault;
        Ok(())
    }

    /// Stub: full deposit will CPI `transfer_checked` using `authority` and `remaining_accounts` (multisig).
    pub fn lender_deposit(_ctx: Context<LenderDeposit>, _amount: u64) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(init, payer = payer, space = 8 + VaultState::INIT_SPACE, seeds = [VAULT_SEED], bump)]
    pub vault: Account<'info, VaultState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LenderDeposit<'info> {
    #[account(seeds = [VAULT_SEED], bump = vault.vault_bump)]
    pub vault: Account<'info, VaultState>,
    #[account(mut)]
    pub from: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = to.owner == vault.key() @ VaultError::InvalidVaultTokenAccount,
        constraint = to.mint == mint.key() @ VaultError::MintMismatch,
    )]
    pub to: Account<'info, TokenAccount>,
    #[account(constraint = from.mint == mint.key() @ VaultError::MintMismatch)]
    pub mint: Account<'info, Mint>,
    /// CHECK: must equal `from.owner` (not `Signer` — wallet / multisig / PDA).
    #[account(constraint = authority.key() == from.owner @ VaultError::InvalidTokenAuthority)]
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub owner: Pubkey,
    pub vault_bump: u8,
}

#[error_code]
pub enum VaultError {
    #[msg("Destination token account must be owned by the vault PDA")]
    InvalidVaultTokenAccount,
    #[msg("Mint mismatch between token accounts and mint")]
    MintMismatch,
    #[msg("Authority must match the source token account owner")]
    InvalidTokenAuthority,
}
