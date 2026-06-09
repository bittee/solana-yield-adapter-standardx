use anchor_lang::prelude::*;
use syas_interface::{seeds, AdapterStatus, Deposited, StandardError, ValueReported, Withdrawn};

declare_id!("8p7m5zPd9S52CXnEzNu3JBVqraUSwyktF4JWYaaphmEr");

#[program]
pub mod mock_adapter {
    use super::*;

    pub fn deposit(ctx: Context<StandardOp>, amount: u64, min_position_out: u64) -> Result<()> {
        assert_enabled(
            &ctx.accounts.registry_entry.to_account_info(),
            &ctx.accounts.base_mint.key(),
        )?;
        let position = &mut ctx.accounts.position;
        position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;

        let position_out = amount;
        require!(
            position_out >= min_position_out,
            StandardError::SlippageExceeded
        );
        position.shares = position
            .shares
            .checked_add(position_out)
            .ok_or(StandardError::MathOverflow)?;
        position.cached_value = position.shares;
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
        assert_enabled(
            &ctx.accounts.registry_entry.to_account_info(),
            &ctx.accounts.base_mint.key(),
        )?;
        let position = &mut ctx.accounts.position;
        position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;
        require!(
            position_amount <= position.shares,
            StandardError::SlippageExceeded
        );
        let amount_out = position_amount;
        require!(
            amount_out >= min_amount_out,
            StandardError::SlippageExceeded
        );
        position.shares = position
            .shares
            .checked_sub(position_amount)
            .ok_or(StandardError::MathOverflow)?;
        position.cached_value = position.shares;
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
        assert_enabled(
            &ctx.accounts.registry_entry.to_account_info(),
            &ctx.accounts.base_mint.key(),
        )?;
        let position = &mut ctx.accounts.position;
        position.initialize_if_needed(
            ctx.accounts.owner.key(),
            ctx.accounts.base_mint.key(),
            ctx.bumps.position,
            ctx.bumps.position_authority,
        )?;
        let value = position.shares;
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

fn assert_enabled(registry_entry: &AccountInfo, base_mint: &Pubkey) -> Result<()> {
    let entry = registry::load_adapter_entry(registry_entry, &crate::ID)?;
    require!(
        entry.status == AdapterStatus::Enabled,
        StandardError::AdapterDisabled
    );
    require_keys_eq!(entry.base_mint, *base_mint, StandardError::MintMismatch);
    Ok(())
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
    /// CHECK: PDA signer for real adapters; the mock only validates derivation.
    #[account(
        seeds = [seeds::POSITION_AUTHORITY, position.key().as_ref()],
        bump,
    )]
    pub position_authority: UncheckedAccount<'info>,
    /// CHECK: mock adapter supports any registered base mint.
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: unused by the mock adapter.
    #[account(mut)]
    pub adapter_vault: UncheckedAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: unused by the mock adapter.
    #[account(mut)]
    pub owner_token_account: UncheckedAccount<'info>,
    /// CHECK: validated by assert_enabled.
    pub registry_entry: UncheckedAccount<'info>,
    /// CHECK: unused by the mock adapter.
    pub token_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
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
