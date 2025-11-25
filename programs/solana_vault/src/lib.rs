use anchor_lang::prelude::*;

declare_id!("GuedoNc8MuqmSowhvL3MRYw21n5VGBWHo7PNoSnkaEPP");

#[program]
pub mod solana_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
