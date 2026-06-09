# Build Your Own Adapter

This checklist is scoped for an experienced Solana engineer to get a first
conforming adapter working in one focused day, including a clear list of any
evidence that still needs a fork or live deployment.

## 1. Pick the Venue Operation

Before writing code, answer:

- What asset is deposited? For this repo, most adapters use USDC.
- What unit represents the position? Examples: cTokens, bank shares, JLP,
  syrupUSDC, or insurance fund shares.
- Is withdrawal instant or delayed?
- What state is required for live valuation?
- Which protocol accounts must be passed as remaining accounts?

Start from deployed reality: program IDL, account bytes, account owners, and a
same-slot mainnet snapshot. Treat docs and SDK examples as hints until the
deployed accounts confirm them.

## 2. Create the Program

Each adapter is its own Anchor program under a clear adapter path. It depends on
the shared interface crate and reads the registry entry for its own program id.

The adapter implements:

```rust
pub fn deposit(ctx: Context<Deposit>, amount: u64, min_position_out: u64) -> Result<()>
pub fn withdraw(ctx: Context<Withdraw>, position_amount: u64, min_amount_out: u64) -> Result<()>
pub fn current_value(ctx: Context<CurrentValue>) -> Result<()>
```

Keep these method names unchanged. Setup instructions may exist, but they are not
part of the public adapter standard.

## 3. Define Account Order

Use the standard prefix documented in `architecture.md`, then append
protocol-specific remaining accounts.

Use the shared PDA helpers instead of inventing new custody seeds:

```text
position = ["position", owner, base_mint]
position_authority = ["position_authority", position]
base vault = ["vault", position, base_mint]
receipt vault = ["receipt_vault", position, receipt_mint]
```

In the adapter source, document the remaining account order in one place:

```text
0. reserve
1. market
2. market authority
3. protocol vault
4. protocol program
```

Validate that order before any token movement or protocol CPI. Remaining
accounts are caller-supplied bytes and should be treated as hostile input.

## 4. Gate Direct Calls

Every adapter instruction should validate:

- Registry entry is owned by the registry program.
- Registry entry PDA matches the adapter program id.
- Adapter status is enabled.
- Registry base mint equals the supplied base mint.

This check belongs in the adapter because the dispatcher is optional at the
transaction layer.

## 5. Validate Token Accounts

For each token account, check:

- owner
- mint
- token program
- authority
- writable flag when mutation is required

For Token-2022 support, avoid hardcoding SPL Token unless the adapter explicitly
supports only classic SPL Token.

## 6. Write the CPI

Build protocol CPIs with explicit account metas. The dispatcher should only see
the standard prefix and an ordered tail; it should not know what a reserve, bank,
pool, or oracle is.

Rules:

- Prefer a single venue instruction per adapter operation.
- Put required refresh/crank instructions before the dispatcher call as separate
  top-level instructions.
- Use `invoke_signed` only with the adapter's own PDA seeds.
- Validate the protocol program id before invoking.
- Measure token balance deltas after the CPI instead of assuming the requested
  amount was filled exactly.

## 7. Implement Valuation

`current_value` is easy to make plausible and wrong. Make the formula explicit
before coding.

Compute the caller's claim from units this adapter position actually controls:

```text
value = position_units * exchange_rate
```

Examples:

- Kamino: cToken amount converted through reserve collateral exchange rate.
- MarginFi: asset shares multiplied by bank asset share value.
- Jupiter JLP: JLP token balance multiplied by pool AUM divided by JLP supply.
- Maple Syrup: syrupUSDC balance valued by an independent exchange-rate source.
- Drift IF: IF shares divided by total IF shares multiplied by fund vault value.

The following numbers are useful context, but they are not the answer:

- protocol total deposits
- vault liquidity
- reserve available amount
- a cached value whose freshness was not checked

## 8. Add Tests

Every adapter needs:

- deposit test
- withdraw test
- current_value test
- failure-path test

Failure paths should include at least:

- disabled adapter
- wrong mint
- wrong protocol account
- impossible slippage limit

Run against a mainnet fork whenever the protocol path exists. If a test only
checks account plans, IDL shape, or clone lists, name it as readiness evidence.
Do not present it as a deposit/value/withdraw fork pass.

## 9. Document the Adapter

Add an adapter section to `docs/protocol-notes.md` with:

- protocol program id
- supported mint
- remaining account order
- deposit flow
- withdraw flow
- current_value formula
- fork fixture accounts
- known limitations

## 10. Register

Governance registers and enables the adapter in the registry. The dispatcher
should route to it without a dispatcher release.

If adding a venue requires new dispatcher branches, the adapter boundary has
leaked.
