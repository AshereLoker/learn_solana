// Error handling / Custom errors

use anchor_lang::prelude::*;

#[error_code]
pub enum MyError {
    #[msg("Counter can't go below zero")]
    BelowZero,
    #[msg("No authority to reset")]
    NoAuthority,
    #[msg("Counter can't go over MAX")]
    TooBigIncrement,
}
