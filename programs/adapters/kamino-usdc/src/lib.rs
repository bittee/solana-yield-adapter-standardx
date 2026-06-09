use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use syas_adapter_utils::{
    assert_enabled, assert_owner, assert_program, checked_add_u64, checked_sub_u64,
    mul_div_floor_u128, read_u128, read_u64, require_pda, u128_to_u64, CpiBuilder, USDC_MINT,
};
use syas_interface::{seeds, Deposited, StandardError, ValueReported, Withdrawn};

declare_id!("E1bTG9vyE27xVay1oZrZck6idNJZjgotaeFia1P7q9Vb");

const KLEND_ID: Pubkey = pubkey!("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");
const RESERVE: Pubkey = pubkey!("D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59");
const LENDING_MARKET: Pubkey = pubkey!("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF");
const RESERVE_LIQUIDITY_SUPPLY: Pubkey = pubkey!("Bgq7trRgVMeq33yt235zM2onQ4bRDBsY5EWiTetF4qw6");
const RESERVE_COLLATERAL_MINT: Pubkey = pubkey!("B8V6WVjPxW1UGwVDfxH2d2r8SyT4cqn7dQRK6XneVa7D");

const R_AVAILABLE_AMOUNT: usize = 224;
const R_BORROWED_AMOUNT_SF: usize = 232;
const R_ACC_PROTOCOL_FEES_SF: usize = 344;
const R_ACC_REFERRER_FEES_SF: usize = 360;
const R_PENDING_REFERRER_FEES_SF: usize = 376;
const R_COLLATERAL_SUPPLY: usize = 2592;
const FRACTIONAL_BITS: u32 = 60;

#[program]
pub mod kamino_usdc_adapter {
    use super::*;

    pub fn deposit(ctx: Context<StandardOp>, amount: u64, min_position_out: u64) -> Result<()> {
        validate_common(&ctx)?;
        let position_key = ctx.accounts.position.key();
        ctx.accounts.position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;

        ctx.accounts.transfer_owner_to_vault(amount)?;

        let collateral_before = ctx.accounts.collateral_vault.amount;
        invoke_deposit_reserve_liquidity(&ctx, amount)?;
        ctx.accounts.collateral_vault.reload()?;
        let position_out =
            checked_sub_u64(ctx.accounts.collateral_vault.amount, collateral_before)?;
        require!(
            position_out >= min_position_out,
            StandardError::SlippageExceeded
        );

        let new_shares = checked_add_u64(ctx.accounts.position.shares, position_out)?;
        let value = ctoken_value(new_shares, &ctx.accounts.reserve.to_account_info())?;
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
        msg!("kamino position {}", position_key);
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
        invoke_redeem_reserve_collateral(&ctx, position_amount)?;
        ctx.accounts.adapter_vault.reload()?;
        let amount_out = checked_sub_u64(ctx.accounts.adapter_vault.amount, usdc_before)?;
        require!(
            amount_out >= min_amount_out,
            StandardError::SlippageExceeded
        );

        ctx.accounts.transfer_vault_to_owner(amount_out)?;

        let new_shares = checked_sub_u64(ctx.accounts.position.shares, position_amount)?;
        let value = ctoken_value(new_shares, &ctx.accounts.reserve.to_account_info())?;
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
        let value = ctoken_value(
            ctx.accounts.position.shares,
            &ctx.accounts.reserve.to_account_info(),
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
    assert_program(&ctx.accounts.klend_program.to_account_info(), &KLEND_ID)?;
    assert_owner(&ctx.accounts.reserve.to_account_info(), &KLEND_ID)?;
    assert_owner(&ctx.accounts.lending_market.to_account_info(), &KLEND_ID)?;
    require_keys_eq!(
        ctx.accounts.reserve.key(),
        RESERVE,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.lending_market.key(),
        LENDING_MARKET,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.reserve_liquidity_supply.key(),
        RESERVE_LIQUIDITY_SUPPLY,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.collateral_mint.key(),
        RESERVE_COLLATERAL_MINT,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.instruction_sysvar.key(),
        sysvar::instructions::ID,
        StandardError::InvalidProtocolAccount
    );
    require_pda(
        &ctx.accounts.lending_market_authority.key(),
        &[b"lma", ctx.accounts.lending_market.key().as_ref()],
        &KLEND_ID,
    )?;
    Ok(())
}

fn invoke_deposit_reserve_liquidity(ctx: &Context<StandardOp>, amount: u64) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];

    CpiBuilder::new(
        ctx.accounts.klend_program.to_account_info(),
        "deposit_reserve_liquidity",
    )
    .arg(&amount)?
    .account(
        ctx.accounts.position_authority.to_account_info(),
        false,
        true,
    )
    .account(ctx.accounts.reserve.to_account_info(), true, false)
    .account(ctx.accounts.lending_market.to_account_info(), false, false)
    .account(
        ctx.accounts.lending_market_authority.to_account_info(),
        false,
        false,
    )
    .account(ctx.accounts.base_mint.to_account_info(), false, false)
    .account(
        ctx.accounts.reserve_liquidity_supply.to_account_info(),
        true,
        false,
    )
    .account(ctx.accounts.collateral_mint.to_account_info(), true, false)
    .account(ctx.accounts.adapter_vault.to_account_info(), true, false)
    .account(ctx.accounts.collateral_vault.to_account_info(), true, false)
    .account(ctx.accounts.token_program.to_account_info(), false, false)
    .account(ctx.accounts.token_program.to_account_info(), false, false)
    .account(
        ctx.accounts.instruction_sysvar.to_account_info(),
        false,
        false,
    )
    .invoke_signed(signer)
}

fn invoke_redeem_reserve_collateral(ctx: &Context<StandardOp>, shares: u64) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];

    CpiBuilder::new(
        ctx.accounts.klend_program.to_account_info(),
        "redeem_reserve_collateral",
    )
    .arg(&shares)?
    .account(
        ctx.accounts.position_authority.to_account_info(),
        false,
        true,
    )
    .account(ctx.accounts.lending_market.to_account_info(), false, false)
    .account(ctx.accounts.reserve.to_account_info(), true, false)
    .account(
        ctx.accounts.lending_market_authority.to_account_info(),
        false,
        false,
    )
    .account(ctx.accounts.base_mint.to_account_info(), false, false)
    .account(ctx.accounts.collateral_mint.to_account_info(), true, false)
    .account(
        ctx.accounts.reserve_liquidity_supply.to_account_info(),
        true,
        false,
    )
    .account(ctx.accounts.collateral_vault.to_account_info(), true, false)
    .account(ctx.accounts.adapter_vault.to_account_info(), true, false)
    .account(ctx.accounts.token_program.to_account_info(), false, false)
    .account(ctx.accounts.token_program.to_account_info(), false, false)
    .account(
        ctx.accounts.instruction_sysvar.to_account_info(),
        false,
        false,
    )
    .invoke_signed(signer)
}

fn ctoken_value(shares: u64, reserve: &AccountInfo) -> Result<u64> {
    if shares == 0 {
        return Ok(0);
    }
    let data = reserve.try_borrow_data()?;
    let available = u128::from(read_u64(&data, R_AVAILABLE_AMOUNT)?);
    let borrowed_sf = read_u128(&data, R_BORROWED_AMOUNT_SF)?;
    let protocol_fees_sf = read_u128(&data, R_ACC_PROTOCOL_FEES_SF)?;
    let referrer_fees_sf = read_u128(&data, R_ACC_REFERRER_FEES_SF)?;
    let pending_referrer_fees_sf = read_u128(&data, R_PENDING_REFERRER_FEES_SF)?;
    let collateral_supply = u128::from(read_u64(&data, R_COLLATERAL_SUPPLY)?);
    require!(collateral_supply != 0, StandardError::AccountDecode);

    let available_sf = available
        .checked_shl(FRACTIONAL_BITS)
        .ok_or(StandardError::MathOverflow)?;
    let fee_total_sf = protocol_fees_sf
        .checked_add(referrer_fees_sf)
        .and_then(|value| value.checked_add(pending_referrer_fees_sf))
        .ok_or(StandardError::MathOverflow)?;
    let total_liquidity_sf = available_sf
        .checked_add(borrowed_sf)
        .and_then(|value| value.checked_sub(fee_total_sf))
        .ok_or(StandardError::MathOverflow)?;
    let total_liquidity = total_liquidity_sf >> FRACTIONAL_BITS;
    u128_to_u64(mul_div_floor_u128(
        u128::from(shares),
        total_liquidity,
        collateral_supply,
    )?)
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
    /// CHECK: validated against the registry program and adapter id.
    pub registry_entry: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    #[account(address = RESERVE_COLLATERAL_MINT @ StandardError::InvalidProtocolAccount)]
    pub collateral_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = owner,
        seeds = [seeds::RECEIPT_VAULT, position.key().as_ref(), collateral_mint.key().as_ref()],
        bump,
        token::mint = collateral_mint,
        token::authority = position_authority,
    )]
    pub collateral_vault: Account<'info, TokenAccount>,
    /// CHECK: exact KLend reserve checked in validate_common.
    #[account(mut)]
    pub reserve: UncheckedAccount<'info>,
    /// CHECK: exact KLend lending market checked in validate_common.
    pub lending_market: UncheckedAccount<'info>,
    /// CHECK: KLend PDA checked in validate_common.
    pub lending_market_authority: UncheckedAccount<'info>,
    /// CHECK: exact KLend reserve liquidity supply vault checked in validate_common.
    #[account(mut)]
    pub reserve_liquidity_supply: UncheckedAccount<'info>,
    /// CHECK: exact instruction sysvar checked in validate_common.
    pub instruction_sysvar: UncheckedAccount<'info>,
    /// CHECK: exact executable KLend program checked in validate_common.
    pub klend_program: UncheckedAccount<'info>,
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
        let seeds: &[&[&[u8]]] = &[&[
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
                seeds,
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
