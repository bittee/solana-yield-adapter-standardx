# Protocol Research Notes

Checked on: 2026-06-09

This file records the source-backed protocol decisions for the five reference
adapters. The reference repositories in `REPOS.md` were used for account-order
lessons and fork failure reports. Current protocol descriptions were checked
against official docs or primary repositories where available.

## Sources Checked

- Kamino developer docs: https://kamino.com/docs/build/developers/borrow
- Kamino KLend repository: https://github.com/Kamino-Finance/klend
- marginfi v2 docs: https://docs.marginfi.com/mfi-v2
- marginfi v2 repository: https://github.com/mrgnlabs/marginfi-v2
- Jupiter JLP docs: https://docs.jup.ag/user-docs/trade/perps-and-jlp/introduction
- Jupiter developer overview: https://dev.jup.ag/get-started/overview
- Maple syrupUSDC Solana launch note: https://maple.finance/insights/syrupusdc-expands-to-solana
- Kamino syrupUSDC forum note: https://gov.kamino.finance/t/introducing-syrupusdc-on-kamino/762
- Drift IF staking docs: https://docs.drift.trade/insurance-fund/insurance-fund-staking
- Drift program addresses: https://docs.drift.trade/about-v2/program-vault-addresses
- Drift protocol repository: https://github.com/drift-labs/protocol-v2

## Kamino USDC

Official docs describe Kamino Lend as a borrow/lend product with SDK support for
deposit and withdraw transaction construction. The KLend repository identifies
the mainnet program as:

```text
KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD
```

Reference repo findings converge on these implementation requirements:

- Target the standalone KLend USDC reserve path.
- Treat USDC as the base mint:
  `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`.
- Use adapter-owned USDC and cToken vaults.
- Measure the cToken vault delta on deposit and store that as position units.
- Measure the USDC vault delta on withdraw and transfer the actual redeemed
  amount to the owner.
- Value the position from reserve exchange-rate state, not from reserve TVL.
- Refresh reserve state with the exact oracle accounts expected by the live
  reserve; fork tests must clone reserve, mint, vault, and oracle state from a
  consistent slot.

Risk note: several repos failed by using stale KLend instruction variants or by
mixing reserve/vault snapshots. The implementation should fail closed when the
reserve layout or oracle account set does not match the pinned USDC reserve.

## MarginFi USDC

The marginfi docs state that marginfi v2 is deployed at:

```text
MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA
```

The same docs describe `lending_account_deposit` as moving funds from a user
token account to a bank liquidity vault through a MarginfiAccount and bank.

Reference repo findings:

- Use the main USDC bank:
  `2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB`.
- Deposit CPI is `lending_account_deposit(amount, Option<bool>)`.
- Withdraw CPI is `lending_account_withdraw(amount, Option<bool>)` or the live
  variant required by the deployed IDL.
- Withdraw must include MarginFi health-check accounts, at minimum the active
  bank and oracle for a single-USDC-bank position.
- Position value is:

```text
asset_shares_for_bank * bank.asset_share_value
```

Both operands are fixed-point. The multiplication must use a widened
intermediate and only shift/truncate at the end.

Design decision: prefer a deterministic adapter-owned MarginFi account PDA when
using the CPI variant that supports PDA initialization. If a fork shows the
deployed instruction requires a standalone signer account, the adapter must store
that key during setup and tests must sign with that keypair.

## Jupiter LP / JLP

Jupiter docs describe JLP as the liquidity-provider token for Jupiter Perps. JLP
represents a share of the pool and accrues value as trading activity generates
fees. Jupiter developer docs list the Perpetuals program as:

```text
PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu
```

Reference repo findings:

- JLP mint:
  `27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4`.
- Common JLP pool:
  `5BUwFW4nRbftYTDMbgxykoFWqWHPzahFSNAaaaJtVKsq`.
- Use the V2 liquidity instructions:
  `add_liquidity2` and `remove_liquidity2`.
- Store position units as actual JLP token balance delta.
- Withdraw should redeem to an adapter-owned USDC vault first, then transfer the
  measured USDC delta to the owner.
- Value is:

```text
jlp_tokens_held * pool_aum_usd / jlp_mint_supply
```

Fork issue: the mutation path needs the full pool AUM account tail, including all
custody and price accounts in Jupiter's expected order. Price feeds have a short
freshness window, so fork tests should refresh or reclone them immediately before
mutation.

## Maple Syrup

Maple's own Solana launch note says syrupUSDC is live on Solana and powered by
Chainlink CCIP. The Kamino forum note says users can access syrupUSDC by
transferring from Ethereum through CCIP or by buying it on Solana through Kamino
Swap. This is not the same as a synchronous Solana-native Maple lending deposit.

Adapter decision for SYAS-1:

- Implement Maple Syrup as USDC entry into syrupUSDC exposure through a Solana
  liquidity route.
- Name the adapter as Maple Syrup exposure, not native Maple lending.
- Use syrupUSDC as the receipt position unit:
  `AvZZF1YaZDziPY2RCK4oJrRVrbN3mTD9NL24hPeaZeUj`.
- For a fork-testable path, use the Orca Whirlpool identified by multiple
  reference repos:
  `6fteKNvMdv7tYmBoJHhj1jx6rHcEwC6RdSEmVpyS613J`.
- Deposit swaps USDC into syrupUSDC and records the actual syrupUSDC delta.
- Withdraw swaps syrupUSDC back to USDC and transfers the measured USDC delta.

Risk note: do not claim the adapter is using native Maple mint/redeem unless the
implementation is changed to a cross-chain pending-settlement design. That would
not be a single-transaction mainnet-fork round trip.

## Drift Insurance Fund

Drift docs describe the Insurance Fund as a protocol backstop. IF stakers earn a
share of revenue, and unstaking has a 13-day cooldown with one pending unstake
request per vault.

Official program address:

```text
dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH
```

Implementation model:

- Use USDC spot market index `0`.
- Create or validate the Drift user-stats and insurance-fund-stake accounts for
  the adapter position authority.
- Deposit calls `add_insurance_fund_stake`.
- Withdraw is two-phase:
  - first call requests removal and records pending state;
  - later call settles after Drift's cooldown.
- Value is:

```text
if_shares * insurance_fund_vault_balance / total_if_shares
```

Reference repo warning: multiple repos report that the current deployed Drift
binary may reject IF CPIs with `InstructionFallbackNotFound`, despite the public
docs and SDK layouts describing those instructions. The fork suite must probe
the live executable and report this as an upstream blocker if reproduced, rather
than silently replacing the adapter with a mock.

## Implementation Rules From Research

- Dispatcher remains protocol-agnostic.
- Adapters must re-check the registry entry on every call.
- Every protocol account in the remaining-account tail must be treated as
  untrusted.
- Position units must be actual protocol receipt units or shares, not original
  deposit amounts unless the protocol truly defines 1:1 units.
- `current_value` must read live protocol state and return base-mint minor units.
- Mainnet-fork tests should clone or refresh all accounts that external CPIs
  touch, not only the accounts explicitly read by adapter code.
