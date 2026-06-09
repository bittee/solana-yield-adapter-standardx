use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use syas_adapter_utils::{
    assert_enabled, assert_owner, assert_program, checked_sub_u64, mul_div_floor_u128, read_i64,
    read_pubkey, read_u128, require_pda, require_token_account, token_amount, u128_to_u64,
    CpiBuilder, USDC_MINT,
};
use syas_interface::{
    seeds, Deposited, StandardError, ValueReported, WithdrawalPending, Withdrawn,
};

declare_id!("BemVwXxgBf71TXQQrWJH61SR1oodD7tPzX8LFymeB6tM");

const DRIFT_ID: Pubkey = pubkey!("dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH");
const MARKET_INDEX: u16 = 0;
const SPOT_MARKET_IF_TOTAL_SHARES: usize = 336;
const SPOT_MARKET_UNSTAKING_PERIOD: usize = 384;
const IF_STAKE_AUTHORITY: usize = 8;
const IF_STAKE_IF_SHARES: usize = 40;

#[program]
pub mod drift_if_adapter {
    use super::*;

    pub fn deposit(ctx: Context<StandardOp>, amount: u64, min_position_out: u64) -> Result<()> {
        validate_common(&ctx)?;
        ctx.accounts.position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;
        initialize_drift_accounts_if_needed(&ctx)?;

        let shares_before = if_shares(&ctx.accounts.insurance_fund_stake.to_account_info())?;
        ctx.accounts.transfer_owner_to_vault(amount)?;
        invoke_add_stake(&ctx, amount)?;
        let shares_after = if_shares(&ctx.accounts.insurance_fund_stake.to_account_info())?;
        let position_out = shares_after
            .checked_sub(shares_before)
            .ok_or(StandardError::MathOverflow)?;
        let position_out_u64 = u128_to_u64(position_out)?;
        require!(
            position_out_u64 >= min_position_out,
            StandardError::SlippageExceeded
        );

        let value = if_value(
            shares_after,
            &ctx.accounts.spot_market.to_account_info(),
            &ctx.accounts.insurance_fund_vault.to_account_info(),
        )?;
        let position = &mut ctx.accounts.position;
        position.shares = u128_to_u64(shares_after)?;
        position.cached_value = value;

        syas_interface::set_return_u64(position_out_u64);
        emit!(Deposited {
            owner: position.owner,
            adapter: crate::ID,
            amount_in: amount,
            position_out: position_out_u64,
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
        if ctx.accounts.position.pending_shares != 0 {
            return settle_pending_withdrawal(ctx, min_amount_out);
        }

        require!(position_amount != 0, StandardError::NothingToWithdraw);
        require!(
            position_amount <= ctx.accounts.position.shares,
            StandardError::SlippageExceeded
        );
        invoke_request_remove(&ctx, position_amount)?;
        let now = Clock::get()?.unix_timestamp;
        let unstaking_period = read_i64(
            &ctx.accounts
                .spot_market
                .to_account_info()
                .try_borrow_data()?,
            SPOT_MARKET_UNSTAKING_PERIOD,
        )?;
        let unlock_ts = now
            .checked_add(unstaking_period)
            .ok_or(StandardError::MathOverflow)?;
        let position = &mut ctx.accounts.position;
        position.pending_shares = position_amount;
        position.pending_min_amount_out = min_amount_out;
        position.unlock_ts = unlock_ts;
        syas_interface::set_return_u64(0);
        emit!(WithdrawalPending {
            owner: position.owner,
            adapter: crate::ID,
            position_in: position_amount,
            unlock_ts,
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
        let shares = if ctx
            .accounts
            .insurance_fund_stake
            .to_account_info()
            .data_is_empty()
        {
            0
        } else {
            if_shares(&ctx.accounts.insurance_fund_stake.to_account_info())?
        };
        let value = if_value(
            shares,
            &ctx.accounts.spot_market.to_account_info(),
            &ctx.accounts.insurance_fund_vault.to_account_info(),
        )?;
        let position = &mut ctx.accounts.position;
        position.shares = u128_to_u64(shares)?;
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

fn settle_pending_withdrawal(ctx: Context<StandardOp>, min_amount_out: u64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    require!(
        now >= ctx.accounts.position.unlock_ts,
        StandardError::WithdrawalLocked
    );
    let usdc_before = ctx.accounts.adapter_vault.amount;
    invoke_remove_stake(&ctx)?;
    ctx.accounts.adapter_vault.reload()?;
    let amount_out = checked_sub_u64(ctx.accounts.adapter_vault.amount, usdc_before)?;
    let required_out = min_amount_out.max(ctx.accounts.position.pending_min_amount_out);
    require!(amount_out >= required_out, StandardError::SlippageExceeded);
    ctx.accounts.transfer_vault_to_owner(amount_out)?;

    let shares_after = if_shares(&ctx.accounts.insurance_fund_stake.to_account_info())?;
    let value = if_value(
        shares_after,
        &ctx.accounts.spot_market.to_account_info(),
        &ctx.accounts.insurance_fund_vault.to_account_info(),
    )?;
    let position_in = ctx.accounts.position.pending_shares;
    let position = &mut ctx.accounts.position;
    position.shares = u128_to_u64(shares_after)?;
    position.cached_value = value;
    position.pending_shares = 0;
    position.pending_min_amount_out = 0;
    position.unlock_ts = 0;

    syas_interface::set_return_u64(amount_out);
    emit!(Withdrawn {
        owner: position.owner,
        adapter: crate::ID,
        position_in,
        amount_out,
    });
    Ok(())
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
    assert_program(&ctx.accounts.drift_program.to_account_info(), &DRIFT_ID)?;
    assert_owner(&ctx.accounts.drift_state.to_account_info(), &DRIFT_ID)?;
    assert_owner(&ctx.accounts.spot_market.to_account_info(), &DRIFT_ID)?;

    let index = MARKET_INDEX.to_le_bytes();
    require_pda(
        &ctx.accounts.drift_state.key(),
        &[b"drift_state"],
        &DRIFT_ID,
    )?;
    require_pda(
        &ctx.accounts.drift_signer.key(),
        &[b"drift_signer"],
        &DRIFT_ID,
    )?;
    require_pda(
        &ctx.accounts.spot_market.key(),
        &[b"spot_market", index.as_ref()],
        &DRIFT_ID,
    )?;
    require_pda(
        &ctx.accounts.spot_market_vault.key(),
        &[b"spot_market_vault", index.as_ref()],
        &DRIFT_ID,
    )?;
    require_pda(
        &ctx.accounts.insurance_fund_vault.key(),
        &[b"insurance_fund_vault", index.as_ref()],
        &DRIFT_ID,
    )?;
    require_pda(
        &ctx.accounts.user_stats.key(),
        &[
            b"user_stats",
            ctx.accounts.position_authority.key().as_ref(),
        ],
        &DRIFT_ID,
    )?;
    require_pda(
        &ctx.accounts.insurance_fund_stake.key(),
        &[
            b"insurance_fund_stake",
            ctx.accounts.position_authority.key().as_ref(),
            index.as_ref(),
        ],
        &DRIFT_ID,
    )?;
    require_token_account(
        &ctx.accounts.spot_market_vault.to_account_info(),
        &USDC_MINT,
        &ctx.accounts.drift_signer.key(),
        &ctx.accounts.token_program.key(),
    )?;
    require_token_account(
        &ctx.accounts.insurance_fund_vault.to_account_info(),
        &USDC_MINT,
        &ctx.accounts.drift_signer.key(),
        &ctx.accounts.token_program.key(),
    )?;
    let insurance_fund_stake = ctx.accounts.insurance_fund_stake.to_account_info();
    if !insurance_fund_stake.data_is_empty() {
        let data = insurance_fund_stake.try_borrow_data()?;
        require_keys_eq!(
            read_pubkey(&data, IF_STAKE_AUTHORITY)?,
            ctx.accounts.position_authority.key(),
            StandardError::InvalidProtocolAccount
        );
    }
    Ok(())
}

fn initialize_drift_accounts_if_needed(ctx: &Context<StandardOp>) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];

    if ctx.accounts.user_stats.to_account_info().data_is_empty() {
        CpiBuilder::new(
            ctx.accounts.drift_program.to_account_info(),
            "initialize_user_stats",
        )
        .account(ctx.accounts.user_stats.to_account_info(), true, false)
        .account(ctx.accounts.drift_state.to_account_info(), true, false)
        .account(
            ctx.accounts.position_authority.to_account_info(),
            false,
            false,
        )
        .account(ctx.accounts.owner.to_account_info(), true, true)
        .account(ctx.accounts.rent.to_account_info(), false, false)
        .account(ctx.accounts.system_program.to_account_info(), false, false)
        .invoke_signed(signer)?;
    }

    if ctx
        .accounts
        .insurance_fund_stake
        .to_account_info()
        .data_is_empty()
    {
        CpiBuilder::new(
            ctx.accounts.drift_program.to_account_info(),
            "initialize_insurance_fund_stake",
        )
        .arg(&MARKET_INDEX)?
        .account(ctx.accounts.spot_market.to_account_info(), false, false)
        .account(
            ctx.accounts.insurance_fund_stake.to_account_info(),
            true,
            false,
        )
        .account(ctx.accounts.user_stats.to_account_info(), true, false)
        .account(ctx.accounts.drift_state.to_account_info(), false, false)
        .account(
            ctx.accounts.position_authority.to_account_info(),
            false,
            true,
        )
        .account(ctx.accounts.owner.to_account_info(), true, true)
        .account(ctx.accounts.rent.to_account_info(), false, false)
        .account(ctx.accounts.system_program.to_account_info(), false, false)
        .invoke_signed(signer)?;
    }
    Ok(())
}

fn invoke_add_stake(ctx: &Context<StandardOp>, amount: u64) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];
    CpiBuilder::new(
        ctx.accounts.drift_program.to_account_info(),
        "add_insurance_fund_stake",
    )
    .arg(&MARKET_INDEX)?
    .arg(&amount)?
    .account(ctx.accounts.drift_state.to_account_info(), false, false)
    .account(ctx.accounts.spot_market.to_account_info(), true, false)
    .account(
        ctx.accounts.insurance_fund_stake.to_account_info(),
        true,
        false,
    )
    .account(ctx.accounts.user_stats.to_account_info(), true, false)
    .account(
        ctx.accounts.position_authority.to_account_info(),
        false,
        true,
    )
    .account(
        ctx.accounts.spot_market_vault.to_account_info(),
        true,
        false,
    )
    .account(
        ctx.accounts.insurance_fund_vault.to_account_info(),
        true,
        false,
    )
    .account(ctx.accounts.drift_signer.to_account_info(), false, false)
    .account(ctx.accounts.adapter_vault.to_account_info(), true, false)
    .account(ctx.accounts.token_program.to_account_info(), false, false)
    .invoke_signed(signer)
}

fn invoke_request_remove(ctx: &Context<StandardOp>, shares: u64) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];
    CpiBuilder::new(
        ctx.accounts.drift_program.to_account_info(),
        "request_remove_insurance_fund_stake",
    )
    .arg(&MARKET_INDEX)?
    .arg(&shares)?
    .account(ctx.accounts.spot_market.to_account_info(), true, false)
    .account(
        ctx.accounts.insurance_fund_stake.to_account_info(),
        true,
        false,
    )
    .account(ctx.accounts.user_stats.to_account_info(), true, false)
    .account(
        ctx.accounts.position_authority.to_account_info(),
        false,
        true,
    )
    .account(
        ctx.accounts.insurance_fund_vault.to_account_info(),
        true,
        false,
    )
    .invoke_signed(signer)
}

fn invoke_remove_stake(ctx: &Context<StandardOp>) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];
    CpiBuilder::new(
        ctx.accounts.drift_program.to_account_info(),
        "remove_insurance_fund_stake",
    )
    .arg(&MARKET_INDEX)?
    .account(ctx.accounts.drift_state.to_account_info(), false, false)
    .account(ctx.accounts.spot_market.to_account_info(), true, false)
    .account(
        ctx.accounts.insurance_fund_stake.to_account_info(),
        true,
        false,
    )
    .account(ctx.accounts.user_stats.to_account_info(), true, false)
    .account(
        ctx.accounts.position_authority.to_account_info(),
        false,
        true,
    )
    .account(
        ctx.accounts.insurance_fund_vault.to_account_info(),
        true,
        false,
    )
    .account(ctx.accounts.drift_signer.to_account_info(), false, false)
    .account(ctx.accounts.adapter_vault.to_account_info(), true, false)
    .account(ctx.accounts.token_program.to_account_info(), false, false)
    .invoke_signed(signer)
}

fn if_shares(if_stake: &AccountInfo) -> Result<u128> {
    let data = if_stake.try_borrow_data()?;
    read_u128(&data, IF_STAKE_IF_SHARES)
}

fn if_value(shares: u128, spot_market: &AccountInfo, if_vault: &AccountInfo) -> Result<u64> {
    if shares == 0 {
        return Ok(0);
    }
    let market_data = spot_market.try_borrow_data()?;
    let total_shares = read_u128(&market_data, SPOT_MARKET_IF_TOTAL_SHARES)?;
    require!(total_shares != 0, StandardError::AccountDecode);
    let vault_amount = u128::from(token_amount(if_vault)?);
    u128_to_u64(mul_div_floor_u128(shares, vault_amount, total_shares)?)
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
    /// CHECK: Drift user stats PDA.
    #[account(mut)]
    pub user_stats: UncheckedAccount<'info>,
    /// CHECK: Drift IF stake PDA.
    #[account(mut)]
    pub insurance_fund_stake: UncheckedAccount<'info>,
    /// CHECK: Drift state PDA.
    #[account(mut)]
    pub drift_state: UncheckedAccount<'info>,
    /// CHECK: Drift USDC spot market PDA.
    #[account(mut)]
    pub spot_market: UncheckedAccount<'info>,
    /// CHECK: Drift USDC spot-market vault.
    #[account(mut)]
    pub spot_market_vault: UncheckedAccount<'info>,
    /// CHECK: Drift USDC insurance fund vault.
    #[account(mut)]
    pub insurance_fund_vault: UncheckedAccount<'info>,
    /// CHECK: Drift signer PDA.
    pub drift_signer: UncheckedAccount<'info>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: exact executable Drift program checked in validate_common.
    pub drift_program: UncheckedAccount<'info>,
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
    pub pending_shares: u64,
    pub pending_min_amount_out: u64,
    pub unlock_ts: i64,
    pub bump: u8,
    pub position_authority_bump: u8,
}

impl Position {
    pub const SPACE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 1 + 1;

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
