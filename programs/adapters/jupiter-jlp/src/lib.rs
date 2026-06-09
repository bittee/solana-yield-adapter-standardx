use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use syas_adapter_utils::{
    assert_enabled, assert_owner, assert_program, checked_add_u64, checked_sub_u64, mint_supply,
    mul_div_floor_u128, require_token_account, u128_to_u64, CpiBuilder, USDC_MINT,
};
use syas_interface::{seeds, Deposited, StandardError, ValueReported, Withdrawn};

declare_id!("4A1xkP49MszrDZE3Pzzq6a69tNprr3X799NitbAy7RmN");

const JUPITER_PERPS_ID: Pubkey = pubkey!("PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu");
const POOL: Pubkey = pubkey!("5BUwFW4nRbftYTDMbgxykoFWqWHPzahFSNAaaaJtVKsq");
const JLP_MINT: Pubkey = pubkey!("27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4");
const USDC_CUSTODY: Pubkey = pubkey!("G18jKKXQwBbrHeiK3C9MRXhkHsLHf7XgCSisykV46EZa");
const USDC_CUSTODY_TOKEN: Pubkey = pubkey!("WzWUoCmtVv7eqAbU3BfKPU3fhLP6CXR8NCJH78UK9VS");
const TRANSFER_AUTHORITY: Pubkey = pubkey!("AVzP2GeRmqGphJsMxWoqjpUifPpCret7LqWhD8NWQK49");
const PERPETUALS: Pubkey = pubkey!("H4ND9aYttUVLFmNypZqLjZ52FYiGvdEB45GmwNoKEjTj");
const EVENT_AUTHORITY: Pubkey = pubkey!("37hJBDnntwqhGbK7L6M1bLyvccj4u55CCUiLPdYkiqBN");
const ADD_LIQUIDITY2: [u8; 8] = [228, 162, 78, 28, 70, 219, 116, 115];
const REMOVE_LIQUIDITY2: [u8; 8] = [230, 215, 82, 127, 241, 101, 227, 146];

#[program]
pub mod jupiter_jlp_adapter {
    use super::*;

    pub fn deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, StandardOp<'info>>,
        amount: u64,
        min_position_out: u64,
    ) -> Result<()> {
        validate_common(&ctx)?;
        ctx.accounts.position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;

        ctx.accounts.transfer_owner_to_vault(amount)?;
        let jlp_before = ctx.accounts.jlp_vault.amount;
        invoke_add_liquidity(&ctx, amount, min_position_out)?;
        ctx.accounts.jlp_vault.reload()?;
        let position_out = checked_sub_u64(ctx.accounts.jlp_vault.amount, jlp_before)?;
        require!(
            position_out >= min_position_out,
            StandardError::SlippageExceeded
        );

        let new_shares = checked_add_u64(ctx.accounts.position.shares, position_out)?;
        let value = jlp_value(
            new_shares,
            &ctx.accounts.pool.to_account_info(),
            &ctx.accounts.jlp_mint.to_account_info(),
        )?;
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

    pub fn withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, StandardOp<'info>>,
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
        invoke_remove_liquidity(&ctx, position_amount, min_amount_out)?;
        ctx.accounts.adapter_vault.reload()?;
        let amount_out = checked_sub_u64(ctx.accounts.adapter_vault.amount, usdc_before)?;
        require!(
            amount_out >= min_amount_out,
            StandardError::SlippageExceeded
        );
        ctx.accounts.transfer_vault_to_owner(amount_out)?;

        let new_shares = checked_sub_u64(ctx.accounts.position.shares, position_amount)?;
        let value = jlp_value(
            new_shares,
            &ctx.accounts.pool.to_account_info(),
            &ctx.accounts.jlp_mint.to_account_info(),
        )?;
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
        let value = jlp_value(
            ctx.accounts.position.shares,
            &ctx.accounts.pool.to_account_info(),
            &ctx.accounts.jlp_mint.to_account_info(),
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
        &ctx.accounts.perps_program.to_account_info(),
        &JUPITER_PERPS_ID,
    )?;
    assert_owner(&ctx.accounts.pool.to_account_info(), &JUPITER_PERPS_ID)?;
    assert_owner(&ctx.accounts.custody.to_account_info(), &JUPITER_PERPS_ID)?;
    require_keys_eq!(
        ctx.accounts.jlp_mint.key(),
        JLP_MINT,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.pool.key(),
        POOL,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.custody.key(),
        USDC_CUSTODY,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.custody_token_account.key(),
        USDC_CUSTODY_TOKEN,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.transfer_authority.key(),
        TRANSFER_AUTHORITY,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.perpetuals.key(),
        PERPETUALS,
        StandardError::InvalidProtocolAccount
    );
    require_keys_eq!(
        ctx.accounts.event_authority.key(),
        EVENT_AUTHORITY,
        StandardError::InvalidProtocolAccount
    );
    require_token_account(
        &ctx.accounts.custody_token_account.to_account_info(),
        &USDC_MINT,
        &ctx.accounts.custody.key(),
        &ctx.accounts.token_program.key(),
    )?;
    Ok(())
}

fn invoke_add_liquidity<'info>(
    ctx: &Context<'_, '_, '_, 'info, StandardOp<'info>>,
    amount: u64,
    min_lp: u64,
) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];
    CpiBuilder::with_discriminator(ctx.accounts.perps_program.to_account_info(), ADD_LIQUIDITY2)
        .bytes(&amount.to_le_bytes())
        .bytes(&min_lp.to_le_bytes())
        .bytes(&[0])
        .account(
            ctx.accounts.position_authority.to_account_info(),
            false,
            true,
        )
        .account(ctx.accounts.adapter_vault.to_account_info(), true, false)
        .account(ctx.accounts.jlp_vault.to_account_info(), true, false)
        .account(
            ctx.accounts.transfer_authority.to_account_info(),
            false,
            false,
        )
        .account(ctx.accounts.perpetuals.to_account_info(), false, false)
        .account(ctx.accounts.pool.to_account_info(), true, false)
        .account(ctx.accounts.custody.to_account_info(), true, false)
        .account(
            ctx.accounts.custody_doves_price_account.to_account_info(),
            false,
            false,
        )
        .account(
            ctx.accounts.custody_pythnet_price_account.to_account_info(),
            false,
            false,
        )
        .account(
            ctx.accounts.custody_token_account.to_account_info(),
            true,
            false,
        )
        .account(ctx.accounts.jlp_mint.to_account_info(), true, false)
        .account(ctx.accounts.token_program.to_account_info(), false, false)
        .account(ctx.accounts.event_authority.to_account_info(), false, false)
        .account(ctx.accounts.perps_program.to_account_info(), false, false)
        .append_remaining(ctx.remaining_accounts)
        .invoke_signed(signer)
}

fn invoke_remove_liquidity<'info>(
    ctx: &Context<'_, '_, '_, 'info, StandardOp<'info>>,
    shares: u64,
    min_amount_out: u64,
) -> Result<()> {
    let position_key = ctx.accounts.position.key();
    let bump = [ctx.accounts.position.position_authority_bump];
    let signer: &[&[&[u8]]] = &[&[
        seeds::POSITION_AUTHORITY,
        position_key.as_ref(),
        bump.as_ref(),
    ]];
    CpiBuilder::with_discriminator(
        ctx.accounts.perps_program.to_account_info(),
        REMOVE_LIQUIDITY2,
    )
    .bytes(&shares.to_le_bytes())
    .bytes(&min_amount_out.to_le_bytes())
    .account(
        ctx.accounts.position_authority.to_account_info(),
        false,
        true,
    )
    .account(ctx.accounts.adapter_vault.to_account_info(), true, false)
    .account(ctx.accounts.jlp_vault.to_account_info(), true, false)
    .account(
        ctx.accounts.transfer_authority.to_account_info(),
        false,
        false,
    )
    .account(ctx.accounts.perpetuals.to_account_info(), false, false)
    .account(ctx.accounts.pool.to_account_info(), true, false)
    .account(ctx.accounts.custody.to_account_info(), true, false)
    .account(
        ctx.accounts.custody_doves_price_account.to_account_info(),
        false,
        false,
    )
    .account(
        ctx.accounts.custody_pythnet_price_account.to_account_info(),
        false,
        false,
    )
    .account(
        ctx.accounts.custody_token_account.to_account_info(),
        true,
        false,
    )
    .account(ctx.accounts.jlp_mint.to_account_info(), true, false)
    .account(ctx.accounts.token_program.to_account_info(), false, false)
    .account(ctx.accounts.event_authority.to_account_info(), false, false)
    .account(ctx.accounts.perps_program.to_account_info(), false, false)
    .append_remaining(ctx.remaining_accounts)
    .invoke_signed(signer)
}

fn jlp_value(shares: u64, pool: &AccountInfo, jlp_mint: &AccountInfo) -> Result<u64> {
    if shares == 0 {
        return Ok(0);
    }
    let aum_usd = pool_aum_usd(pool)?;
    let supply = u128::from(mint_supply(jlp_mint)?);
    require!(supply != 0, StandardError::AccountDecode);
    u128_to_u64(mul_div_floor_u128(u128::from(shares), aum_usd, supply)?)
}

fn pool_aum_usd(pool: &AccountInfo) -> Result<u128> {
    let data = pool.try_borrow_data()?;
    let mut cursor = 8usize;
    let name_len = read_u32(data.as_ref(), cursor)? as usize;
    cursor = cursor
        .checked_add(4)
        .and_then(|value| value.checked_add(name_len))
        .ok_or(StandardError::MathOverflow)?;
    let custody_count = read_u32(data.as_ref(), cursor)? as usize;
    cursor = cursor
        .checked_add(4)
        .and_then(|value| value.checked_add(custody_count.checked_mul(32)?))
        .ok_or(StandardError::MathOverflow)?;
    let bytes = data
        .get(cursor..cursor + 16)
        .ok_or(StandardError::AccountDecode)?;
    let array: [u8; 16] = bytes
        .try_into()
        .map_err(|_| error!(StandardError::AccountDecode))?;
    Ok(u128::from_le_bytes(array))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(StandardError::AccountDecode)?;
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| error!(StandardError::AccountDecode))?;
    Ok(u32::from_le_bytes(array))
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
    #[account(address = JLP_MINT @ StandardError::InvalidProtocolAccount)]
    pub jlp_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = owner,
        seeds = [seeds::RECEIPT_VAULT, position.key().as_ref(), jlp_mint.key().as_ref()],
        bump,
        token::mint = jlp_mint,
        token::authority = position_authority,
    )]
    pub jlp_vault: Account<'info, TokenAccount>,
    /// CHECK: exact Jupiter transfer authority checked in validate_common.
    pub transfer_authority: UncheckedAccount<'info>,
    /// CHECK: exact Jupiter perpetuals account checked in validate_common.
    pub perpetuals: UncheckedAccount<'info>,
    /// CHECK: exact Jupiter pool checked in validate_common and parsed for value.
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: exact USDC custody checked in validate_common.
    #[account(mut)]
    pub custody: UncheckedAccount<'info>,
    /// CHECK: forwarded Jupiter price account.
    pub custody_doves_price_account: UncheckedAccount<'info>,
    /// CHECK: forwarded Jupiter price account.
    pub custody_pythnet_price_account: UncheckedAccount<'info>,
    /// CHECK: exact custody token account checked in validate_common.
    #[account(mut)]
    pub custody_token_account: UncheckedAccount<'info>,
    /// CHECK: exact Jupiter event authority checked in validate_common.
    pub event_authority: UncheckedAccount<'info>,
    /// CHECK: exact executable Jupiter Perps program checked in validate_common.
    pub perps_program: UncheckedAccount<'info>,
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
