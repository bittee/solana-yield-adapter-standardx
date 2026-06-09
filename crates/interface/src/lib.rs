use anchor_lang::prelude::*;
use anchor_lang::solana_program::{hash::hash, program};

pub mod seeds {
    pub const REGISTRY: &[u8] = b"registry";
    pub const ADAPTER_ENTRY: &[u8] = b"adapter_entry";
    pub const POSITION: &[u8] = b"position";
    pub const POSITION_AUTHORITY: &[u8] = b"position_authority";
    pub const VAULT: &[u8] = b"vault";
    pub const RECEIPT_VAULT: &[u8] = b"receipt_vault";
    pub const TICKET: &[u8] = b"ticket";
}

pub mod ix {
    pub const DEPOSIT: &str = "deposit";
    pub const WITHDRAW: &str = "withdraw";
    pub const CURRENT_VALUE: &str = "current_value";
}

pub const STANDARD_VERSION: u16 = 1;
pub const MAX_RISK_TIER: u8 = 5;

pub fn anchor_discriminator(name: &str) -> [u8; 8] {
    let digest = hash(format!("global:{name}").as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest.to_bytes()[..8]);
    out
}

pub fn encode_deposit(amount: u64, min_position_out: u64) -> Vec<u8> {
    encode_two_u64(ix::DEPOSIT, amount, min_position_out)
}

pub fn encode_withdraw(position_amount: u64, min_amount_out: u64) -> Vec<u8> {
    encode_two_u64(ix::WITHDRAW, position_amount, min_amount_out)
}

pub fn encode_current_value() -> Vec<u8> {
    anchor_discriminator(ix::CURRENT_VALUE).to_vec()
}

fn encode_two_u64(name: &str, first: u64, second: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&anchor_discriminator(name));
    data.extend_from_slice(&first.to_le_bytes());
    data.extend_from_slice(&second.to_le_bytes());
    data
}

pub fn set_return_u64(value: u64) {
    program::set_return_data(&value.to_le_bytes());
}

pub fn read_return_u64(expected_program: &Pubkey) -> Result<u64> {
    let (program_id, data) = program::get_return_data().ok_or(StandardError::MissingReturnData)?;
    require_keys_eq!(
        program_id,
        *expected_program,
        StandardError::UnexpectedReturnProgram
    );
    require!(data.len() == 8, StandardError::InvalidReturnData);
    let bytes: [u8; 8] = data
        .as_slice()
        .try_into()
        .map_err(|_| error!(StandardError::InvalidReturnData))?;
    Ok(u64::from_le_bytes(bytes))
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AdapterStatus {
    Disabled,
    Enabled,
}

impl AdapterStatus {
    pub const LEN: usize = 1;

    pub fn accepts_user_flow(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Protocol {
    Mock,
    KaminoUsdc,
    MarginfiUsdc,
    JupiterJlp,
    MapleSyrup,
    DriftInsuranceFund,
}

impl Protocol {
    pub const LEN: usize = 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AdapterAction {
    Deposit,
    Withdraw,
    CurrentValue,
}

#[event]
pub struct AdapterRouted {
    pub adapter: Pubkey,
    pub action: AdapterAction,
}

#[event]
pub struct Deposited {
    pub owner: Pubkey,
    pub adapter: Pubkey,
    pub amount_in: u64,
    pub position_out: u64,
}

#[event]
pub struct Withdrawn {
    pub owner: Pubkey,
    pub adapter: Pubkey,
    pub position_in: u64,
    pub amount_out: u64,
}

#[event]
pub struct WithdrawalPending {
    pub owner: Pubkey,
    pub adapter: Pubkey,
    pub position_in: u64,
    pub unlock_ts: i64,
}

#[event]
pub struct ValueReported {
    pub owner: Pubkey,
    pub adapter: Pubkey,
    pub value: u64,
}

#[error_code]
pub enum StandardError {
    #[msg("adapter is not enabled")]
    AdapterDisabled,
    #[msg("base mint does not match the registered adapter")]
    MintMismatch,
    #[msg("remaining accounts are invalid for this adapter")]
    InvalidRemainingAccounts,
    #[msg("protocol account is invalid")]
    InvalidProtocolAccount,
    #[msg("token account is invalid")]
    InvalidTokenAccount,
    #[msg("program derived address does not match expected seeds")]
    InvalidPda,
    #[msg("slippage limit was not satisfied")]
    SlippageExceeded,
    #[msg("withdrawal has been requested and is waiting for protocol unlock")]
    WithdrawalPending,
    #[msg("withdrawal is still locked by the protocol")]
    WithdrawalLocked,
    #[msg("oracle or market data is stale")]
    OracleStale,
    #[msg("arithmetic overflow")]
    MathOverflow,
    #[msg("position has no redeemable balance")]
    NothingToWithdraw,
    #[msg("adapter did not return data")]
    MissingReturnData,
    #[msg("adapter returned malformed data")]
    InvalidReturnData,
    #[msg("return data came from an unexpected program")]
    UnexpectedReturnProgram,
    #[msg("account data could not be decoded")]
    AccountDecode,
    #[msg("instruction data could not be encoded")]
    InstructionEncode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_encoders_have_expected_lengths() {
        assert_eq!(encode_deposit(1, 2).len(), 24);
        assert_eq!(encode_withdraw(1, 2).len(), 24);
        assert_eq!(encode_current_value().len(), 8);
    }

    #[test]
    fn discriminators_match_anchor_method_names() {
        assert_eq!(
            anchor_discriminator("deposit"),
            [242, 35, 198, 137, 82, 225, 242, 182]
        );
        assert_eq!(
            anchor_discriminator("withdraw"),
            [183, 18, 70, 156, 148, 109, 161, 34]
        );
        assert_eq!(
            anchor_discriminator("current_value"),
            [232, 199, 167, 206, 247, 56, 234, 20]
        );
    }
}
