use anchor_lang::prelude::*;

#[error_code]
pub enum MyError {
    #[msg("Cant accept direct offer; has no authority")]
    WrongTaker,
}
