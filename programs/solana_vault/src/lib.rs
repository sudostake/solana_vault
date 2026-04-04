use anchor_lang::prelude::*;

declare_id!("GuedoNc8MuqmSowhvL3MRYw21n5VGBWHo7PNoSnkaEPP");

#[program]
pub mod solana_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Hello from my vault: {:?}", ctx.program_id);
        let account_data = &mut ctx.accounts.account_data;
        account_data.owner = *ctx.accounts.user.key;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 32)]
    pub account_data: Account<'info, AccountData>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct AccountData {
    pub owner: Pubkey,
}

// TransferHookAccount
