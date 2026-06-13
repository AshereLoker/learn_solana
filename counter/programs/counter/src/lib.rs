// Main entry point

pub mod error;
use anchor_lang::prelude::*;

pub use error::*;

// Entry point. On-chain address
declare_id!("8oWMa5rkb5cEyfwUmFK6ozTGYFdjmfxvDr9QDMZCcn58");

// Main isntruction logic
#[program]
pub mod counter {

    use super::*;

    pub fn initialize(ctx: Context<Create>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.authority = ctx.accounts.user.key();
        counter.count = 0;
        Ok(())
    }

    pub fn decrement(ctx: Context<Decrement>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        require!(counter.count > 0, MyError::BelowZero);
        counter.count -= 1;
        Ok(())
    }

    pub fn reset(ctx: Context<Reset>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = 0;
        Ok(())
    }

    pub fn increment_by(ctx: Context<Increment>, amount: u64) -> Result<()> {
        require!(amount <= 10, MyError::TooBigIncrement);
        let counter = &mut ctx.accounts.counter;
        counter.count += amount;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(init, payer = user,space = 8 + 32 + 8)]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Decrement<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Reset<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Counter {
    pub authority: Pubkey,
    pub count: u64,
}
