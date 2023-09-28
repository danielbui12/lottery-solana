use anchor_lang::{
    prelude::*,
    solana_program::{clock::Clock, hash::hash, program::invoke, system_instruction::transfer},
};

mod constants;
mod errors;
mod states;
use crate::{constants::*, errors::*, states::*};

declare_id!("D5fZnKmT4GNTybNoHCGNjhNLt1JLXFhwF8qaAt6ajzXn");

#[program]
mod lottery {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn create_lottery(ctx: Context<CreateLottery>, ticket_price: u64) -> Result<()> {
        let lottery: &mut Account<Lottery> = &mut ctx.accounts.lottery;
        let master: &mut Account<Master> = &mut ctx.accounts.master;

        // increment the ticket id
        master.last_id = master.last_id.checked_add(1).unwrap();

        // set lottery values
        lottery.id = master.last_id;
        lottery.authority = ctx.accounts.authority.key();
        lottery.ticket_price = ticket_price;

        Ok(())
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>, lottery_id: u32) -> Result<()> {
        let lottery: &mut Account<Lottery> = &mut ctx.accounts.lottery;
        let ticket: &mut Account<Ticket> = &mut ctx.accounts.ticket;
        let buyer: &mut Signer = &mut ctx.accounts.buyer;

        if lottery.winner_id.is_some() {
            return err!(LotteryError::WinnerAlreadyExists);
        }

        // Transfer SOL to Lottery PDA
        invoke(
            &transfer(&buyer.key(), &lottery.key(), lottery.ticket_price),
            &[
                buyer.to_account_info(),
                lottery.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        lottery.last_ticket_id += 1;
        ticket.id = lottery.last_ticket_id;
        ticket.authority = buyer.key();

        Ok(())
    }

    pub fn pick_winner(ctx: Context<PickWinner>, lottery_id: u32) -> Result<()> {
        let lottery: &mut Account<Lottery> = &mut ctx.accounts.lottery;

        if lottery.winner_id.is_some() {
            return err!(LotteryError::WinnerAlreadyExists);
        }

        if lottery.last_ticket_id == 0 {
            return err!(LotteryError::NoTicket);
        }

        // pick a psudo-random winner
        let clock = Clock::get()?;
        let psudo_random_number: u32 = ((u64::from_le_bytes(
            <[u8; 8]>::try_from(&hash(&clock.unix_timestamp.to_be_bytes()).to_bytes()[..8])
                .unwrap(),
        ) * clock.slot)
            % u32::MAX as u64) as u32;

        let winner_id: u32 = psudo_random_number % lottery.last_ticket_id + 1;
        lottery.winner_id = Some(winner_id);

        Ok(())
    }

    pub fn claim_price(ctx: Context<ClaimPrice>, lottery_id: u32, ticket_id: u32) -> Result<()> {
        let lottery: &mut Account<Lottery> = &mut ctx.accounts.lottery;
        let ticket: &mut Account<Ticket> = &mut ctx.accounts.ticket;
        let winner: &mut Signer = &mut ctx.accounts.authority;

        if lottery.claimed {
            return err!(LotteryError::AlreadyClaimed);
        }

        match lottery.winner_id {
            Some(winner_id) => {
                if winner_id != ticket.id {
                    return err!(LotteryError::InvalidWinner);
                }
            }
            None => return err!(LotteryError::WinnerNotChosen),
        }

        // pay winner
        let price = lottery
            .ticket_price
            .checked_mul(lottery.last_ticket_id.into())
            .unwrap();

        **lottery.to_account_info().try_borrow_mut_lamports()? -= price;
        **winner.to_account_info().try_borrow_mut_lamports()? += price;

        lottery.claimed = true;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = signer, space = 4 + 8, seeds = [MASTER_SEED.as_bytes()], bump)]
    pub master: Account<'info, Master>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateLottery<'info> {
    #[account(
        init,
        payer = authority,
        space = 4 + 32 + 8 + 4 + 1 + 4 + 1 + 8,
        seeds = [LOTTERY_SEED.as_bytes(), &(master.last_id + 1).to_le_bytes()],
        bump
    )]
    pub lottery: Account<'info, Lottery>,

    #[account(mut, seeds = [MASTER_SEED.as_bytes()], bump)]
    pub master: Account<'info, Master>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(lottery_id: u32)]
pub struct BuyTicket<'info> {
    #[account(
        mut,
        seeds = [LOTTERY_SEED.as_bytes(), &lottery_id.to_le_bytes()],
        bump
    )]
    pub lottery: Account<'info, Lottery>,

    #[account(
        init,
        payer = buyer,
        space = 4 + 4 + 4 + 32 + 8,
        seeds = [
            TICKET_SEED.as_bytes(),
            lottery.key().as_ref(),
            &(lottery.last_ticket_id + 1).to_le_bytes()
        ],
        bump,
    )]
    pub ticket: Account<'info, Ticket>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(lottery_id: u32)]
pub struct PickWinner<'info> {
    #[account(
        mut,
        seeds = [LOTTERY_SEED.as_bytes(), &lottery_id.to_le_bytes()],
        bump,
        has_one = authority,
    )]
    pub lottery: Account<'info, Lottery>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(lottery_id: u32, ticket_id: u32)]
pub struct ClaimPrice<'info> {
    #[account(
        mut,
        seeds = [LOTTERY_SEED.as_bytes(), &lottery_id.to_le_bytes()],
        bump,
    )]
    pub lottery: Account<'info, Lottery>,

    #[account(
        mut,
        seeds = [
            TICKET_SEED.as_bytes(),
            lottery.key().as_ref(),
            &ticket_id.to_le_bytes(),
        ],
        bump,
        has_one = authority
    )]
    pub ticket: Account<'info, Ticket>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}
