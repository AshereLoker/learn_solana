pub mod error;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface, transfer_checked, TransferChecked},
};
pub use error::ErrorCode;

use anchor_lang::prelude::*;

declare_id!("62t5hLMRW19m1c9Th9JbdYvYeJQsKmzWnEVk2qExbt4b");

#[program]
pub mod stacking {
    use anchor_lang::solana_program::clock::Clock;

    use super::*;

    pub fn create_reward(ctx: Context<CreateReward>, seed: u64, rate: u64) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.seed = seed;
        config.admin = ctx.accounts.admin.key();
        config.lock_mint = ctx.accounts.lock_mint.key();
        config.reward_mint = ctx.accounts.reward_mint.key();
        config.reward_rate = rate;
        config.bump = ctx.bumps.config;

        Ok(())
    }

    // Админ пополняет пул наград
    pub fn fund_rewards(ctx: Context<FundRewards>, amount: u64) -> Result<()> {
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(), 
                TransferChecked { 
                    from: ctx.accounts.admin_reward_token.to_account_info(), 
                    mint: ctx.accounts.reward_mint.to_account_info(), 
                    to: ctx.accounts.reward_vault.to_account_info(), 
                    authority: ctx.accounts.admin.to_account_info(), 
                }, 
            ), 
            amount, 
            ctx.accounts.reward_mint.decimals,
        )?;

        Ok(())
    }

    // Пользователь депозитит токены на N секунд
    pub fn stake(ctx: Context<Stake>, seed: u64, amount: u64, lock_period: i64) -> Result<()> {
        let clock = Clock::get()?;
        let entry = &mut ctx.accounts.entry;
        entry.lock_at = clock.unix_timestamp;
        entry.staker = ctx.accounts.staker.key();
        entry.seed = seed;
        entry.amount = amount;
        entry.lock_period = lock_period;
        entry.lock_mint = ctx.accounts.lock_mint.key();
        entry.bump = ctx.bumps.entry;


        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(), 
                TransferChecked { 
                    from: ctx.accounts.staker_lock_token.to_account_info(), 
                    mint: ctx.accounts.lock_mint.to_account_info(), 
                    to: ctx.accounts.lock_vault.to_account_info(), 
                    authority: ctx.accounts.staker.to_account_info(), 
                }, 
            ), 
            amount, 
            ctx.accounts.lock_mint.decimals,
        )?;


        Ok(())
    }

    // Пользователь забирает токены + награду после lock_period
    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let lock_period = ctx.accounts.entry.lock_at + ctx.accounts.entry.lock_period;
        require!(now > lock_period, ErrorCode::CantTakeEarly);
        let reward = ctx.accounts.entry.amount * ctx.accounts.config.reward_rate * (now - ctx.accounts.entry.lock_at) as u64 / 1_000_000;

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            ctx.accounts.config.admin.as_ref(),
            &ctx.accounts.config.seed.to_le_bytes(),  
            &[ctx.accounts.config.bump],              
        ]];

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(), 
                TransferChecked { 
                    from: ctx.accounts.reward_vault.to_account_info(), 
                    mint: ctx.accounts.reward_mint.to_account_info(), 
                    to: ctx.accounts.staker_reward_token.to_account_info(), 
                    authority: ctx.accounts.config.to_account_info(), 
                }, 
                signer_seeds,
            ), 
            reward, 
            ctx.accounts.reward_mint.decimals,
        )?;

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"entry",
            ctx.accounts.entry.staker.as_ref(),
            &ctx.accounts.entry.seed.to_le_bytes(),  
            &[ctx.accounts.entry.bump],              
        ]];

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(), 
                TransferChecked { 
                    from: ctx.accounts.lock_vault.to_account_info(), 
                    mint: ctx.accounts.lock_mint.to_account_info(), 
                    to: ctx.accounts.staker_lock_token.to_account_info(), 
                    authority: ctx.accounts.entry.to_account_info(), 
                }, 
                signer_seeds,
            ), 
            ctx.accounts.entry.amount, 
            ctx.accounts.lock_mint.decimals,
        )?;

        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct StakeEntry {
    pub bump: u8,
    pub amount: u64,
    pub lock_at: i64,
    pub lock_period: i64,
    pub seed: u64,
    pub staker: Pubkey,
    pub lock_mint: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct StakeConfig {
    pub bump: u8,
    pub reward_rate: u64,
    pub seed: u64,
    pub admin: Pubkey,
    pub lock_mint: Pubkey,
    pub reward_mint: Pubkey,
}

#[derive(Accounts)]
pub struct FundRewards<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        has_one = reward_mint,
        seeds = [b"config", admin.key().as_ref(), config.seed.to_le_bytes().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, StakeConfig>,
    #[account(mint::token_program = token_program)]
    pub reward_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = reward_mint,
        associated_token::authority = admin,
        associated_token::token_program = token_program,
    )]
    pub admin_reward_token: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = reward_mint,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub reward_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct CreateReward<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + StakeConfig::INIT_SPACE,
        seeds = [b"config", admin.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump,
    )]
    pub config: Account<'info, StakeConfig>,
    #[account(mint::token_program = token_program)]
    pub lock_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mint::token_program = token_program)]
    pub reward_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Stake<'info> {
    #[account(mut)]
    pub staker: Signer<'info>,
    #[account(
        init, 
        payer = staker, 
        space = 8 + StakeEntry::INIT_SPACE,
        seeds = [b"entry", staker.key().as_ref(), seed.to_le_bytes().as_ref()], 
        bump,
    )]
    pub entry: Account<'info, StakeEntry>,
    #[account(mint::token_program = token_program)]
    pub lock_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = lock_mint,
        associated_token::authority = staker,
        associated_token::token_program = token_program,
    )]
    pub staker_lock_token: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = staker,
        associated_token::mint = lock_mint,
        associated_token::authority = entry,
        associated_token::token_program = token_program,
    )]
    pub lock_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub staker: Signer<'info>,
    #[account(mut)]
    pub admin: SystemAccount<'info>,
    #[account(
        mut,
        has_one = staker, 
        has_one = lock_mint,
        seeds = [b"entry", staker.key().as_ref(), entry.seed.to_le_bytes().as_ref()],
        bump = entry.bump,
        close = staker,
    )]
    pub entry: Account<'info, StakeEntry>,
    #[account(mint::token_program = token_program)]
    pub lock_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = lock_mint,
        associated_token::authority = staker,
        associated_token::token_program = token_program,
    )]
    pub staker_lock_token: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = reward_mint,
        associated_token::authority = staker,
        associated_token::token_program = token_program,
    )]
    pub staker_reward_token: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = lock_mint,
        associated_token::authority = entry,
        associated_token::token_program = token_program,
    )]
    pub lock_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = reward_mint,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub reward_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        seeds = [b"config", admin.key().as_ref(), config.seed.to_le_bytes().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, StakeConfig>,
    #[account(mint::token_program = token_program)]
    pub reward_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}