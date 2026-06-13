use anchor_lang::prelude::*;

#[error_code]
pub enum MyError {
    #[msg("Name exceed 50 chars")]
    ExceedMaxNameLen,
    #[msg("Name exceed 20 chars")]
    ExceedMaxOptionNameLen,
    #[msg("Unavaliable poll variant")]
    UnavalibalePollVariants,
    #[msg("Too much variants to create")]
    ExceedMaxVariants,
    #[msg("Can't create poll with less that 2 variant")]
    ContainAtLeastTwo,
}
