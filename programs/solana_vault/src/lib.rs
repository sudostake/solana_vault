use anchor_lang::prelude::*;

declare_id!("GuedoNc8MuqmSowhvL3MRYw21n5VGBWHo7PNoSnkaEPP");

const VAULT_SEED: &[u8] = b"vault";
const TREASURY_SEED: &[u8] = b"treasury";
const STAKE_AUTHORITY_SEED: &[u8] = b"stake-authority";
const WITHDRAW_AUTHORITY_SEED: &[u8] = b"withdraw-authority";
const POSITION_SEED: &[u8] = b"position";
const MANAGED_STAKE_SEED: &[u8] = b"stake";
const VAULT_VERSION: u8 = 1;

#[program]
pub mod solana_vault {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        owner: Pubkey,
        preferred_vote_account: Pubkey,
        min_idle_buffer_lamports: u64,
    ) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.version = VAULT_VERSION;
        vault.owner = owner;
        vault.preferred_vote_account = preferred_vote_account;
        vault.next_position_index = 0;
        vault.min_idle_buffer_lamports = min_idle_buffer_lamports;
        vault.vault_bump = ctx.bumps.vault;
        vault.treasury_bump = ctx.bumps.treasury;
        vault.stake_authority_bump = ctx.bumps.stake_authority;
        vault.withdraw_authority_bump = ctx.bumps.withdraw_authority;

        msg!("initialize_vault stub");
        Ok(())
    }

    pub fn deposit_sol(_ctx: Context<DepositSol>, lamports: u64) -> Result<()> {
        msg!("deposit_sol stub: {} lamports", lamports);
        Ok(())
    }

    pub fn withdraw_idle_sol(_ctx: Context<WithdrawIdleSol>, lamports: u64) -> Result<()> {
        msg!("withdraw_idle_sol stub: {} lamports", lamports);
        Ok(())
    }

    pub fn set_preferred_validator(
        _ctx: Context<SetPreferredValidator>,
        vote_account: Pubkey,
    ) -> Result<()> {
        msg!("set_preferred_validator stub: {}", vote_account);
        Ok(())
    }

    pub fn stake_idle_sol(
        _ctx: Context<StakeIdleSol>,
        expected_index: u64,
        lamports: u64,
    ) -> Result<()> {
        msg!(
            "stake_idle_sol stub: index={}, lamports={}",
            expected_index,
            lamports
        );
        Ok(())
    }

    pub fn deactivate_position(_ctx: Context<ManagePosition>, index: u64) -> Result<()> {
        msg!("deactivate_position stub: index={}", index);
        Ok(())
    }

    pub fn withdraw_inactive_position(
        _ctx: Context<WithdrawInactivePosition>,
        index: u64,
    ) -> Result<()> {
        msg!("withdraw_inactive_position stub: index={}", index);
        Ok(())
    }

    pub fn split_position(
        _ctx: Context<SplitPosition>,
        source_index: u64,
        expected_new_index: u64,
        lamports: u64,
    ) -> Result<()> {
        msg!(
            "split_position stub: source_index={}, new_index={}, lamports={}",
            source_index,
            expected_new_index,
            lamports
        );
        Ok(())
    }

    pub fn merge_positions(
        _ctx: Context<MergePositions>,
        dst_index: u64,
        src_index: u64,
    ) -> Result<()> {
        msg!(
            "merge_positions stub: dst_index={}, src_index={}",
            dst_index,
            src_index
        );
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + VaultState::INIT_SPACE,
        seeds = [VAULT_SEED],
        bump
    )]
    pub vault: Account<'info, VaultState>,
    #[account(
        init,
        payer = payer,
        space = 8 + Treasury::INIT_SPACE,
        seeds = [TREASURY_SEED],
        bump
    )]
    pub treasury: Account<'info, Treasury>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [STAKE_AUTHORITY_SEED], bump)]
    pub stake_authority: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [WITHDRAW_AUTHORITY_SEED], bump)]
    pub withdraw_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositSol<'info> {
    #[account(mut, seeds = [VAULT_SEED], bump = vault.vault_bump, has_one = owner)]
    pub vault: Account<'info, VaultState>,
    pub owner: Signer<'info>,
    #[account(mut, seeds = [TREASURY_SEED], bump = vault.treasury_bump)]
    pub treasury: Account<'info, Treasury>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawIdleSol<'info> {
    #[account(mut, seeds = [VAULT_SEED], bump = vault.vault_bump, has_one = owner)]
    pub vault: Account<'info, VaultState>,
    pub owner: Signer<'info>,
    #[account(mut, seeds = [TREASURY_SEED], bump = vault.treasury_bump)]
    pub treasury: Account<'info, Treasury>,
    /// CHECK: Recipient is an arbitrary system account for future SOL transfers.
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetPreferredValidator<'info> {
    #[account(mut, seeds = [VAULT_SEED], bump = vault.vault_bump, has_one = owner)]
    pub vault: Account<'info, VaultState>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(expected_index: u64)]
pub struct StakeIdleSol<'info> {
    #[account(mut, seeds = [VAULT_SEED], bump = vault.vault_bump, has_one = owner)]
    pub vault: Account<'info, VaultState>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [TREASURY_SEED], bump = vault.treasury_bump)]
    pub treasury: Account<'info, Treasury>,
    #[account(
        init,
        payer = owner,
        space = 8 + StakePosition::INIT_SPACE,
        seeds = [POSITION_SEED, &expected_index.to_le_bytes()],
        bump
    )]
    pub position: Account<'info, StakePosition>,
    /// CHECK: PDA address reserved for the future stake account managed by this position.
    #[account(mut, seeds = [MANAGED_STAKE_SEED, &expected_index.to_le_bytes()], bump)]
    pub managed_stake: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [STAKE_AUTHORITY_SEED], bump = vault.stake_authority_bump)]
    pub stake_authority: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [WITHDRAW_AUTHORITY_SEED], bump = vault.withdraw_authority_bump)]
    pub withdraw_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(index: u64)]
pub struct ManagePosition<'info> {
    #[account(seeds = [VAULT_SEED], bump = vault.vault_bump, has_one = owner)]
    pub vault: Account<'info, VaultState>,
    pub owner: Signer<'info>,
    #[account(mut, seeds = [POSITION_SEED, &index.to_le_bytes()], bump = position.bump)]
    pub position: Account<'info, StakePosition>,
    /// CHECK: Future managed stake account for this position.
    #[account(mut, address = position.stake_account)]
    pub managed_stake: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [STAKE_AUTHORITY_SEED], bump = vault.stake_authority_bump)]
    pub stake_authority: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [WITHDRAW_AUTHORITY_SEED], bump = vault.withdraw_authority_bump)]
    pub withdraw_authority: UncheckedAccount<'info>,
    /// CHECK: Stake program account reserved for future CPI.
    pub stake_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(index: u64)]
pub struct WithdrawInactivePosition<'info> {
    #[account(seeds = [VAULT_SEED], bump = vault.vault_bump, has_one = owner)]
    pub vault: Account<'info, VaultState>,
    pub owner: Signer<'info>,
    #[account(mut, seeds = [TREASURY_SEED], bump = vault.treasury_bump)]
    pub treasury: Account<'info, Treasury>,
    #[account(mut, seeds = [POSITION_SEED, &index.to_le_bytes()], bump = position.bump)]
    pub position: Account<'info, StakePosition>,
    /// CHECK: Future managed stake account for this position.
    #[account(mut, address = position.stake_account)]
    pub managed_stake: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [WITHDRAW_AUTHORITY_SEED], bump = vault.withdraw_authority_bump)]
    pub withdraw_authority: UncheckedAccount<'info>,
    /// CHECK: Stake program account reserved for future CPI.
    pub stake_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(source_index: u64, expected_new_index: u64)]
pub struct SplitPosition<'info> {
    #[account(seeds = [VAULT_SEED], bump = vault.vault_bump, has_one = owner)]
    pub vault: Account<'info, VaultState>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [POSITION_SEED, &source_index.to_le_bytes()], bump = source_position.bump)]
    pub source_position: Account<'info, StakePosition>,
    /// CHECK: Existing stake account tied to the source position.
    #[account(mut, address = source_position.stake_account)]
    pub source_stake: UncheckedAccount<'info>,
    #[account(
        init,
        payer = owner,
        space = 8 + StakePosition::INIT_SPACE,
        seeds = [POSITION_SEED, &expected_new_index.to_le_bytes()],
        bump
    )]
    pub new_position: Account<'info, StakePosition>,
    /// CHECK: PDA address reserved for the future split stake account.
    #[account(mut, seeds = [MANAGED_STAKE_SEED, &expected_new_index.to_le_bytes()], bump)]
    pub new_stake: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [STAKE_AUTHORITY_SEED], bump = vault.stake_authority_bump)]
    pub stake_authority: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [WITHDRAW_AUTHORITY_SEED], bump = vault.withdraw_authority_bump)]
    pub withdraw_authority: UncheckedAccount<'info>,
    /// CHECK: Stake program account reserved for future CPI.
    pub stake_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(dst_index: u64, src_index: u64)]
pub struct MergePositions<'info> {
    #[account(seeds = [VAULT_SEED], bump = vault.vault_bump, has_one = owner)]
    pub vault: Account<'info, VaultState>,
    pub owner: Signer<'info>,
    #[account(mut, seeds = [POSITION_SEED, &dst_index.to_le_bytes()], bump = dst_position.bump)]
    pub dst_position: Account<'info, StakePosition>,
    /// CHECK: Existing stake account tied to the destination position.
    #[account(mut, address = dst_position.stake_account)]
    pub dst_stake: UncheckedAccount<'info>,
    #[account(mut, seeds = [POSITION_SEED, &src_index.to_le_bytes()], bump = src_position.bump)]
    pub src_position: Account<'info, StakePosition>,
    /// CHECK: Existing stake account tied to the source position.
    #[account(mut, address = src_position.stake_account)]
    pub src_stake: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [STAKE_AUTHORITY_SEED], bump = vault.stake_authority_bump)]
    pub stake_authority: UncheckedAccount<'info>,
    /// CHECK: PDA signer for future stake-program CPI.
    #[account(seeds = [WITHDRAW_AUTHORITY_SEED], bump = vault.withdraw_authority_bump)]
    pub withdraw_authority: UncheckedAccount<'info>,
    /// CHECK: Stake program account reserved for future CPI.
    pub stake_program: UncheckedAccount<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub version: u8,
    pub owner: Pubkey,
    pub preferred_vote_account: Pubkey,
    pub next_position_index: u64,
    pub min_idle_buffer_lamports: u64,
    pub vault_bump: u8,
    pub treasury_bump: u8,
    pub stake_authority_bump: u8,
    pub withdraw_authority_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Treasury {}

#[account]
#[derive(InitSpace)]
pub struct StakePosition {
    pub index: u64,
    pub stake_account: Pubkey,
    pub last_delegated_vote_account: Pubkey,
    pub is_closed: bool,
    pub bump: u8,
}
