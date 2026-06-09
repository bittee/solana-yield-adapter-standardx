# Protocol Notes

These notes are the protocol map for the five required adapters. They summarize
what the reference repos taught us, with unresolved or risky areas called out
instead of hidden behind optimistic wording.

## Shared Rules

All adapters must:

- Validate the registry entry on direct calls.
- Validate the base mint.
- Validate protocol program id and account ownership.
- Validate all PDA derivations.
- Validate token account mint and authority.
- Use checked arithmetic.
- Enforce slippage floors.
- Use same-slot protocol state in fork tests where possible.

## Kamino USDC

Protocol:

```text
Kamino KLend: KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD
Base mint:    EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```

Preferred deposit route:

- If the chosen KLend path requires refreshes, submit them before the dispatcher
  route as top-level instructions.
- Transfer user USDC into the adapter vault.
- CPI into KLend's reserve deposit instruction.
- Measure the collateral-token increase in the adapter vault and store that as
  the position unit.

Preferred withdrawal route:

- Run required refreshes before the route.
- CPI into KLend's collateral redemption instruction.
- Measure the USDC increase in the adapter vault.
- Transfer the measured USDC amount to the user.

current_value:

```text
value = adapter collateral units * reserve collateral exchange rate
```

Avoid valuing by reserve available liquidity or total reserve deposits.
Use wide integer math for exchange-rate conversion and reject zero collateral
supply or malformed reserve data.

Validation focus:

- reserve owner and discriminator/layout
- reserve liquidity mint equals USDC
- reserve liquidity supply vault matches the reserve
- reserve collateral mint
- lending market and lending market authority relationship
- adapter collateral vault mint and authority
- instruction sysvar when required by KLend

Fork caveats:

- Reserve/oracle freshness can break old snapshots.
- Refresh instructions should not be hidden inside the adapter CPI if that causes
  call-depth or compute issues.
- Clone reserve, vault, and mint accounts from the same slot; mismatched reserve
  accounting can fail KLend invariants even when each account exists.

Implemented tail after the standard prefix:

```text
9. collateral mint
10. adapter collateral vault
11. reserve
12. lending market
13. lending market authority
14. reserve liquidity supply
15. instruction sysvar
16. KLend program
```

## MarginFi USDC

Protocol:

```text
MarginFi v2: MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA
Base mint:   EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```

Deposit flow:

- Create or validate a MarginFi account whose authority is the adapter position
  authority.
- Transfer user USDC into the adapter vault.
- CPI into `lending_account_deposit` with the optional limit argument encoded as
  the deployed program expects.
- Record the resulting asset shares for the USDC bank.

Withdraw flow:

- CPI into `lending_account_withdraw`; pass the withdraw-all flag only when the
  position is being fully closed.
- Include health-check remaining accounts required by MarginFi.
- Transfer the actual USDC delta to the user.

current_value:

```text
value = adapter asset shares * bank asset share value
```

Validation focus:

- bank owner and layout
- bank mint equals USDC
- bank belongs to expected group
- liquidity vault and vault authority relationship
- marginfi account authority equals the adapter position authority
- marginfi account balance entry belongs to the USDC bank
- oracle account used for health checks

Fork caveats:

- Health-check remaining accounts are easy to omit.
- Interest accrual means exact equality tests are fragile; use narrow tolerance.
- Do not truncate fixed-point values before multiplication. Multiply in a wide
  type and shift only at the end.

Implemented tail after the standard prefix:

```text
9. adapter-owned MarginFi account PDA
10. MarginFi group
11. USDC bank
12. bank liquidity vault authority
13. bank liquidity vault
14. USDC bank oracle
15. instruction sysvar
16. MarginFi program
```

## Jupiter LP / JLP

Protocol:

```text
Jupiter Perps: PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu
JLP mint:       27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4
Common pool:    5BUwFW4nRbftYTDMbgxykoFWqWHPzahFSNAaaaJtVKsq
```

Deposit flow:

- Transfer user USDC into the adapter vault.
- CPI into Jupiter Perps `add_liquidity2`.
- Receive JLP into the adapter JLP vault.
- Record JLP balance delta as position units.

Withdraw flow:

- CPI into Jupiter Perps `remove_liquidity2`.
- Burn/redeem JLP for USDC.
- Transfer received USDC to the user.

current_value:

```text
value = adapter JLP amount * pool AUM / JLP mint supply
```

Validation focus:

- pool account owner and layout
- JLP mint key
- custody belongs to pool
- custody mint equals USDC
- custody token account relationship
- all custody and price accounts required by the pool, in the order Jupiter
  expects
- Jupiter Perps program id

Fork caveats:

- The account list is large. Tests may need address lookup tables.
- Doves/Pythnet price account freshness can fail on slow fork startup.
- Remaining accounts for pool AUM must be pinned and documented.
- Use the deployed V2 liquidity instructions. Older liquidity instruction names
  may deserialize locally but fail against the live program.

Implemented tail after the standard prefix:

```text
9. JLP mint
10. adapter JLP vault
11. Jupiter transfer authority
12. perpetuals account
13. pool
14. USDC custody
15. custody Doves price account
16. custody Pythnet price account
17. custody token account
18. event authority
19. Jupiter Perps program
20+. optional Jupiter AUM tail passed through to the CPI
```

## Maple Syrup

Known Solana asset:

```text
syrupUSDC mint: AvZZF1YaZDziPY2RCK4oJrRVrbN3mTD9NL24hPeaZeUj
```

Important design note:

Maple needs a naming decision before code. A synchronous Solana instruction that
mints syrupUSDC from USDC is different from buying syrupUSDC through Solana
liquidity, and both are different from a future cross-chain settlement flow.
The adapter docs and tests must identify the exact primitive being standardized.

Acceptable designs:

- USDC -> syrupUSDC market-execution adapter, using a direct pool and slippage
  limits.
- syrupUSDC custody adapter, where the base mint is syrupUSDC instead of USDC.
- Cross-chain pending-settlement adapter, if the standard is later extended for
  asynchronous proof-based settlement.

For this repo's required "Maple Syrup" adapter, the SYAS-1-compatible route is
USDC entry into syrupUSDC exposure through one direct Solana pool. Do not
describe that as native Maple lending; describe it as Maple Syrup exposure.

current_value:

```text
value = syrupUSDC held * independent syrupUSDC/USDC exchange rate
```

If entry and exit use a liquidity pool, mark the held position from an
independent feed or accounting source. The same trade path should not also set
the reported value.

Validation focus:

- fixed syrupUSDC mint
- pool program id and pool account
- adapter USDC and syrupUSDC vault PDAs
- syrupUSDC vault mint and authority
- exchange-rate feed owner, key, timestamp, decimals, and positive answer

Fork caveats:

- Do not claim native Maple Solana mint/redeem unless verified against the
  deployed program and a live fork.
- Label DEX exposure clearly as Maple Syrup exposure, not native Maple lending.
- Run clock-warping tests after oracle-sensitive Maple/Jupiter tests, because
  time travel can make short-lived feeds stale.

Implemented tail after the standard prefix:

```text
9. syrupUSDC mint
10. adapter syrupUSDC vault
11. Orca Whirlpool
12. Whirlpool token vault A
13. Whirlpool token vault B
14. tick array 0
15. tick array 1
16. tick array 2
17. Whirlpool oracle
18. Orca Whirlpool program
```

## Drift Insurance Fund

Protocol:

```text
Drift v2: dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH
Base mint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```

Deposit flow:

- Create or validate Drift user stats / insurance fund stake accounts required
  by the deployed Drift interface.
- Transfer or authorize USDC from adapter custody.
- CPI into Drift to add insurance fund stake.
- Record insurance fund shares.

Withdrawal route:

- Drift insurance fund withdrawal is cooldown-based.
- First `withdraw` requests stake removal and stores the pending amount and
  unlock condition.
- A later `withdraw` settles when Drift allows removal.

current_value:

```text
value = adapter IF shares * insurance fund vault balance / total IF shares
```

While a withdrawal is pending, cap the reported value to the amount Drift says is
removable for that request, if the protocol exposes such a field.

Validation focus:

- Drift state
- spot market index and mint
- insurance fund vault
- insurance fund stake account authority
 
Implemented tail after the standard prefix:

```text
9. user stats PDA
10. insurance fund stake PDA
11. Drift state PDA
12. USDC spot market
13. USDC spot market vault
14. insurance fund vault
15. Drift signer PDA
16. rent sysvar
17. Drift program
```
- user stats account
- oracle if required by the path

Fork caveats:

- Verify that the deployed Drift program exposes the needed insurance fund stake
  instructions by CPI before claiming live support.
- If the deployed venue cannot dispatch the required instruction, report that
  blocker separately. A substitute program may test local lifecycle logic, but it
  is not live Drift evidence.
- Long cooldown tests need clock control. Keep them isolated from oracle tests
  whose timestamps are sensitive.
