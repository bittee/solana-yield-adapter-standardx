use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use syas_adapter_utils::{
    assert_enabled, assert_owner, assert_program, checked_sub_u64, mul_div_floor_u128, read_i128,
    read_pubkey, require_pda, u128_to_u64, CpiBuilder, USDC_MINT,
};
use syas_interface::{seeds, Deposited, StandardError, ValueReported, Withdrawn};

declare_id!("CEy21HbuzU6K9WLueUwXtjeiLicVhyMLPtkMHXEzccXu");

const MARGINFI_ID: Pubkey = pubkey!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");
const MARGINFI_GROUP: Pubkey = pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8");
const USDC_BANK: Pubkey = pubkey!("2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB");
const USDC_LIQUIDITY_VAULT: Pubkey = pubkey!("7jaiZR5Sk8hdYN9MxTpczTcwbWpb5WEoxSANuUwveuat");
const USDC_LIQUIDITY_VAULT_AUTHORITY: Pubkey =
    pubkey!("3uxNepDbmkDNq6JhRja5Z8QwbTrfmkKP8AKZV5chYDGG");
const USDC_ORACLE: Pubkey = pubkey!("Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX");

const BANK_DISC: [u8; 8] = [142, 49, 166, 242, 50, 66, 97, 188];
const MARGINFI_ACCOUNT_DISC: [u8; 8] = [67, 178, 130, 109, 126, 114, 28, 42];
const BANK_MINT_OFFSET: usize = 8;
const BANK_GROUP_OFFSET: usize = 41;
const BANK_ASSET_SHARE_VALUE_OFFSET: usize = 80;
const BANK_LIQUIDITY_VAULT_OFFSET: usize = 112;
const BANK_MIN_LEN: usize = 146;
const ACCOUNT_GROUP_OFFSET: usize = 8;
const ACCOUNT_AUTHORITY_OFFSET: usize = 40;
const ACCOUNT_BALANCES_OFFSET: usize = 72;
const BALANCE_SIZE: usize = 104;
const BALANCE_BANK_OFFSET: usize = 1;
const BALANCE_ASSET_SHARES_OFFSET: usize = 40;
const MAX_BALANCES: usize = 16;
const I80F48_PRODUCT_SCALE: u128 = 1u128 << 96;

#[program]
pub mod marginfi_usdc_adapter {
    use super::*;

    pub fn deposit(ctx: Context<StandardOp>, amount: u64, min_position_out: u64) -> Result<()> {
        validate_common(&ctx)?;
        ctx.accounts.position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;
        initialize_marginfi_account_if_needed(&ctx)?;

        let shares_before = marginfi_asset_shares(
            &ctx.accounts.marginfi_account.to_account_info(),
            &ctx.accounts.bank.key(),
        )?;
        ctx.accounts.transfer_owner_to_vault(amount)?;
        invoke_lending_deposit(&ctx, amount)?;
        let shares_after = marginfi_asset_shares(
            &ctx.accounts.marginfi_account.to_account_info(),
            &ctx.accounts.bank.key(),
        )?;
        let position_out = shares_after
            .checked_sub(shares_before)
            .ok_or(StandardError::MathOverflow)?;
        let position_out_u64 = u128_to_u64(position_out)?;
        require!(
            position_out_u64 >= min_position_out,
            StandardError::SlippageExceeded
        );

        let live_value = position_value(
            &ctx.accounts.marginfi_account.to_account_info(),
            &ctx.accounts.bank.to_account_info(),
        )?;
        let position = &mut ctx.accounts.position;
        position.shares = u128_to_u64(shares_after)?;
        position.cached_value = live_value;

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
        require!(position_amount != 0, StandardError::NothingToWithdraw);
        require!(
            position_amount <= ctx.accounts.position.shares,
            StandardError::SlippageExceeded
        );

        let bank = read_bank(&ctx.accounts.bank.to_account_info())?;
        let shares_before = marginfi_asset_shares(
            &ctx.accounts.marginfi_account.to_account_info(),
            &ctx.accounts.bank.key(),
        )?;
        require!(shares_before != 0, StandardError::NothingToWithdraw);
        let withdraw_all = u128::from(position_amount) >= shares_before;
        let requested_amount =
            shares_to_tokens(u128::from(position_amount), bank.asset_share_value)?;

        let usdc_before = ctx.accounts.adapter_vault.amount;
        invoke_lending_withdraw(&ctx, requested_amount, withdraw_all)?;
        ctx.accounts.adapter_vault.reload()?;
        let amount_out = checked_sub_u64(ctx.accounts.adapter_vault.amount, usdc_before)?;
        require!(
            amount_out >= min_amount_out,
            StandardError::SlippageExceeded
        );
        ctx.accounts.transfer_vault_to_owner(amount_out)?;

        let shares_after = marginfi_asset_shares(
            &ctx.accounts.marginfi_account.to_account_info(),
            &ctx.accounts.bank.key(),
        )?;
        let position = &mut ctx.accounts.position;
        position.shares = u128_to_u64(shares_after)?;
        position.cached_value = shares_to_tokens(shares_after, bank.asset_share_value)?;

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
        let shares = if ctx
            .accounts
            .marginfi_account
            .to_account_info()
            .data_is_empty()
        {
            0
        } else {
            marginfi_asset_shares(
                &ctx.accounts.marginfi_account.to_account_info(),
                &ctx.accounts.bank.key(),
            )?
        };
        let value = if shares == 0 {
            0
        } else {
            let bank = read_bank(&ctx.accounts.bank.to_account_info())?;
            shares_to_tokens(shares, bank.asset_share_value)?
        };
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
        &ctx.accounts.marginfi_program.to_account_info(),
        &MARGINFI_ID,
    )?;
    assert_owner(&ctx.accounts.marginfi_group.to_account_info(), &MARGINFI_ID)?;
    assert_owner(&ctx.accounts.bank.to_account_info(), &MARGINFI_ID)?;
    require_keys_eq!(
        ctx.accounts.marginfi_group.key(),
        MARGINFI_GROUP,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.bank.key(),
        USDC_BANK,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.liquidity_vault.key(),
        USDC_LIQUIDITY_VAULT,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.liquidity_vault_authority.key(),
        USDC_LIQUIDITY_VAULT_AUTHORITY,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.oracle.key(),
        USDC_ORACLE,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.instructions_sysvar.key(),
        sysvar::instructions::ID,
        StandardError::InvalidProtocolAccount
    );
    require_pda(
        &ctx.accounts.marginfi_account.key(),
        &[
            b"marginfi_account",
            ctx.accounts.marginfi_group.key().as_ref(),
            ctx.accounts.position_authority.key().as_ref(),
            &0u16.to_le_bytes(),
            &0u16.to_le_bytes(),
        ],
        &MARGINFI_ID,
    )?;

    let bank = read_bank(&ctx.accounts.bank.to_account_info())?;
    require_keys_eq!(bank.mint, USDC_MINT, StandardError::MintMismatch);
    require_keys_eq!(
        bank.group,
        MARGINFI_GROUP,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        bank.liquidity_vault,
        ctx.accounts.liquidity_vault.key(),
        StandardError::InvalidProtocolAccount
    );

    let marginfi_account = ctx.accounts.marginfi_account.to_account_info();
    if !marginfi_account.data_is_empty() {
        validate_marginfi_account(&marginfi_account, &ctx.accounts.position_authority.key())?;
    }
    Ok(())
}

fn initialize_marginfi_account_if_needed(ctx: &Context<StandardOp>) -> Result<()> {
    if !ctx
        .accounts
        .marginfi_account
        .to_account_info()
        .data_is_empty()
    {
        return Ok(());
    }

    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];

    CpiBuilder::new(
        ctx.accounts.marginfi_program.to_account_info(),
        "marginfi_account_initialize_pda",
    )
    .arg(&0u16)?
    .arg(&None::<u16>)?
    .account(ctx.accounts.marginfi_group.to_account_info(), false, false)
    .account(ctx.accounts.marginfi_account.to_account_info(), true, false)
    .account(
        ctx.accounts.position_authority.to_account_info(),
        false,
        true,
    )
    .account(ctx.accounts.owner.to_account_info(), true, true)
    .account(
        ctx.accounts.instructions_sysvar.to_account_info(),
        false,
        false,
    )
    .account(ctx.accounts.system_program.to_account_info(), false, false)
    .invoke_signed(signer)
}

fn invoke_lending_deposit(ctx: &Context<StandardOp>, amount: u64) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];

    CpiBuilder::new(
        ctx.accounts.marginfi_program.to_account_info(),
        "lending_account_deposit",
    )
    .arg(&amount)?
    .arg(&None::<bool>)?
    .account(ctx.accounts.marginfi_group.to_account_info(), false, false)
    .account(ctx.accounts.marginfi_account.to_account_info(), true, false)
    .account(
        ctx.accounts.position_authority.to_account_info(),
        false,
        true,
    )
    .account(ctx.accounts.bank.to_account_info(), true, false)
    .account(ctx.accounts.adapter_vault.to_account_info(), true, false)
    .account(ctx.accounts.liquidity_vault.to_account_info(), true, false)
    .account(ctx.accounts.token_program.to_account_info(), false, false)
    .invoke_signed(signer)
}

fn invoke_lending_withdraw(
    ctx: &Context<StandardOp>,
    amount: u64,
    withdraw_all: bool,
) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];

    CpiBuilder::new(
        ctx.accounts.marginfi_program.to_account_info(),
        "lending_account_withdraw",
    )
    .arg(&amount)?
    .arg(&Some(withdraw_all))?
    .account(ctx.accounts.marginfi_group.to_account_info(), false, false)
    .account(ctx.accounts.marginfi_account.to_account_info(), true, false)
    .account(
        ctx.accounts.position_authority.to_account_info(),
        false,
        true,
    )
    .account(ctx.accounts.bank.to_account_info(), true, false)
    .account(ctx.accounts.adapter_vault.to_account_info(), true, false)
    .account(
        ctx.accounts.liquidity_vault_authority.to_account_info(),
        false,
        false,
    )
    .account(ctx.accounts.liquidity_vault.to_account_info(), true, false)
    .account(ctx.accounts.token_program.to_account_info(), false, false)
    .account(ctx.accounts.bank.to_account_info(), false, false)
    .account(ctx.accounts.oracle.to_account_info(), false, false)
    .invoke_signed(signer)
}

fn read_bank(bank: &AccountInfo) -> Result<BankView> {
    let data = bank.try_borrow_data()?;
    require!(data.len() >= BANK_MIN_LEN, StandardError::AccountDecode);
    require!(data[0..8] == BANK_DISC, StandardError::AccountDecode);
    let asset_share_value = positive_i80f48(&data, BANK_ASSET_SHARE_VALUE_OFFSET)?;
    Ok(BankView {
        mint: read_pubkey(&data, BANK_MINT_OFFSET)?,
        group: read_pubkey(&data, BANK_GROUP_OFFSET)?,
        asset_share_value,
        liquidity_vault: read_pubkey(&data, BANK_LIQUIDITY_VAULT_OFFSET)?,
    })
}

fn validate_marginfi_account(account: &AccountInfo, expected_authority: &Pubkey) -> Result<()> {
    assert_owner(account, &MARGINFI_ID)?;
    let data = account.try_borrow_data()?;
    require!(
        data.len() >= ACCOUNT_BALANCES_OFFSET,
        StandardError::AccountDecode
    );
    require!(
        data[0..8] == MARGINFI_ACCOUNT_DISC,
        StandardError::AccountDecode
    );
    require_keys_eq!(
        read_pubkey(&data, ACCOUNT_GROUP_OFFSET)?,
        MARGINFI_GROUP,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        read_pubkey(&data, ACCOUNT_AUTHORITY_OFFSET)?,
        *expected_authority,
        StandardError::InvalidProtocolAccount
    );
    Ok(())
}

fn marginfi_asset_shares(account: &AccountInfo, bank: &Pubkey) -> Result<u128> {
    validate_marginfi_account(
        account,
        &read_pubkey(&account.try_borrow_data()?, ACCOUNT_AUTHORITY_OFFSET)?,
    )?;
    let data = account.try_borrow_data()?;
    for index in 0..MAX_BALANCES {
        let base = ACCOUNT_BALANCES_OFFSET + index * BALANCE_SIZE;
        let slot = data
            .get(base..base + BALANCE_SIZE)
            .ok_or(StandardError::AccountDecode)?;
        if slot[0] == 0 {
            continue;
        }
        let slot_bank = read_pubkey(slot, BALANCE_BANK_OFFSET)?;
        if slot_bank == *bank {
            return positive_i80f48(slot, BALANCE_ASSET_SHARES_OFFSET);
        }
    }
    Ok(0)
}

fn positive_i80f48(data: &[u8], offset: usize) -> Result<u128> {
    let value = read_i128(data, offset)?;
    require!(value >= 0, StandardError::AccountDecode);
    Ok(value as u128)
}

fn position_value(marginfi_account: &AccountInfo, bank: &AccountInfo) -> Result<u64> {
    let bank_view = read_bank(bank)?;
    let shares = marginfi_asset_shares(marginfi_account, &USDC_BANK)?;
    shares_to_tokens(shares, bank_view.asset_share_value)
}

fn shares_to_tokens(asset_shares_bits: u128, asset_share_value_bits: u128) -> Result<u64> {
    u128_to_u64(mul_div_floor_u128(
        asset_shares_bits,
        asset_share_value_bits,
        I80F48_PRODUCT_SCALE,
    )?)
}

#[derive(Clone, Copy)]
struct BankView {
    mint: Pubkey,
    group: Pubkey,
    asset_share_value: u128,
    liquidity_vault: Pubkey,
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
    /// CHECK: deterministic MarginFi PDA validated in validate_common.
    #[account(mut)]
    pub marginfi_account: UncheckedAccount<'info>,
    /// CHECK: exact group and owner checked in validate_common.
    pub marginfi_group: UncheckedAccount<'info>,
    /// CHECK: exact bank, owner, and layout checked in validate_common.
    #[account(mut)]
    pub bank: UncheckedAccount<'info>,
    /// CHECK: exact vault authority checked in validate_common.
    pub liquidity_vault_authority: UncheckedAccount<'info>,
    /// CHECK: exact bank liquidity vault checked in validate_common.
    #[account(mut)]
    pub liquidity_vault: UncheckedAccount<'info>,
    /// CHECK: exact oracle checked in validate_common.
    pub oracle: UncheckedAccount<'info>,
    /// CHECK: exact instruction sysvar checked in validate_common.
    pub instructions_sysvar: UncheckedAccount<'info>,
    /// CHECK: exact executable MarginFi program checked in validate_common.
    pub marginfi_program: UncheckedAccount<'info>,
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
