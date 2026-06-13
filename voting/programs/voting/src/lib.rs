pub mod error;
use anchor_lang::prelude::*;

pub use error::*;

declare_id!("7fufM9CFAfneRx9bB5SbkS4AhMzf3hVsPYLaiH96U15o");

#[program]
pub mod voting {
    use super::*;

    // Создать голосование
    pub fn create_poll(
        ctx: Context<CreatePoll>,
        title: String,
        options: Vec<String>,
    ) -> Result<()> {
        let poll = &mut ctx.accounts.poll;
        require!(title.len() <= 50, MyError::ExceedMaxNameLen);
        require!(options.len() <= 4, MyError::ExceedMaxVariants);
        for option in &options {
            require!(option.len() <= 20, MyError::ExceedMaxOptionNameLen);
        }
        require!(options.len() > 1, MyError::ContainAtLeastTwo);
        poll.authority = ctx.accounts.user.key();
        poll.title = title;
        poll.votes = vec![0u64; options.len()];
        poll.options = options;
        Ok(())
    }

    pub fn vote(ctx: Context<Vote>, option_index: u8) -> Result<()> {
        let poll = &mut ctx.accounts.poll;
        require!(
            option_index < poll.options.len() as u8,
            MyError::UnavalibalePollVariants
        );
        ctx.accounts.vote_receipt.option_index = option_index;
        poll.votes[option_index as usize] += 1;
        Ok(())
    }
}

#[account]
pub struct Poll {
    pub authority: Pubkey,    // 32 bytes
    pub title: String,        // 4 + len = 54 bytes // макс 50 символов
    pub options: Vec<String>, // 4 + (size(T) * amount) // 4 + 24 * 4 = 100 bytes // макс 4 варианта, каждый макс 20 символов
    pub votes: Vec<u64>, // 4 + (size(T) * amount) // 4 + 8 * 4 = 36 bytes счётчик для каждого варианта
}

#[account]
pub struct VoteReceipt {
    pub option_index: u8,
}

#[derive(Accounts)]
pub struct Vote<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub poll: Account<'info, Poll>,
    #[account(init, payer = authority, space = 8 + 1, seeds = [b"reciept", poll.key().as_ref(), authority.key.as_ref()], bump)]
    pub vote_receipt: Account<'info, VoteReceipt>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreatePoll<'info> {
    #[account(init, payer = user, space = 8 + 32 + 54 + 100 + 36)]
    pub poll: Account<'info, Poll>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}
