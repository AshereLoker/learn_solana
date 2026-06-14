use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Cannot untake before grace period")]
    CantTakeEarly,
}
