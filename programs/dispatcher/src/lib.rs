use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::AccountMeta, program::invoke};
use syas_interface::{AdapterAction, AdapterRouted, AdapterStatus, StandardError};

declare_id!("HGj3chDufhrN3LZE31jjK9Kv4ETzmzkxHwxDQKCzUrk");

#[program]
pub mod dispatcher {
    use super::*;

    pub fn route_deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, Route<'info>>,
        amount: u64,
        min_position_out: u64,
    ) -> Result<()> {
        gate(&ctx)?;
        invoke_adapter(
            &ctx,
            syas_interface::encode_deposit(amount, min_position_out),
        )?;
        forward_adapter_return(&ctx)?;
        emit!(AdapterRouted {
            adapter: ctx.accounts.adapter_program.key(),
            action: AdapterAction::Deposit,
        });
        Ok(())
    }

    pub fn route_withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, Route<'info>>,
        position_amount: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        gate(&ctx)?;
        invoke_adapter(
            &ctx,
            syas_interface::encode_withdraw(position_amount, min_amount_out),
        )?;
        forward_adapter_return(&ctx)?;
        emit!(AdapterRouted {
            adapter: ctx.accounts.adapter_program.key(),
            action: AdapterAction::Withdraw,
        });
        Ok(())
    }

    pub fn route_current_value<'info>(ctx: Context<'_, '_, '_, 'info, Route<'info>>) -> Result<()> {
        gate(&ctx)?;
        invoke_adapter(&ctx, syas_interface::encode_current_value())?;
        forward_adapter_return(&ctx)?;
        emit!(AdapterRouted {
            adapter: ctx.accounts.adapter_program.key(),
            action: AdapterAction::CurrentValue,
        });
        Ok(())
    }
}

fn forward_adapter_return(ctx: &Context<Route>) -> Result<()> {
    let value = syas_interface::read_return_u64(&ctx.accounts.adapter_program.key())?;
    syas_interface::set_return_u64(value);
    Ok(())
}

fn gate(ctx: &Context<Route>) -> Result<()> {
    let entry = registry::load_adapter_entry(
        &ctx.accounts.registry_entry.to_account_info(),
        &ctx.accounts.adapter_program.key(),
    )?;
    require!(
        entry.status == AdapterStatus::Enabled,
        StandardError::AdapterDisabled
    );
    require_keys_eq!(
        entry.base_mint,
        ctx.accounts.base_mint.key(),
        StandardError::MintMismatch
    );
    Ok(())
}

fn invoke_adapter<'info>(
    ctx: &Context<'_, '_, '_, 'info, Route<'info>>,
    data: Vec<u8>,
) -> Result<()> {
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.adapter_program.key(),
        accounts: forward_metas(ctx),
        data,
    };
    invoke(&ix, &forward_infos(ctx))?;
    Ok(())
}

fn forward_metas(ctx: &Context<Route>) -> Vec<AccountMeta> {
    let accounts = &ctx.accounts;
    let mut metas = vec![
        AccountMeta::new(accounts.position.key(), false),
        AccountMeta::new_readonly(accounts.position_authority.key(), false),
        AccountMeta::new_readonly(accounts.base_mint.key(), false),
        AccountMeta::new(accounts.adapter_vault.key(), false),
        AccountMeta::new(accounts.owner.key(), true),
        AccountMeta::new(accounts.owner_token_account.key(), false),
        AccountMeta::new_readonly(accounts.registry_entry.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
    ];
    metas.extend(ctx.remaining_accounts.iter().map(|account| {
        if account.is_writable {
            AccountMeta::new(*account.key, account.is_signer)
        } else {
            AccountMeta::new_readonly(*account.key, account.is_signer)
        }
    }));
    metas
}

fn forward_infos<'info>(ctx: &Context<'_, '_, '_, 'info, Route<'info>>) -> Vec<AccountInfo<'info>> {
    let accounts = &ctx.accounts;
    let mut infos = vec![
        accounts.position.to_account_info(),
        accounts.position_authority.to_account_info(),
        accounts.base_mint.to_account_info(),
        accounts.adapter_vault.to_account_info(),
        accounts.owner.to_account_info(),
        accounts.owner_token_account.to_account_info(),
        accounts.registry_entry.to_account_info(),
        accounts.token_program.to_account_info(),
        accounts.system_program.to_account_info(),
    ];
    infos.extend(ctx.remaining_accounts.iter().cloned());
    infos.push(accounts.adapter_program.to_account_info());
    infos
}

#[derive(Accounts)]
pub struct Route<'info> {
    /// CHECK: adapter-owned position account; the adapter validates seeds and contents.
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    /// CHECK: adapter-owned position authority PDA.
    pub position_authority: UncheckedAccount<'info>,
    /// CHECK: compared against the registry entry.
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: adapter-owned token vault or placeholder in tests.
    #[account(mut)]
    pub adapter_vault: UncheckedAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: owner token account; the adapter validates mint and authority.
    #[account(mut)]
    pub owner_token_account: UncheckedAccount<'info>,
    /// CHECK: validated with syas_registry::load_adapter_entry.
    pub registry_entry: UncheckedAccount<'info>,
    /// CHECK: forwarded to adapter.
    pub token_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: registry binds this executable account to registry_entry.
    #[account(executable)]
    pub adapter_program: UncheckedAccount<'info>,
}
