use anchor_lang::prelude::error_code;

#[error_code]
pub enum LotteryError {
    #[msg("Winner already exists")]
    WinnerAlreadyExists,
    #[msg("Can't choose a winner when there is no ticket")]
    NoTicket,
    #[msg("Winner has not been chosen")]
    WinnerNotChosen,
    #[msg("Invalid Winner")]
    InvalidWinner,
    #[msg("The price has already been claimed")]
    AlreadyClaimed
}