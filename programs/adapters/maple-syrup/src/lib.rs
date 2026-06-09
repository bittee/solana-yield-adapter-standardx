use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use syas_adapter_utils::{
    assert_enabled, assert_owner, assert_program, checked_add_u64, checked_sub_u64,
    mul_div_floor_u128, read_pubkey, read_u128, require_pda, require_token_account, u128_to_u64,
    CpiBuilder, USDC_MINT,
};
use syas_interface::{seeds, Deposited, StandardError, ValueReported, Withdrawn};

declare_id!("7Bw1gXZzHz1RFD1FBkqGbAfVoBTz5CYk73QQkzjw8NWf");

const SYRUP_USDC_MINT: Pubkey = pubkey!("AvZZF1YaZDziPY2RCK4oJrRVrbN3mTD9NL24hPeaZeUj");
const WHIRLPOOL_ID: Pubkey = pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
const WHIRLPOOL: Pubkey = pubkey!("6fteKNvMdv7tYmBoJHhj1jx6rHcEwC6RdSEmVpyS613J");
const TOKEN_VAULT_A: Pubkey = pubkey!("FM2RuqFYo9umA1yc5FyQn6pSDZJZ1MXAdaekJZ4dQCvi");
const TOKEN_VAULT_B: Pubkey = pubkey!("Fw6Xr45rBBrXbWJd5ZbSg44kacrKRLef4rHkZ8gWC5Ab");
const ORCA_SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

const WHIRLPOOL_SQRT_PRICE_OFFSET: usize = 65;
const WHIRLPOOL_TOKEN_MINT_A_OFFSET: usize = 101;
const WHIRLPOOL_TOKEN_MINT_B_OFFSET: usize = 181;
const Q64: u128 = 1u128 << 64;
const MIN_SQRT_PRICE_LIMIT: u128 = 4_295_048_017;
const MAX_SQRT_PRICE_LIMIT: u128 = 79_226_673_515_401_279_992_447_579_054;

#[program]
pub mod maple_syrup_adapter {
    use super::*;

    pub fn deposit(ctx: Context<StandardOp>, amount: u64, min_position_out: u64) -> Result<()> {
        validate_common(&ctx)?;
        ctx.accounts.position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;

        ctx.accounts.transfer_owner_to_vault(amount)?;
        let syrup_before = ctx.accounts.syrup_vault.amount;
        invoke_orca_swap(&ctx, amount, min_position_out, false)?;
        ctx.accounts.syrup_vault.reload()?;
        let position_out = checked_sub_u64(ctx.accounts.syrup_vault.amount, syrup_before)?;
        require!(
            position_out >= min_position_out,
            StandardError::SlippageExceeded
        );

        let new_shares = checked_add_u64(ctx.accounts.position.shares, position_out)?;
        let value = syrup_value(new_shares, &ctx.accounts.whirlpool.to_account_info())?;
        let position = &mut ctx.accounts.position;
        position.shares = new_shares;
        position.cached_value = value;

        syas_interface::set_return_u64(position_out);
        emit!(Deposited {
            owner: position.owner,
            adapter: crate::ID,
            amount_in: amount,
            position_out,
        });
        Ok(())
    }

    pub fn withdraw(
        ctx: Context<StandardOp>,
        position_amount: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        validate_common(&ctx)?;
        ctx.accounts.position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;
        require!(
            position_amount <= ctx.accounts.position.shares,
            StandardError::SlippageExceeded
        );

        let usdc_before = ctx.accounts.adapter_vault.amount;
        invoke_orca_swap(&ctx, position_amount, min_amount_out, true)?;
        ctx.accounts.adapter_vault.reload()?;
        let amount_out = checked_sub_u64(ctx.accounts.adapter_vault.amount, usdc_before)?;
        require!(
            amount_out >= min_amount_out,
            StandardError::SlippageExceeded
        );
        ctx.accounts.transfer_vault_to_owner(amount_out)?;

        let new_shares = checked_sub_u64(ctx.accounts.position.shares, position_amount)?;
        let value = syrup_value(new_shares, &ctx.accounts.whirlpool.to_account_info())?;
        let position = &mut ctx.accounts.position;
        position.shares = new_shares;
        position.cached_value = value;

        syas_interface::set_return_u64(amount_out);
        emit!(Withdrawn {
            owner: position.owner,
            adapter: crate::ID,
            position_in: position_amount,
            amount_out,
        });
        Ok(())
    }

    pub fn current_value(ctx: Context<StandardOp>) -> Result<()> {
        validate_common(&ctx)?;
        ctx.accounts.position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;
        let value = syrup_value(
            ctx.accounts.position.shares,
            &ctx.accounts.whirlpool.to_account_info(),
        )?;
        let position = &mut ctx.accounts.position;
        position.cached_value = value;
        syas_interface::set_return_u64(value);
        emit!(ValueReported {
            owner: position.owner,
            adapter: crate::ID,
            value,
        });
        Ok(())
    }
}

fn validate_common(ctx: &Context<StandardOp>) -> Result<()> {
    assert_enabled(
        &ctx.accounts.registry_entry.to_account_info(),
        &crate::ID,
        &ctx.accounts.base_mint.key(),
    )?;
    require_keys_eq!(
        ctx.accounts.base_mint.key(),
        USDC_MINT,
        StandardError::MintMismatch
    );
    assert_program(
        &ctx.accounts.whirlpool_program.to_account_info(),
        &WHIRLPOOL_ID,
    )?;
    assert_owner(&ctx.accounts.whirlpool.to_account_info(), &WHIRLPOOL_ID)?;
    require_keys_eq!(
        ctx.accounts.syrup_mint.key(),
        SYRUP_USDC_MINT,
        StandardError::MintMismatch
    );
    require_keys_eq!(
        ctx.accounts.whirlpool.key(),
        WHIRLPOOL,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.token_vault_a.key(),
        TOKEN_VAULT_A,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.token_vault_b.key(),
        TOKEN_VAULT_B,
        StandardError::InvalidProtocolAccount
    );
    require_token_account(
        &ctx.accounts.token_vault_a.to_account_info(),
        &SYRUP_USDC_MINT,
        &ctx.accounts.whirlpool.key(),
        &ctx.accounts.token_program.key(),
    )?;
    require_token_account(
        &ctx.accounts.token_vault_b.to_account_info(),
        &USDC_MINT,
        &ctx.accounts.whirlpool.key(),
        &ctx.accounts.token_program.key(),
    )?;
    require_pda(
        &ctx.accounts.oracle.key(),
        &[b"oracle", ctx.accounts.whirlpool.key().as_ref()],
        &WHIRLPOOL_ID,
    )?;
    assert_owner(&ctx.accounts.tick_array_0.to_account_info(), &WHIRLPOOL_ID)?;
    assert_owner(&ctx.accounts.tick_array_1.to_account_info(), &WHIRLPOOL_ID)?;
    assert_owner(&ctx.accounts.tick_array_2.to_account_info(), &WHIRLPOOL_ID)?;
    assert_owner(&ctx.accounts.oracle.to_account_info(), &WHIRLPOOL_ID)?;

    let state = read_whirlpool(&ctx.accounts.whirlpool.to_account_info())?;
    require_keys_eq!(
        state.token_mint_a,
        SYRUP_USDC_MINT,
        StandardError::MintMismatch
    );
    require_keys_eq!(state.token_mint_b, USDC_MINT, StandardError::MintMismatch);
    Ok(())
}

fn invoke_orca_swap(
    ctx: &Context<StandardOp>,
    amount: u64,
    threshold: u64,
    a_to_b: bool,
) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];
    let amount_specified_is_input = true;
    let sqrt_price_limit = if a_to_b {
        MIN_SQRT_PRICE_LIMIT
    } else {
        MAX_SQRT_PRICE_LIMIT
    };

    CpiBuilder::with_discriminator(ctx.accounts.whirlpool_program.to_account_info(), ORCA_SWAP)
        .arg(&amount)?
        .arg(&threshold)?
        .arg(&sqrt_price_limit)?
        .arg(&amount_specified_is_input)?
        .arg(&a_to_b)?
        .account(ctx.accounts.token_program.to_account_info(), false, false)
        .account(
            ctx.accounts.position_authority.to_account_info(),
            false,
            true,
        )
        .account(ctx.accounts.whirlpool.to_account_info(), true, false)
        .account(ctx.accounts.syrup_vault.to_account_info(), true, false)
        .account(ctx.accounts.token_vault_a.to_account_info(), true, false)
        .account(ctx.accounts.adapter_vault.to_account_info(), true, false)
        .account(ctx.accounts.token_vault_b.to_account_info(), true, false)
        .account(ctx.accounts.tick_array_0.to_account_info(), true, false)
        .account(ctx.accounts.tick_array_1.to_account_info(), true, false)
        .account(ctx.accounts.tick_array_2.to_account_info(), true, false)
        .account(ctx.accounts.oracle.to_account_info(), true, false)
        .invoke_signed(signer)
}

fn syrup_value(shares: u64, whirlpool: &AccountInfo) -> Result<u64> {
    if shares == 0 {
        return Ok(0);
    }
    let sqrt_price = read_whirlpool(whirlpool)?.sqrt_price;
    let price_x64 = mul_div_floor_u128(sqrt_price, sqrt_price, Q64)?;
    u128_to_u64(mul_div_floor_u128(u128::from(shares), price_x64, Q64)?)
}

fn read_whirlpool(whirlpool: &AccountInfo) -> Result<WhirlpoolState> {
    let data = whirlpool.try_borrow_data()?;
    let sqrt_price = read_u128(&data, WHIRLPOOL_SQRT_PRICE_OFFSET)?;
    require!(sqrt_price != 0, StandardError::AccountDecode);
    Ok(WhirlpoolState {
        sqrt_price,
        token_mint_a: read_pubkey(&data, WHIRLPOOL_TOKEN_MINT_A_OFFSET)?,
        token_mint_b: read_pubkey(&data, WHIRLPOOL_TOKEN_MINT_B_OFFSET)?,
    })
}

#[derive(Clone, Copy)]
struct WhirlpoolState {
    sqrt_price: u128,
    token_mint_a: Pubkey,
    token_mint_b: Pubkey,
}

#[derive(Accounts)]
pub struct StandardOp<'info> {
    #[account(
        init_if_needed,
        payer = owner,
        space = Position::SPACE,
        seeds = [seeds::POSITION, owner.key().as_ref(), base_mint.key().as_ref()],
        bump,
    )]
    pub position: Account<'info, Position>,
    /// CHECK: PDA signer validated by seeds.
    #[account(
        seeds = [seeds::POSITION_AUTHORITY, position.key().as_ref()],
        bump,
    )]
    pub position_authority: UncheckedAccount<'info>,
    #[account(address = USDC_MINT @ StandardError::MintMismatch)]
    pub base_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = owner,
        seeds = [seeds::VAULT, position.key().as_ref(), base_mint.key().as_ref()],
        bump,
        token::mint = base_mint,
        token::authority = position_authority,
    )]
    pub adapter_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        token::mint = base_mint,
        token::authority = owner,
    )]
    pub owner_token_account: Account<'info, TokenAccount>,
    /// CHECK: validated by the registry program and adapter id.
    pub registry_entry: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    #[account(address = SYRUP_USDC_MINT @ StandardError::MintMismatch)]
    pub syrup_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = owner,
        seeds = [seeds::RECEIPT_VAULT, position.key().as_ref(), syrup_mint.key().as_ref()],
        bump,
        token::mint = syrup_mint,
        token::authority = position_authority,
    )]
    pub syrup_vault: Account<'info, TokenAccount>,
    /// CHECK: exact Whirlpool checked in validate_common.
    #[account(mut)]
    pub whirlpool: UncheckedAccount<'info>,
    /// CHECK: exact token vault A checked in validate_common.
    #[account(mut)]
    pub token_vault_a: UncheckedAccount<'info>,
    /// CHECK: exact token vault B checked in validate_common.
    #[account(mut)]
    pub token_vault_b: UncheckedAccount<'info>,
    /// CHECK: Whirlpool tick array owned by Orca.
    #[account(mut)]
    pub tick_array_0: UncheckedAccount<'info>,
    /// CHECK: Whirlpool tick array owned by Orca.
    #[account(mut)]
    pub tick_array_1: UncheckedAccount<'info>,
    /// CHECK: Whirlpool tick array owned by Orca.
    #[account(mut)]
    pub tick_array_2: UncheckedAccount<'info>,
    /// CHECK: Whirlpool oracle PDA checked in validate_common.
    #[account(mut)]
    pub oracle: UncheckedAccount<'info>,
    /// CHECK: exact executable Orca Whirlpool program checked in validate_common.
    pub whirlpool_program: UncheckedAccount<'info>,
}

impl<'info> StandardOp<'info> {
    fn transfer_owner_to_vault(&self, amount: u64) -> Result<()> {
        token::transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.owner_token_account.to_account_info(),
                    to: self.adapter_vault.to_account_info(),
                    authority: self.owner.to_account_info(),
                },
            ),
            amount,
        )
    }

    fn transfer_vault_to_owner(&self, amount: u64) -> Result<()> {
        let position_key = self.position.key();
        let bump = [self.position.position_authority_bump];
        let signer: &[&[&[u8]]] = &[&[
            seeds::POSITION_AUTHORITY,
            position_key.as_ref(),
            bump.as_ref(),
        ]];
        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.adapter_vault.to_account_info(),
                    to: self.owner_token_account.to_account_info(),
                    authority: self.position_authority.to_account_info(),
                },
                signer,
            ),
            amount,
        )
    }
}

#[account]
#[derive(Default)]
pub struct Position {
    pub owner: Pubkey,
    pub base_mint: Pubkey,
    pub shares: u64,
    pub cached_value: u64,
    pub bump: u8,
    pub position_authority_bump: u8,
}

impl Position {
    pub const SPACE: usize = 8 + 32 + 32 + 8 + 8 + 1 + 1;

    pub fn initialize_if_needed(
        &mut self,
        owner: Pubkey,
        base_mint: Pubkey,
        bump: u8,
        position_authority_bump: u8,
    ) -> Result<()> {
        if self.owner == Pubkey::default() {
            self.owner = owner;
            self.base_mint = base_mint;
            self.bump = bump;
            self.position_authority_bump = position_authority_bump;
        } else {
            require_keys_eq!(self.owner, owner, StandardError::InvalidProtocolAccount);
            require_keys_eq!(self.base_mint, base_mint, StandardError::MintMismatch);
        }
        Ok(())
    }
}
