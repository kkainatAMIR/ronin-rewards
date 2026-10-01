use anchor_lang::prelude::*;

declare_id!("6Uyjo8oDGQJeb8zS1yFqwLCguc4gfUB1V4xWheAD7RYC");

#[program]
pub mod ronin_rewards {
    use super::*;

    // ============================================================
    // 1. INITIALIZE
    // ============================================================

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.reward_config;

        config.admin = ctx.accounts.admin.key();
        config.bump = ctx.bumps.reward_config;
        config.vault_bump = ctx.bumps.reward_vault;
        config.total_claimed = 0;
        config.total_claims = 0;
        config.paused = false;

        Ok(())
    }

    // ============================================================
    // 2. FUND REWARD VAULT
    // ============================================================

    pub fn fund_vault(ctx: Context<FundVault>, amount: u64) -> Result<()> {
        require!(amount > 0, RewardError::InvalidAmount);

        let transfer_accounts = anchor_lang::system_program::Transfer {
            from: ctx.accounts.admin.to_account_info(),
            to: ctx.accounts.reward_vault.to_account_info(),
        };

        let transfer_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
        );

        anchor_lang::system_program::transfer(transfer_ctx, amount)?;

        emit!(VaultFunded {
            admin: ctx.accounts.admin.key(),
            amount,
        });

        Ok(())
    }

    // ============================================================
    // 3. CLAIM REWARD
    // ============================================================
    //
    // IMPORTANT:
    // claim_id is kept as instruction data and in the event so the
    // backend/Supabase claim can still be correlated with the
    // on-chain payout.
    //
    // There is intentionally NO per-claim PDA/account anymore.
    //
    // Existing Claim PDAs created by the previous version remain
    // untouched. This version simply stops creating new ones.
    // ============================================================

    pub fn claim_reward(
        ctx: Context<ClaimReward>,
        claim_id: String,
        points_claimed: u64,
        reward_amount: u64,
    ) -> Result<()> {
        let config = &mut ctx.accounts.reward_config;

        // --------------------------------------------------------
        // Basic validation
        // --------------------------------------------------------

        require!(!config.paused, RewardError::RewardsPaused);

        require!(!claim_id.is_empty(), RewardError::InvalidClaimId);

        require!(claim_id.len() >= 8, RewardError::InvalidClaimId);

        require!(claim_id.len() <= 200, RewardError::InvalidClaimId);

        require!(points_claimed > 0, RewardError::InvalidPoints);

        require!(reward_amount > 0, RewardError::InvalidAmount);

        // --------------------------------------------------------
        // The backend/admin wallet must authorize this payout.
        //
        // The frontend cannot simply invent reward amounts because
        // admin is a required signer and RewardConfig has_one admin.
        // --------------------------------------------------------

        require!(
            ctx.accounts.admin.key() == config.admin,
            RewardError::Unauthorized
        );

        // --------------------------------------------------------
        // Calculate the vault balance.
        //
        // Keep the vault rent-exempt.
        // --------------------------------------------------------

        let vault_info = ctx.accounts.reward_vault.to_account_info();

        let minimum_balance = Rent::get()?.minimum_balance(vault_info.data_len());

        let current_balance = vault_info.lamports();

        require!(
            current_balance >= minimum_balance,
            RewardError::InvalidVaultBalance
        );

        let available_balance = current_balance
            .checked_sub(minimum_balance)
            .ok_or(RewardError::MathOverflow)?;

        require!(
            available_balance >= reward_amount,
            RewardError::InsufficientVaultBalance
        );

        // --------------------------------------------------------
        // Transfer SOL from program-owned vault PDA to recipient.
        //
        // Because RewardVault is owned by this program, we perform
        // the lamport movement directly instead of using the System
        // Program transfer CPI.
        // --------------------------------------------------------

        **vault_info.try_borrow_mut_lamports()? -= reward_amount;

        **ctx
            .accounts
            .recipient
            .to_account_info()
            .try_borrow_mut_lamports()? += reward_amount;

        // --------------------------------------------------------
        // Update global accounting.
        // --------------------------------------------------------

        config.total_claimed = config
            .total_claimed
            .checked_add(reward_amount)
            .ok_or(RewardError::MathOverflow)?;

        config.total_claims = config
            .total_claims
            .checked_add(1)
            .ok_or(RewardError::MathOverflow)?;

        emit!(RewardClaimed {
            claim_id,
            wallet_address: ctx.accounts.recipient.key(),
            points_claimed,
            reward_amount,
        });

        Ok(())
    }

    // ============================================================
    // 4. PAUSE / UNPAUSE
    // ============================================================

    pub fn set_paused(ctx: Context<AdminOnly>, paused: bool) -> Result<()> {
        let config = &mut ctx.accounts.reward_config;

        require!(
            ctx.accounts.admin.key() == config.admin,
            RewardError::Unauthorized
        );

        config.paused = paused;

        emit!(RewardsPauseChanged { paused });

        Ok(())
    }

    // ============================================================
    // 5. UPDATE ADMIN
    // ============================================================

    pub fn update_admin(ctx: Context<AdminOnly>, new_admin: Pubkey) -> Result<()> {
        require!(new_admin != Pubkey::default(), RewardError::InvalidAdmin);

        let config = &mut ctx.accounts.reward_config;

        require!(
            ctx.accounts.admin.key() == config.admin,
            RewardError::Unauthorized
        );

        let old_admin = config.admin;

        config.admin = new_admin;

        emit!(AdminUpdated {
            old_admin,
            new_admin,
        });

        Ok(())
    }

    // ============================================================
    // 6. WITHDRAW UNUSED VAULT FUNDS
    // ============================================================
    //
    // Admin can withdraw excess SOL.
    //
    // We always keep the vault rent-exempt.
    //

    pub fn withdraw_vault(ctx: Context<WithdrawVault>, amount: u64) -> Result<()> {
        require!(amount > 0, RewardError::InvalidAmount);

        let config = &ctx.accounts.reward_config;

        require!(
            ctx.accounts.admin.key() == config.admin,
            RewardError::Unauthorized
        );

        let vault_info = ctx.accounts.reward_vault.to_account_info();

        let minimum_balance = Rent::get()?.minimum_balance(vault_info.data_len());

        let current_balance = vault_info.lamports();

        require!(
            current_balance >= minimum_balance,
            RewardError::InvalidVaultBalance
        );

        let available_balance = current_balance
            .checked_sub(minimum_balance)
            .ok_or(RewardError::MathOverflow)?;

        require!(
            available_balance >= amount,
            RewardError::InsufficientVaultBalance
        );

        **vault_info.try_borrow_mut_lamports()? -= amount;

        **ctx
            .accounts
            .admin
            .to_account_info()
            .try_borrow_mut_lamports()? += amount;

        emit!(VaultWithdrawn {
            admin: ctx.accounts.admin.key(),
            amount,
        });

        Ok(())
    }
}

// ================================================================
// ACCOUNTS
// ================================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + RewardConfig::INIT_SPACE,
        seeds = [b"reward_config"],
        bump
    )]
    pub reward_config: Account<'info, RewardConfig>,

    #[account(
        init,
        payer = admin,
        space = 8 + RewardVault::INIT_SPACE,
        seeds = [b"reward_vault"],
        bump
    )]
    pub reward_vault: Account<'info, RewardVault>,

    pub system_program: Program<'info, System>,
}

// ================================================================
// FUND VAULT
// ================================================================

#[derive(Accounts)]
pub struct FundVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"reward_config"],
        bump = reward_config.bump,
        has_one = admin
    )]
    pub reward_config: Account<'info, RewardConfig>,

    #[account(
        mut,
        seeds = [b"reward_vault"],
        bump = reward_config.vault_bump
    )]
    pub reward_vault: Account<'info, RewardVault>,

    pub system_program: Program<'info, System>,
}

// ================================================================
// CLAIM
// ================================================================
//
// IMPORTANT CHANGE:
//
// There is NO `claim` account anymore.
//
// There is also NO `system_program` because the claim instruction
// no longer creates an account.
//
// Required accounts are only:
//
//   1. admin
//   2. reward_config
//   3. reward_vault
//   4. recipient
//
// ================================================================

#[derive(Accounts)]
#[instruction(claim_id: String)]
pub struct ClaimReward<'info> {
    // Backend/admin signer
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"reward_config"],
        bump = reward_config.bump,
        has_one = admin
    )]
    pub reward_config: Account<'info, RewardConfig>,

    // SOL reward vault
    #[account(
        mut,
        seeds = [b"reward_vault"],
        bump = reward_config.vault_bump
    )]
    pub reward_vault: Account<'info, RewardVault>,

    // User receiving SOL
    #[account(mut)]
    pub recipient: SystemAccount<'info>,
}

// ================================================================
// ADMIN ONLY
// ================================================================

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"reward_config"],
        bump = reward_config.bump,
        has_one = admin
    )]
    pub reward_config: Account<'info, RewardConfig>,
}

// ================================================================
// WITHDRAW
// ================================================================

#[derive(Accounts)]
pub struct WithdrawVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"reward_config"],
        bump = reward_config.bump,
        has_one = admin
    )]
    pub reward_config: Account<'info, RewardConfig>,

    #[account(
        mut,
        seeds = [b"reward_vault"],
        bump = reward_config.vault_bump
    )]
    pub reward_vault: Account<'info, RewardVault>,
}

// ================================================================
// STATE: REWARD CONFIG
// ================================================================

#[account]
#[derive(InitSpace)]
pub struct RewardConfig {
    // Backend/admin authority
    pub admin: Pubkey,

    // PDA bump
    pub bump: u8,

    // Reward vault PDA bump
    pub vault_bump: u8,

    // Total SOL paid
    pub total_claimed: u64,

    // Number of completed on-chain claims
    pub total_claims: u64,

    // Emergency pause
    pub paused: bool,
}

// ================================================================
// STATE: REWARD VAULT
// ================================================================

#[account]
#[derive(InitSpace)]
pub struct RewardVault {
    // Identifier / version marker
    pub version: u8,
}

// ================================================================
// STATE: CLAIM
// ================================================================
//
// IMPORTANT:
//
// This struct is intentionally kept because old Claim PDAs already
// exist on-chain from the previous program version.
//
// We do NOT close, migrate, or modify those accounts.
//
// The upgraded claim_reward instruction simply no longer creates
// new Claim accounts.
//

#[account]
#[derive(InitSpace)]
pub struct Claim {
    // Same claim_id generated by your backend.
    //
    // DB:
    // reward_claims.claim_id
    //
    #[max_len(200)]
    pub claim_id: String,

    // Same as:
    // reward_claims.wallet_address
    //
    pub wallet_address: Pubkey,

    // Same as:
    // reward_claims.points_claimed
    //
    pub points_claimed: u64,

    // Same as:
    // reward_claims.reward_amount
    //
    // IMPORTANT:
    // This is stored in LAMPORTS on-chain.
    //
    pub reward_amount: u64,

    // Prevent duplicate claim.
    pub claimed: bool,

    // PDA bump.
    pub bump: u8,
}

// ================================================================
// EVENTS
// ================================================================

#[event]
pub struct RewardClaimed {
    pub claim_id: String,

    pub wallet_address: Pubkey,

    pub points_claimed: u64,

    pub reward_amount: u64,
}

#[event]
pub struct VaultFunded {
    pub admin: Pubkey,

    pub amount: u64,
}

#[event]
pub struct VaultWithdrawn {
    pub admin: Pubkey,

    pub amount: u64,
}

#[event]
pub struct RewardsPauseChanged {
    pub paused: bool,
}

#[event]
pub struct AdminUpdated {
    pub old_admin: Pubkey,

    pub new_admin: Pubkey,
}

// ================================================================
// ERRORS
// ================================================================

#[error_code]
pub enum RewardError {
    #[msg("Unauthorized.")]
    Unauthorized,

    #[msg("Invalid claim ID.")]
    InvalidClaimId,

    #[msg("Invalid points amount.")]
    InvalidPoints,

    #[msg("Invalid reward amount.")]
    InvalidAmount,

    #[msg("Reward vault has insufficient available SOL.")]
    InsufficientVaultBalance,

    #[msg("Invalid vault balance.")]
    InvalidVaultBalance,

    #[msg("Arithmetic overflow.")]
    MathOverflow,

    #[msg("Rewards are currently paused.")]
    RewardsPaused,

    #[msg("Invalid admin address.")]
    InvalidAdmin,
}
