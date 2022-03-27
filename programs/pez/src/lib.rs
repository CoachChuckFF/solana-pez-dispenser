use spl_associated_token_account::get_associated_token_address;
use anchor_lang::prelude::*;
use anchor_spl::token::*;
use std::convert::Into;

const ACCOUNT_DISCRIMINATOR: usize = 8;
const MAX_NAME_LEN: usize = 32;

declare_id!("5oVf3ZxZ4JSZ1BhSpXERNssnFYXx49cTTKkbEz6C1Fu5");

#[program]
pub mod pez {
    use super::*;
    pub fn create_pez(
        ctx: Context<CreatePez>,
        params: CreatePezParams,
    ) -> ProgramResult {

        let dispenser = &mut ctx.accounts.dispenser;

        if params.amount_to_load > ctx.accounts.owner_candy_vault.amount { return Err(ErrorCode::NotEnoughToLoad.into()); }

        if params.name.len() > MAX_NAME_LEN - 1 { return Err(ErrorCode::NameTooLong.into()); }
        if params.candy_per_pull == 0 { return Err(ErrorCode::NeedCandyPerPull.into()); }
        if params.candy_per_wallet == 0 { return Err(ErrorCode::NeedCandyPerWallet.into()); }

        // Check Gatekeeper
        let gatekeeper = Pubkey::create_program_address(
            &[
                dispenser.to_account_info().key.as_ref(), 
                &[params.nonce]
            ],
            ctx.program_id,
        )
        .map_err(|_| ErrorCode::BadGateKeeper)?;

        if &gatekeeper != ctx.accounts.gatekeeper.key {
            return Err(ErrorCode::BadGateKeeper.into());
        }

        // TX 
        if params.amount_to_load > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.owner_candy_vault.to_account_info().clone(),
                to: ctx.accounts.candy_shaft.to_account_info().clone(),
                authority: ctx.accounts.owner.to_account_info().clone(),
            };
            let cpi_program = ctx.accounts.token_program.clone();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    
            let token_tx_result = transfer(cpi_ctx, params.amount_to_load);
    
            if !token_tx_result.is_ok() {
                return Err(ErrorCode::CouldNotTX.into());
            }
        }

        // Set State
        dispenser.name = String::from(params.name);
        dispenser.dispenser = dispenser.key();
        dispenser.owner = ctx.accounts.owner.key();
        dispenser.gatekeeper = gatekeeper;
        dispenser.nonce = params.nonce;
        dispenser.candy_mint = ctx.accounts.candy_shaft.mint;
        dispenser.candy_shaft = ctx.accounts.candy_shaft.key();

        dispenser.candy_per_pull = params.candy_per_pull;
        dispenser.candy_per_wallet = params.candy_per_wallet;
        dispenser.candy_taken = 0;

        Ok(())
    }

    pub fn update_pez(
        ctx: Context<UpdatePez>,
        params: UpdatePezParams,
    ) -> ProgramResult {

        let dispenser = &mut ctx.accounts.dispenser;

        if params.name.len() > MAX_NAME_LEN { return Err(ErrorCode::NameTooLong.into()); }

        // Set State
        if params.name.len() != 0 {
            dispenser.name = String::from(params.name);
        }

        if  params.candy_per_pull != 0 {
            dispenser.candy_per_pull = params.candy_per_pull;
        }

        if  params.candy_per_wallet != 0 {
            dispenser.candy_per_wallet = params.candy_per_wallet;
        }

        Ok(())
    }

    pub fn load_pez(
        ctx: Context<LoadPez>,
        params: LoadPezParams,
    ) -> ProgramResult {

        if params.amount_to_load > ctx.accounts.owner_candy_vault.amount { return Err(ErrorCode::NotEnoughToLoad.into()); }

        // TX 
        if params.amount_to_load > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.owner_candy_vault.to_account_info().clone(),
                to: ctx.accounts.candy_shaft.to_account_info().clone(),
                authority: ctx.accounts.owner.to_account_info().clone(),
            };
            let cpi_program = ctx.accounts.token_program.clone();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    
            let token_tx_result = transfer(cpi_ctx, params.amount_to_load);
    
            if !token_tx_result.is_ok() {
                return Err(ErrorCode::CouldNotTX.into());
            }
        }

        Ok(())
    }

    pub fn empty_pez(
        ctx: Context<EmptyPez>,
        params: EmptyPezParams,
    ) -> ProgramResult {

        let dispenser = &mut ctx.accounts.dispenser;

        if params.amount_to_empty > ctx.accounts.candy_shaft.amount { return Err(ErrorCode::NotEnoughCandy.into()); }

        // TX 
        if params.amount_to_empty > 0 {
            let seeds = &[
                dispenser.to_account_info().key.as_ref(),
                &[dispenser.nonce],
            ];
            let signer = &[&seeds[..]];
            let cpi_program = ctx.accounts.token_program.clone();

            let output_tx = Transfer {
                from: ctx.accounts.candy_shaft.to_account_info().clone(),
                to: ctx.accounts.owner_candy_vault.to_account_info().clone(),
                authority: ctx.accounts.gatekeeper.clone(),
            };
            let output_cpi = CpiContext::new_with_signer(cpi_program.clone(), output_tx, signer);
            let output_tx_result = transfer(output_cpi, params.amount_to_empty);

            if !output_tx_result.is_ok() {
                return Err(ErrorCode::CouldNotTX.into());
            }
        }

        Ok(())
    }

    pub fn take_pez(
        ctx: Context<TakePez>
    ) -> ProgramResult {
        let dispenser = &mut ctx.accounts.dispenser;

        if ctx.accounts.taker_candy_vault.amount >= dispenser.candy_per_wallet { return Err(ErrorCode::OnlyTakeX.into()); }
        if ctx.accounts.candy_shaft.amount < dispenser.candy_per_pull { return Err(ErrorCode::NotEnoughCandy.into()); }

        // TX 
        let seeds = &[
            dispenser.to_account_info().key.as_ref(),
            &[dispenser.nonce],
        ];
        let signer = &[&seeds[..]];
        let cpi_program = ctx.accounts.token_program.clone();

        let output_tx = Transfer {
            from: ctx.accounts.candy_shaft.to_account_info().clone(),
            to: ctx.accounts.taker_candy_vault.to_account_info().clone(),
            authority: ctx.accounts.gatekeeper.clone(),
        };
        let output_cpi = CpiContext::new_with_signer(cpi_program.clone(), output_tx, signer);
        let output_tx_result = transfer(output_cpi, dispenser.candy_per_pull);

        if !output_tx_result.is_ok() {
            return Err(ErrorCode::CouldNotTX.into());
        }

        // Update State
        dispenser.candy_taken += dispenser.candy_per_pull;

        Ok(())
    }
}

// ------------ CREATE PEZ -------------------------------
#[derive(Accounts)]
#[instruction(params: CreatePezParams)]
pub struct CreatePez<'info> {
    #[account(
        init, 
        payer = owner, 
        space = get_pez_size()
    )]
    pub dispenser: Account<'info, PezDispenser>,
    #[account(
        seeds = [dispenser.to_account_info().key.as_ref()],
        bump = params.nonce,
    )]
    pub gatekeeper: AccountInfo<'info>,

    #[account(
        mut, 
        constraint = &candy_shaft.owner == gatekeeper.key
        && candy_shaft.mint == owner_candy_vault.mint
        && get_associated_token_address(&gatekeeper.key(), &candy_shaft.mint) == candy_shaft.key()
    )]
    pub candy_shaft: Account<'info, TokenAccount>,

    #[account(
        mut, 
        constraint = &owner_candy_vault.owner == owner.key
        && get_associated_token_address(&owner.key(), &owner_candy_vault.mint) == owner_candy_vault.key()
    )]
    pub owner_candy_vault: Account<'info, TokenAccount>,

    // Signers
    #[account(mut)]
    pub owner: Signer<'info>, 
    pub system_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CreatePezParams {
    pub nonce: u8,
    pub amount_to_load: u64,
    pub name: String,
    pub candy_per_wallet: u64, //usally 1
    pub candy_per_pull: u64, //usally 1
}

// ------------ UPDATE PEZ -------------------------------
#[derive(Accounts)]
#[instruction(params: UpdatePezParams)]
pub struct UpdatePez<'info> {
    #[account(
        mut, 
        has_one = owner,
        constraint = dispenser.owner == owner.key() 
    )]
    pub dispenser: Account<'info, PezDispenser>,

    // Signers
    #[account(mut)]
    pub owner: Signer<'info>, 
}
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UpdatePezParams {
    pub name: String,
    pub candy_per_wallet: u64, //usally 1
    pub candy_per_pull: u64, //usally 1
}

// ------------ LOAD PEZ -------------------------------
#[derive(Accounts)]
#[instruction(params: LoadPezParams)]
pub struct LoadPez<'info> {
    #[account(
        mut, 
        has_one = owner,
        constraint = dispenser.owner == owner.key() 
    )]
    pub dispenser: Account<'info, PezDispenser>,
    
    #[account(
        seeds = [dispenser.to_account_info().key.as_ref()],
        bump = dispenser.nonce,
    )]
    pub gatekeeper: AccountInfo<'info>,

    #[account(
        mut, 
        constraint = &candy_shaft.owner == gatekeeper.key
        && candy_shaft.mint == owner_candy_vault.mint
    )]
    pub candy_shaft: Account<'info, TokenAccount>,

    #[account(
        mut, 
        constraint = &owner_candy_vault.owner == owner.key
    )]
    pub owner_candy_vault: Account<'info, TokenAccount>,

    // Signers
    #[account(mut)]
    pub owner: Signer<'info>, 
    pub token_program: AccountInfo<'info>,
}
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct LoadPezParams {
    pub amount_to_load: u64,
}

// ------------ EMPTY PEZ -------------------------------
#[derive(Accounts)]
#[instruction(params: EmptyPezParams)]
pub struct EmptyPez<'info> {
    #[account(
        mut, 
        has_one = owner,
        constraint = dispenser.owner == owner.key() 
    )]
    pub dispenser: Account<'info, PezDispenser>,

    #[account(
        seeds = [dispenser.to_account_info().key.as_ref()],
        bump = dispenser.nonce,
    )]
    pub gatekeeper: AccountInfo<'info>,

    #[account(
        mut, 
        constraint = &candy_shaft.owner == gatekeeper.key
        && candy_shaft.mint == owner_candy_vault.mint
    )]
    pub candy_shaft: Account<'info, TokenAccount>,

    #[account(
        mut, 
        constraint = &owner_candy_vault.owner == owner.key
    )]
    pub owner_candy_vault: Account<'info, TokenAccount>,

    // Signers
    #[account(mut)]
    pub owner: Signer<'info>, 
    pub token_program: AccountInfo<'info>, 
}
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct EmptyPezParams {
    pub amount_to_empty: u64,
}

// ------------ TAKE PEZ -------------------------------
#[derive(Accounts)]
pub struct TakePez<'info> {
    #[account(mut)]
    pub dispenser: Account<'info, PezDispenser>,
    #[account(
        seeds = [dispenser.to_account_info().key.as_ref()],
        bump = dispenser.nonce,
    )]
    pub gatekeeper: AccountInfo<'info>,

    #[account(
        mut, 
        constraint = &candy_shaft.owner == gatekeeper.key
        && candy_shaft.mint == taker_candy_vault.mint
    )]
    pub candy_shaft: Account<'info, TokenAccount>,

    #[account(
        mut, 
        constraint = &taker_candy_vault.owner == taker.key
    )]
    pub taker_candy_vault: Account<'info, TokenAccount>,

    // Signers
    #[account(mut)]
    pub taker: Signer<'info>,
    pub token_program: AccountInfo<'info>,
}

// ------------ STRUCTS -------------------------------
#[account]
pub struct PezDispenser {
    pub name: String,
    pub dispenser: Pubkey,

    pub owner: Pubkey,
    pub gatekeeper: Pubkey,
    pub nonce: u8,

    pub candy_mint: Pubkey,
    pub candy_shaft: Pubkey,

    pub candy_per_wallet: u64, //usally 1
    pub candy_per_pull: u64, //usally 1

    pub candy_taken: u64,
}
pub fn get_pez_size () -> usize {
    return  ACCOUNT_DISCRIMINATOR +
            (MAX_NAME_LEN) + 
            (32 * 5) + //Pubkeys
            1 + //nonce
            8 * 3; //params
}


// ENUM - Error Codes
#[error]
pub enum ErrorCode {
    #[msg("Error TXing the candy")]
    CouldNotTX,

    #[msg("Not enough candy in vault")]
    NotEnoughToLoad,

    #[msg("Not enough candy in shaft")]
    NotEnoughCandy,
    #[msg("You already have the max candy")]
    OnlyTakeX,


    #[msg("Name must be less than 32 chars")]
    NameTooLong,
    #[msg("Candy per pull must be bigger than 0")]
    NeedCandyPerPull,
    #[msg("Candy per wallet must be bigger than 0")]
    NeedCandyPerWallet,

    #[msg("Bad gatekeeper")]
    BadGateKeeper,
}
