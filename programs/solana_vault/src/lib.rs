use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer as SystemTransfer};
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

declare_id!("GuedoNc8MuqmSowhvL3MRYw21n5VGBWHo7PNoSnkaEPP");

const VAULT_SEED: &[u8] = b"vault";
const TREASURY_SEED: &[u8] = b"treasury";
const COUNTER_OFFER_SEED: &[u8] = b"counter_offer";

const STATUS_IDLE: u8 = 0;
const STATUS_REQUESTED: u8 = 1;
const STATUS_FUNDED: u8 = 2;
const STATUS_REPAID: u8 = 3;
const STATUS_LIQUIDATED: u8 = 4;
const STATUS_COUNTERED: u8 = 5;

#[program]
pub mod solana_vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.request_id = 0;
        vault.vault_bump = ctx.bumps.vault;
        vault.treasury_bump = ctx.bumps.treasury;
        vault.reset_loan(STATUS_IDLE);

        let treasury = &mut ctx.accounts.treasury;
        treasury.bump = ctx.bumps.treasury;

        Ok(())
    }

    pub fn open_loan_request(
        ctx: Context<OpenLoanRequest>,
        amount: u64,
        interest: u64,
        collateral: u64,
        duration_seconds: i64,
    ) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        require!(collateral > 0, VaultError::InvalidCollateral);
        require!(duration_seconds > 0, VaultError::InvalidDuration);

        let vault = &mut ctx.accounts.vault;
        require!(!vault.has_pending_loan(), VaultError::LoanAlreadyInProgress);
        vault.request_id = vault
            .request_id
            .checked_add(1)
            .ok_or(VaultError::ArithmeticOverflow)?;

        let transfer_accounts = SystemTransfer {
            from: ctx.accounts.owner.to_account_info(),
            to: ctx.accounts.treasury.to_account_info(),
        };
        let transfer_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
        );
        system_program::transfer(transfer_ctx, collateral)?;

        vault.amount = amount;
        vault.interest = interest;
        vault.collateral = collateral;
        vault.duration_seconds = duration_seconds;
        vault.funded_at = 0;
        vault.due_at = 0;
        vault.lender = Pubkey::default();
        vault.usdc_mint = Pubkey::default();
        vault.status = STATUS_REQUESTED;

        Ok(())
    }

    pub fn post_counter_offer(
        ctx: Context<PostCounterOffer>,
        amount: u64,
        interest: u64,
        collateral: u64,
        duration_seconds: i64,
    ) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        require!(collateral > 0, VaultError::InvalidCollateral);
        require!(duration_seconds > 0, VaultError::InvalidDuration);
        require!(
            ctx.accounts.vault.status == STATUS_REQUESTED,
            VaultError::RequestNotOpen
        );

        let counter_offer = &mut ctx.accounts.counter_offer;
        counter_offer.vault = ctx.accounts.vault.key();
        counter_offer.lender = ctx.accounts.lender.key();
        counter_offer.request_id = ctx.accounts.vault.request_id;
        counter_offer.amount = amount;
        counter_offer.interest = interest;
        counter_offer.collateral = collateral;
        counter_offer.duration_seconds = duration_seconds;
        counter_offer.bump = ctx.bumps.counter_offer;

        Ok(())
    }

    pub fn accept_counter_offer(ctx: Context<AcceptCounterOffer>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let counter_offer = &ctx.accounts.counter_offer;

        require!(vault.status == STATUS_REQUESTED, VaultError::RequestNotOpen);
        require!(
            counter_offer.request_id == vault.request_id,
            VaultError::StaleCounterOffer
        );

        vault.amount = counter_offer.amount;
        vault.interest = counter_offer.interest;
        vault.collateral = counter_offer.collateral;
        vault.duration_seconds = counter_offer.duration_seconds;
        vault.funded_at = 0;
        vault.due_at = 0;
        vault.lender = counter_offer.lender;
        vault.usdc_mint = Pubkey::default();
        vault.status = STATUS_COUNTERED;

        Ok(())
    }

    pub fn cancel_loan_request(ctx: Context<CancelLoanRequest>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        require!(
            matches!(vault.status, STATUS_REQUESTED | STATUS_COUNTERED),
            VaultError::RequestNotOpen
        );

        release_collateral(
            &ctx.accounts.treasury.to_account_info(),
            &ctx.accounts.owner.to_account_info(),
            vault.collateral,
        )?;

        vault.reset_loan(STATUS_IDLE);

        Ok(())
    }

    pub fn fund_loan(ctx: Context<FundLoan>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        require!(
            matches!(vault.status, STATUS_REQUESTED | STATUS_COUNTERED),
            VaultError::RequestNotOpen
        );
        if vault.status == STATUS_COUNTERED {
            require_keys_eq!(
                ctx.accounts.lender.key(),
                vault.lender,
                VaultError::UnauthorizedCounterOfferLender
            );
        }

        let transfer_accounts = TransferChecked {
            from: ctx.accounts.lender_usdc.to_account_info(),
            mint: ctx.accounts.usdc_mint.to_account_info(),
            to: ctx.accounts.borrower_usdc.to_account_info(),
            authority: ctx.accounts.lender.to_account_info(),
        };
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
        );
        token::transfer_checked(transfer_ctx, vault.amount, ctx.accounts.usdc_mint.decimals)?;

        let now = Clock::get()?.unix_timestamp;
        let due_at = now
            .checked_add(vault.duration_seconds)
            .ok_or(VaultError::ArithmeticOverflow)?;

        vault.lender = ctx.accounts.lender.key();
        vault.usdc_mint = ctx.accounts.usdc_mint.key();
        vault.funded_at = now;
        vault.due_at = due_at;
        vault.status = STATUS_FUNDED;

        Ok(())
    }

    pub fn repay_loan(ctx: Context<RepayLoan>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        require!(vault.status == STATUS_FUNDED, VaultError::LoanNotFunded);
        require!(
            Clock::get()?.unix_timestamp <= vault.due_at,
            VaultError::LoanExpired
        );

        let total_due = vault
            .amount
            .checked_add(vault.interest)
            .ok_or(VaultError::ArithmeticOverflow)?;

        let transfer_accounts = TransferChecked {
            from: ctx.accounts.owner_usdc.to_account_info(),
            mint: ctx.accounts.usdc_mint.to_account_info(),
            to: ctx.accounts.lender_usdc.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
        );
        token::transfer_checked(transfer_ctx, total_due, ctx.accounts.usdc_mint.decimals)?;

        release_collateral(
            &ctx.accounts.treasury.to_account_info(),
            &ctx.accounts.owner.to_account_info(),
            vault.collateral,
        )?;

        vault.reset_loan(STATUS_REPAID);

        Ok(())
    }

    pub fn liquidate_loan(ctx: Context<LiquidateLoan>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        require!(vault.status == STATUS_FUNDED, VaultError::LoanNotFunded);
        require!(
            Clock::get()?.unix_timestamp > vault.due_at,
            VaultError::LoanNotExpired
        );

        release_collateral(
            &ctx.accounts.treasury.to_account_info(),
            &ctx.accounts.lender.to_account_info(),
            vault.collateral,
        )?;

        vault.reset_loan(STATUS_LIQUIDATED);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + VaultState::INIT_SPACE,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultState>,
    #[account(
        init,
        payer = owner,
        space = 8 + VaultTreasury::INIT_SPACE,
        seeds = [TREASURY_SEED, vault.key().as_ref()],
        bump
    )]
    pub treasury: Account<'info, VaultTreasury>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OpenLoanRequest<'info> {
    #[account(
        mut,
        has_one = owner,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump = vault.vault_bump
    )]
    pub vault: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [TREASURY_SEED, vault.key().as_ref()],
        bump = vault.treasury_bump
    )]
    pub treasury: Account<'info, VaultTreasury>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PostCounterOffer<'info> {
    #[account(
        seeds = [VAULT_SEED, vault.owner.as_ref()],
        bump = vault.vault_bump
    )]
    pub vault: Account<'info, VaultState>,
    #[account(
        init_if_needed,
        payer = lender,
        space = 8 + CounterOffer::INIT_SPACE,
        seeds = [COUNTER_OFFER_SEED, vault.key().as_ref(), lender.key().as_ref()],
        bump
    )]
    pub counter_offer: Account<'info, CounterOffer>,
    #[account(mut)]
    pub lender: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AcceptCounterOffer<'info> {
    #[account(
        mut,
        has_one = owner,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump = vault.vault_bump
    )]
    pub vault: Account<'info, VaultState>,
    #[account(
        constraint = counter_offer.vault == vault.key() @ VaultError::CounterOfferMismatch
    )]
    pub counter_offer: Account<'info, CounterOffer>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelLoanRequest<'info> {
    #[account(
        mut,
        has_one = owner,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump = vault.vault_bump
    )]
    pub vault: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [TREASURY_SEED, vault.key().as_ref()],
        bump = vault.treasury_bump
    )]
    pub treasury: Account<'info, VaultTreasury>,
    #[account(mut)]
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct FundLoan<'info> {
    #[account(
        mut,
        seeds = [VAULT_SEED, vault.owner.as_ref()],
        bump = vault.vault_bump
    )]
    pub vault: Account<'info, VaultState>,
    #[account(mut)]
    pub lender: Signer<'info>,
    #[account(
        mut,
        constraint = lender_usdc.owner == lender.key() @ VaultError::InvalidTokenAuthority,
        constraint = lender_usdc.mint == usdc_mint.key() @ VaultError::MintMismatch,
    )]
    pub lender_usdc: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = borrower_usdc.owner == vault.owner @ VaultError::InvalidBorrowerTokenAccount,
        constraint = borrower_usdc.mint == usdc_mint.key() @ VaultError::MintMismatch,
    )]
    pub borrower_usdc: Account<'info, TokenAccount>,
    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RepayLoan<'info> {
    #[account(
        mut,
        has_one = owner,
        seeds = [VAULT_SEED, owner.key().as_ref()],
        bump = vault.vault_bump
    )]
    pub vault: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [TREASURY_SEED, vault.key().as_ref()],
        bump = vault.treasury_bump
    )]
    pub treasury: Account<'info, VaultTreasury>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        constraint = owner_usdc.owner == owner.key() @ VaultError::InvalidTokenAuthority,
        constraint = owner_usdc.mint == usdc_mint.key() @ VaultError::MintMismatch,
    )]
    pub owner_usdc: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = lender_usdc.owner == vault.lender @ VaultError::InvalidLenderTokenAccount,
        constraint = lender_usdc.mint == usdc_mint.key() @ VaultError::MintMismatch,
    )]
    pub lender_usdc: Account<'info, TokenAccount>,
    #[account(address = vault.usdc_mint @ VaultError::MintMismatch)]
    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct LiquidateLoan<'info> {
    #[account(
        mut,
        seeds = [VAULT_SEED, vault.owner.as_ref()],
        bump = vault.vault_bump
    )]
    pub vault: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [TREASURY_SEED, vault.key().as_ref()],
        bump = vault.treasury_bump
    )]
    pub treasury: Account<'info, VaultTreasury>,
    #[account(mut, address = vault.lender @ VaultError::UnauthorizedLender)]
    pub lender: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub owner: Pubkey,
    pub lender: Pubkey,
    pub usdc_mint: Pubkey,
    pub request_id: u64,
    pub amount: u64,
    pub interest: u64,
    pub collateral: u64,
    pub duration_seconds: i64,
    pub funded_at: i64,
    pub due_at: i64,
    pub vault_bump: u8,
    pub treasury_bump: u8,
    pub status: u8,
}

impl VaultState {
    fn has_pending_loan(&self) -> bool {
        matches!(
            self.status,
            STATUS_REQUESTED | STATUS_COUNTERED | STATUS_FUNDED
        )
    }

    fn reset_loan(&mut self, status: u8) {
        self.status = status;
        self.lender = Pubkey::default();
        self.usdc_mint = Pubkey::default();
        self.amount = 0;
        self.interest = 0;
        self.collateral = 0;
        self.duration_seconds = 0;
        self.funded_at = 0;
        self.due_at = 0;
    }
}

#[account]
#[derive(InitSpace)]
pub struct CounterOffer {
    pub vault: Pubkey,
    pub lender: Pubkey,
    pub request_id: u64,
    pub amount: u64,
    pub interest: u64,
    pub collateral: u64,
    pub duration_seconds: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct VaultTreasury {
    pub bump: u8,
}

fn release_collateral<'info>(
    treasury: &AccountInfo<'info>,
    recipient: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    require!(
        treasury.lamports() >= amount,
        VaultError::InsufficientTreasuryBalance
    );

    **treasury.try_borrow_mut_lamports()? -= amount;
    **recipient.try_borrow_mut_lamports()? += amount;

    Ok(())
}

#[error_code]
pub enum VaultError {
    #[msg("Loan request amount must be greater than zero")]
    InvalidAmount,
    #[msg("Collateral amount must be greater than zero")]
    InvalidCollateral,
    #[msg("Duration must be greater than zero")]
    InvalidDuration,
    #[msg("Vault already has an open or funded loan")]
    LoanAlreadyInProgress,
    #[msg("Loan request is not open")]
    RequestNotOpen,
    #[msg("Loan is not currently funded")]
    LoanNotFunded,
    #[msg("Loan has already expired and must be liquidated")]
    LoanExpired,
    #[msg("Loan has not expired yet")]
    LoanNotExpired,
    #[msg("Token account owner does not match the expected signer")]
    InvalidTokenAuthority,
    #[msg("Borrower token account must be owned by the vault owner")]
    InvalidBorrowerTokenAccount,
    #[msg("Lender token account must be owned by the recorded lender")]
    InvalidLenderTokenAccount,
    #[msg("Mint mismatch between token accounts and mint")]
    MintMismatch,
    #[msg("Only the recorded lender can liquidate this loan")]
    UnauthorizedLender,
    #[msg("Only the accepted counter-offer lender can fund this loan")]
    UnauthorizedCounterOfferLender,
    #[msg("Counter offer does not belong to this vault")]
    CounterOfferMismatch,
    #[msg("Counter offer is stale for the current request")]
    StaleCounterOffer,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Treasury does not hold enough collateral")]
    InsufficientTreasuryBalance,
}
