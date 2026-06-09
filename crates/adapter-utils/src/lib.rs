use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
};
use syas_interface::{AdapterStatus, StandardError};

pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

pub fn assert_enabled(
    registry_entry: &AccountInfo,
    adapter_program: &Pubkey,
    base_mint: &Pubkey,
) -> Result<registry::AdapterEntry> {
    let entry = registry::load_adapter_entry(registry_entry, adapter_program)?;
    require!(
        entry.status == AdapterStatus::Enabled,
        StandardError::AdapterDisabled
    );
    require_keys_eq!(entry.base_mint, *base_mint, StandardError::MintMismatch);
    Ok(entry)
}

pub fn assert_program(info: &AccountInfo, expected: &Pubkey) -> Result<()> {
    require_keys_eq!(*info.key, *expected, StandardError::InvalidProtocolAccount);
    require!(info.executable, StandardError::InvalidProtocolAccount);
    Ok(())
}

pub fn assert_owner(info: &AccountInfo, expected: &Pubkey) -> Result<()> {
    require_keys_eq!(
        *info.owner,
        *expected,
        StandardError::InvalidProtocolAccount
    );
    Ok(())
}

pub fn require_pda(expected: &Pubkey, seeds: &[&[u8]], program_id: &Pubkey) -> Result<u8> {
    let (derived, bump) = Pubkey::find_program_address(seeds, program_id);
    require_keys_eq!(derived, *expected, StandardError::InvalidPda);
    Ok(bump)
}

pub struct CpiBuilder<'info> {
    program: AccountInfo<'info>,
    accounts: Vec<AccountInfo<'info>>,
    metas: Vec<AccountMeta>,
    data: Vec<u8>,
}

impl<'info> CpiBuilder<'info> {
    pub fn new(program: AccountInfo<'info>, name: &str) -> Self {
        Self {
            program,
            accounts: Vec::new(),
            metas: Vec::new(),
            data: syas_interface::anchor_discriminator(name).to_vec(),
        }
    }

    pub fn with_discriminator(program: AccountInfo<'info>, discriminator: [u8; 8]) -> Self {
        Self {
            program,
            accounts: Vec::new(),
            metas: Vec::new(),
            data: discriminator.to_vec(),
        }
    }

    pub fn arg<T: AnchorSerialize>(mut self, value: &T) -> Result<Self> {
        value
            .serialize(&mut self.data)
            .map_err(|_| error!(StandardError::InstructionEncode))?;
        Ok(self)
    }

    pub fn bytes(mut self, value: &[u8]) -> Self {
        self.data.extend_from_slice(value);
        self
    }

    pub fn account(mut self, info: AccountInfo<'info>, writable: bool, signer: bool) -> Self {
        if writable {
            self.metas.push(AccountMeta::new(*info.key, signer));
        } else {
            self.metas
                .push(AccountMeta::new_readonly(*info.key, signer));
        }
        self.accounts.push(info);
        self
    }

    pub fn append_remaining(mut self, accounts: &[AccountInfo<'info>]) -> Self {
        for account in accounts {
            self = self.account(account.clone(), account.is_writable, account.is_signer);
        }
        self
    }

    pub fn invoke(self) -> Result<()> {
        let ix = Instruction {
            program_id: *self.program.key,
            accounts: self.metas,
            data: self.data,
        };
        let mut infos = self.accounts;
        infos.push(self.program);
        invoke(&ix, &infos)?;
        Ok(())
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<()> {
        let ix = Instruction {
            program_id: *self.program.key,
            accounts: self.metas,
            data: self.data,
        };
        let mut infos = self.accounts;
        infos.push(self.program);
        invoke_signed(&ix, &infos, signer_seeds)?;
        Ok(())
    }
}

pub fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset + 32)
        .ok_or(StandardError::AccountDecode)?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| error!(StandardError::AccountDecode))?;
    Ok(Pubkey::new_from_array(array))
}

pub fn read_u8(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or(error!(StandardError::AccountDecode))
}

pub fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or(StandardError::AccountDecode)?;
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| error!(StandardError::AccountDecode))?;
    Ok(u64::from_le_bytes(array))
}

pub fn read_i64(data: &[u8], offset: usize) -> Result<i64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or(StandardError::AccountDecode)?;
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| error!(StandardError::AccountDecode))?;
    Ok(i64::from_le_bytes(array))
}

pub fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    let bytes = data
        .get(offset..offset + 16)
        .ok_or(StandardError::AccountDecode)?;
    let array: [u8; 16] = bytes
        .try_into()
        .map_err(|_| error!(StandardError::AccountDecode))?;
    Ok(u128::from_le_bytes(array))
}

pub fn read_i128(data: &[u8], offset: usize) -> Result<i128> {
    let bytes = data
        .get(offset..offset + 16)
        .ok_or(StandardError::AccountDecode)?;
    let array: [u8; 16] = bytes
        .try_into()
        .map_err(|_| error!(StandardError::AccountDecode))?;
    Ok(i128::from_le_bytes(array))
}

pub fn token_mint(info: &AccountInfo) -> Result<Pubkey> {
    let data = info.try_borrow_data()?;
    read_pubkey(&data, 0)
}

pub fn token_owner(info: &AccountInfo) -> Result<Pubkey> {
    let data = info.try_borrow_data()?;
    read_pubkey(&data, 32)
}

pub fn token_amount(info: &AccountInfo) -> Result<u64> {
    let data = info.try_borrow_data()?;
    read_u64(&data, 64)
}

pub fn mint_supply(info: &AccountInfo) -> Result<u64> {
    let data = info.try_borrow_data()?;
    read_u64(&data, 36)
}

pub fn mint_decimals(info: &AccountInfo) -> Result<u8> {
    let data = info.try_borrow_data()?;
    read_u8(&data, 44)
}

pub fn require_token_account(
    account: &AccountInfo,
    mint: &Pubkey,
    authority: &Pubkey,
    token_program: &Pubkey,
) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        *token_program,
        StandardError::InvalidTokenAccount
    );
    require_keys_eq!(
        token_mint(account)?,
        *mint,
        StandardError::InvalidTokenAccount
    );
    require_keys_eq!(
        token_owner(account)?,
        *authority,
        StandardError::InvalidTokenAccount
    );
    Ok(())
}

pub fn checked_add_u64(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(error!(StandardError::MathOverflow))
}

pub fn checked_sub_u64(a: u64, b: u64) -> Result<u64> {
    a.checked_sub(b).ok_or(error!(StandardError::MathOverflow))
}

pub fn u128_to_u64(value: u128) -> Result<u64> {
    u64::try_from(value).map_err(|_| error!(StandardError::MathOverflow))
}

pub fn mul_div_floor_u128(a: u128, b: u128, denominator: u128) -> Result<u128> {
    require!(denominator != 0, StandardError::MathOverflow);
    if let Some(product) = a.checked_mul(b) {
        return Ok(product / denominator);
    }

    let product = product_limbs(a, b);
    let mut quotient = 0u128;
    let mut remainder = 0u128;

    for bit in (0..256usize).rev() {
        require!(remainder <= (u128::MAX >> 1), StandardError::MathOverflow);
        remainder = (remainder << 1) | u128::from(bit_at(&product, bit));
        if remainder >= denominator {
            remainder -= denominator;
            require!(bit < 128, StandardError::MathOverflow);
            quotient |= 1u128 << bit;
        }
    }

    Ok(quotient)
}

fn product_limbs(a: u128, b: u128) -> [u64; 4] {
    let a_limbs = [a as u64, (a >> 64) as u64];
    let b_limbs = [b as u64, (b >> 64) as u64];
    let mut out = [0u64; 4];

    for (i, a_limb) in a_limbs.iter().enumerate() {
        for (j, b_limb) in b_limbs.iter().enumerate() {
            let product = u128::from(*a_limb) * u128::from(*b_limb);
            add_to_limb(&mut out, i + j, product as u64);
            add_to_limb(&mut out, i + j + 1, (product >> 64) as u64);
        }
    }

    out
}

fn add_to_limb(limbs: &mut [u64; 4], start: usize, value: u64) {
    let mut index = start;
    let mut carry = u128::from(value);
    while carry != 0 && index < limbs.len() {
        let sum = u128::from(limbs[index]) + (carry & u128::from(u64::MAX));
        limbs[index] = sum as u64;
        carry = (carry >> 64) + (sum >> 64);
        index += 1;
    }
}

fn bit_at(limbs: &[u64; 4], bit: usize) -> u8 {
    let limb = bit / 64;
    let shift = bit % 64;
    ((limbs[limb] >> shift) & 1) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_div_handles_small_values() {
        assert_eq!(mul_div_floor_u128(7, 9, 4).unwrap(), 15);
    }

    #[test]
    fn mul_div_handles_wide_values_when_result_fits() {
        let result = mul_div_floor_u128(u128::MAX / 3, 9, 3).unwrap();
        assert_eq!(result, u128::MAX);
    }
}
