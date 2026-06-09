use anchor_lang::prelude::*;
use syas_interface::{seeds, AdapterStatus, Protocol, MAX_RISK_TIER, STANDARD_VERSION};

declare_id!("BWTrd2xVhy2T12CLrr9ffy3StQtYdJtRWGejWLVtCd2p");

#[program]
pub mod registry {
    use super::*;

    pub fn initialize_registry(ctx: Context<InitializeRegistry>, governance: Pubkey) -> Result<()> {
        require_keys_neq!(
            governance,
            Pubkey::default(),
            RegistryError::InvalidGovernance
        );
        let registry = &mut ctx.accounts.registry;
        registry.version = STANDARD_VERSION;
        registry.governance = governance;
        registry.pending_governance = Pubkey::default();
        registry.adapter_count = 0;
        registry.bump = ctx.bumps.registry;
        emit!(RegistryInitialized { governance });
        Ok(())
    }

    pub fn register_adapter(
        ctx: Context<RegisterAdapter>,
        adapter_program: Pubkey,
        base_mint: Pubkey,
        protocol: Protocol,
        risk_tier: u8,
    ) -> Result<()> {
        require_keys_neq!(
            adapter_program,
            Pubkey::default(),
            RegistryError::InvalidAdapterProgram
        );
        require_keys_neq!(base_mint, Pubkey::default(), RegistryError::InvalidBaseMint);
        require!(risk_tier <= MAX_RISK_TIER, RegistryError::InvalidRiskTier);

        let entry = &mut ctx.accounts.adapter_entry;
        entry.version = STANDARD_VERSION;
        entry.adapter_program = adapter_program;
        entry.base_mint = base_mint;
        entry.protocol = protocol;
        entry.status = AdapterStatus::Disabled;
        entry.risk_tier = risk_tier;
        entry.bump = ctx.bumps.adapter_entry;

        let registry = &mut ctx.accounts.registry;
        registry.adapter_count = registry
            .adapter_count
            .checked_add(1)
            .ok_or(RegistryError::MathOverflow)?;

        emit!(AdapterRegistered {
            adapter_program,
            base_mint,
            protocol,
            risk_tier,
        });
        Ok(())
    }

    pub fn enable_adapter(ctx: Context<GovernedEntry>) -> Result<()> {
        ctx.accounts.adapter_entry.status = AdapterStatus::Enabled;
        emit!(AdapterStatusChanged {
            adapter_program: ctx.accounts.adapter_entry.adapter_program,
            status: AdapterStatus::Enabled,
        });
        Ok(())
    }

    pub fn disable_adapter(ctx: Context<GovernedEntry>) -> Result<()> {
        ctx.accounts.adapter_entry.status = AdapterStatus::Disabled;
        emit!(AdapterStatusChanged {
            adapter_program: ctx.accounts.adapter_entry.adapter_program,
            status: AdapterStatus::Disabled,
        });
        Ok(())
    }

    pub fn propose_governance(
        ctx: Context<GovernedRegistry>,
        new_governance: Pubkey,
    ) -> Result<()> {
        require_keys_neq!(
            new_governance,
            Pubkey::default(),
            RegistryError::InvalidGovernance
        );
        ctx.accounts.registry.pending_governance = new_governance;
        emit!(GovernanceProposed { new_governance });
        Ok(())
    }

    pub fn accept_governance(ctx: Context<AcceptGovernance>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        require_keys_eq!(
            ctx.accounts.new_governance.key(),
            registry.pending_governance,
            RegistryError::Unauthorized
        );
        registry.governance = registry.pending_governance;
        registry.pending_governance = Pubkey::default();
        emit!(GovernanceAccepted {
            governance: registry.governance,
        });
        Ok(())
    }
}

pub fn load_adapter_entry(info: &AccountInfo, adapter_program: &Pubkey) -> Result<AdapterEntry> {
    require_keys_eq!(*info.owner, crate::ID, RegistryError::InvalidEntryOwner);
    let data = info.try_borrow_data()?;
    let mut bytes: &[u8] = &data;
    let entry = AdapterEntry::try_deserialize(&mut bytes)?;
    let expected = Pubkey::create_program_address(
        &[
            seeds::ADAPTER_ENTRY,
            adapter_program.as_ref(),
            &[entry.bump],
        ],
        &crate::ID,
    )
    .map_err(|_| error!(RegistryError::InvalidEntryPda))?;
    require_keys_eq!(*info.key, expected, RegistryError::InvalidEntryPda);
    require_keys_eq!(
        entry.adapter_program,
        *adapter_program,
        RegistryError::AdapterProgramMismatch
    );
    Ok(entry)
}

#[derive(Accounts)]
pub struct InitializeRegistry<'info> {
    #[account(
        init,
        payer = payer,
        space = Registry::SPACE,
        seeds = [seeds::REGISTRY],
        bump,
    )]
    pub registry: Account<'info, Registry>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(adapter_program: Pubkey)]
pub struct RegisterAdapter<'info> {
    #[account(
        mut,
        seeds = [seeds::REGISTRY],
        bump = registry.bump,
        has_one = governance @ RegistryError::Unauthorized,
    )]
    pub registry: Account<'info, Registry>,
    #[account(
        init,
        payer = governance,
        space = AdapterEntry::SPACE,
        seeds = [seeds::ADAPTER_ENTRY, adapter_program.as_ref()],
        bump,
    )]
    pub adapter_entry: Account<'info, AdapterEntry>,
    #[account(mut)]
    pub governance: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GovernedEntry<'info> {
    #[account(
        seeds = [seeds::REGISTRY],
        bump = registry.bump,
        has_one = governance @ RegistryError::Unauthorized,
    )]
    pub registry: Account<'info, Registry>,
    #[account(
        mut,
        seeds = [seeds::ADAPTER_ENTRY, adapter_entry.adapter_program.as_ref()],
        bump = adapter_entry.bump,
    )]
    pub adapter_entry: Account<'info, AdapterEntry>,
    pub governance: Signer<'info>,
}

#[derive(Accounts)]
pub struct GovernedRegistry<'info> {
    #[account(
        mut,
        seeds = [seeds::REGISTRY],
        bump = registry.bump,
        has_one = governance @ RegistryError::Unauthorized,
    )]
    pub registry: Account<'info, Registry>,
    pub governance: Signer<'info>,
}

#[derive(Accounts)]
pub struct AcceptGovernance<'info> {
    #[account(
        mut,
        seeds = [seeds::REGISTRY],
        bump = registry.bump,
    )]
    pub registry: Account<'info, Registry>,
    pub new_governance: Signer<'info>,
}

#[account]
pub struct Registry {
    pub version: u16,
    pub governance: Pubkey,
    pub pending_governance: Pubkey,
    pub adapter_count: u32,
    pub bump: u8,
}

impl Registry {
    pub const SPACE: usize = 8 + 2 + 32 + 32 + 4 + 1;
}

#[account]
pub struct AdapterEntry {
    pub version: u16,
    pub adapter_program: Pubkey,
    pub base_mint: Pubkey,
    pub protocol: Protocol,
    pub status: AdapterStatus,
    pub risk_tier: u8,
    pub bump: u8,
}

impl AdapterEntry {
    pub const SPACE: usize = 8 + 2 + 32 + 32 + Protocol::LEN + AdapterStatus::LEN + 1 + 1;
}

#[event]
pub struct RegistryInitialized {
    pub governance: Pubkey,
}

#[event]
pub struct AdapterRegistered {
    pub adapter_program: Pubkey,
    pub base_mint: Pubkey,
    pub protocol: Protocol,
    pub risk_tier: u8,
}

#[event]
pub struct AdapterStatusChanged {
    pub adapter_program: Pubkey,
    pub status: AdapterStatus,
}

#[event]
pub struct GovernanceProposed {
    pub new_governance: Pubkey,
}

#[event]
pub struct GovernanceAccepted {
    pub governance: Pubkey,
}

#[error_code]
pub enum RegistryError {
    #[msg("governance signer is not authorized")]
    Unauthorized,
    #[msg("governance cannot be the default public key")]
    InvalidGovernance,
    #[msg("adapter program cannot be the default public key")]
    InvalidAdapterProgram,
    #[msg("base mint cannot be the default public key")]
    InvalidBaseMint,
    #[msg("risk tier is outside the standard range")]
    InvalidRiskTier,
    #[msg("registry arithmetic overflow")]
    MathOverflow,
    #[msg("registry entry is not owned by the registry program")]
    InvalidEntryOwner,
    #[msg("registry entry PDA does not match the adapter program")]
    InvalidEntryPda,
    #[msg("registry entry program id does not match the requested adapter")]
    AdapterProgramMismatch,
}
