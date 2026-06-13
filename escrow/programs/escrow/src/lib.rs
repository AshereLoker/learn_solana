pub mod error;

pub use error::*;
use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount,TokenInterface, transfer_checked, TransferChecked}};

declare_id!("8rafFTdYBJKJadrveLV8n3vpk1DHPrCecFdkAtph2Nix");

#[program]
pub mod escrow {
    use super::*;

    pub fn make_offer(
        ctx: Context<MakeOffer>,
        seed: u64,
        taker: Option<Pubkey>,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<()> {
    let escrow = &mut ctx.accounts.escrow;
    escrow.seed = seed;
    escrow.maker = ctx.accounts.maker.key();
    escrow.mint_a = ctx.accounts.mint_a.key();
    escrow.mint_b = ctx.accounts.mint_b.key();
    escrow.amount_b = amount_b;
    escrow.taker = taker;
    escrow.bump = ctx.bumps.escrow;

    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.maker_token_a.to_account_info(),
                mint: ctx.accounts.mint_a.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.maker.to_account_info(),
            },
        ),
        amount_a,
        ctx.accounts.mint_a.decimals,
    )?;

        Ok(())
    }


    pub fn take_offer(ctx: Context<TakeOffer>) -> Result<()> {
        if let Some(taker) = ctx.accounts.escrow.taker {
            require!(taker == ctx.accounts.taker.key(), MyError::WrongTaker)
        }
        
        let amount_b = ctx.accounts.escrow.amount_b;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            ctx.accounts.escrow.maker.as_ref(),
            &ctx.accounts.escrow.seed.to_le_bytes(),  
            &[ctx.accounts.escrow.bump],              
        ]];
    
       

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                    TransferChecked {
                       from: ctx.accounts.vault.to_account_info(),
                        mint: ctx.accounts.mint_a.to_account_info(),
                        to: ctx.accounts.taker_token_a.to_account_info(),
                        authority: ctx.accounts.escrow.to_account_info(),
                    },
                signer_seeds,
            ), 
            ctx.accounts.vault.amount, 
            ctx.accounts.mint_a.decimals,
        )?;

        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                    TransferChecked {
                        from: ctx.accounts.taker_token_b.to_account_info(),
                        mint: ctx.accounts.mint_b.to_account_info(),
                        to: ctx.accounts.maker_token_b.to_account_info(),
                        authority: ctx.accounts.taker.to_account_info(),
                    },
             
            ), 
            amount_b, 
            ctx.accounts.mint_b.decimals,
        )?;
        Ok(())
    }

    pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            ctx.accounts.escrow.maker.as_ref(),
            &ctx.accounts.escrow.seed.to_le_bytes(),  
            &[ctx.accounts.escrow.bump],              
        ]];
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                    TransferChecked {
                       from: ctx.accounts.vault.to_account_info(),
                        mint: ctx.accounts.mint_a.to_account_info(),
                        to: ctx.accounts.maker_token_a.to_account_info(),
                        authority: ctx.accounts.escrow.to_account_info(),
                    },
                signer_seeds,
            ), 
             ctx.accounts.vault.amount, 
            ctx.accounts.mint_a.decimals,
        )?;
       Ok(())
    }
    
}

#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub taker: Option<Pubkey>,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub amount_b: u64,
    pub bump: u8,
}

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account(init, payer = maker, space = 8 + Escrow::INIT_SPACE, seeds = [b"escrow", maker.key().as_ref(), seed.to_le_bytes().as_ref()] , bump)]
    pub escrow: Account<'info, Escrow>,
    #[account(
        init, 
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_token_a: InterfaceAccount<'info, TokenAccount>,
    #[account(mint::token_program = token_program)]
    pub mint_a: InterfaceAccount<'info, Mint>,
    #[account(mint::token_program = token_program)]
    pub mint_b: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}



#[derive(Accounts)]
pub struct TakeOffer<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,
    #[account(mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
        close = maker,
    )]
    pub escrow: Box<Account<'info, Escrow>>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_token_b: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_token_a: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_token_b: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
   
}
#[derive(Accounts)]
pub struct CancelOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account(
        mut,
        has_one = maker,
        has_one = mint_a,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
        close = maker,
    )]
    pub escrow: Account<'info, Escrow>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_token_a: InterfaceAccount<'info, TokenAccount>,
    #[account(mint::token_program = token_program)]
    pub mint_a: InterfaceAccount<'info, Mint>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
