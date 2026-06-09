# Mainnet Fork Tests

The fork suite records what ran against cloned mainnet accounts and separates
local preflight checks from live protocol round-trip evidence.

## Required Tests Per Adapter

Each adapter needs:

- deposit test
- withdraw test
- current_value test
- failure-path test

Failure-path tests should cover:

- disabled adapter
- wrong base mint
- wrong registry entry
- wrong protocol account
- impossible slippage limit

## Test Categories

Use separate names for separate levels of evidence.

### Unit

Fast local checks:

- instruction discriminator stability
- PDA derivation
- account size/layout
- math helpers
- error conditions that do not need protocol state

### Preflight

Static and fixture checks:

- account plan can be built
- fixture accounts exist
- protocol IDL contains expected instructions
- clone account list is deduplicated

These tests catch wrong addresses and account ordering. Reserve the fork
round-trip label for tests that execute adapter mutations against cloned protocol
state.

### Fork Round Trip

A round-trip test runs on a local validator loaded with mainnet accounts and
executes the complete adapter path:

```text
register adapter
enable adapter
fund test user
deposit
current_value
withdraw
assert balances and position state
```

The round-trip label is reserved for executions that reach the target protocol
path on the forked validator.

## Fork Harness Requirements

The fork harness should:

- require an explicit `MAINNET_RPC_URL` or equivalent.
- print the fork slot or snapshot slot.
- clone all required protocol accounts.
- document accounts cloned for each adapter.
- avoid embedding private keypairs in the repo.
- record logs or transaction signatures for evidence.
- separate fixture checks from mutation tests.

The TypeScript harness should build adapter accounts from `sdk/src/index.ts`
PDA helpers:

- `positionPda`
- `positionAuthorityPda`
- `adapterVaultPda`
- `receiptVaultPda`

Each adapter then appends the protocol tail documented in
`docs/protocol-notes.md`.

## Current Harness

This repo now includes two fork-test layers:

```text
tests/mainnet-fork/account-plan.spec.ts
tests/mainnet-fork/roundtrip.spec.ts
scripts/mainnet-fork/run.ts
```

Run local account-plan checks:

```bash
node --import tsx ./node_modules/mocha/bin/mocha.js --timeout 1000000 tests/mainnet-fork/account-plan.spec.ts
```

Run strict mainnet-fork roundtrips:

```bash
MAINNET_RPC_URL=<rpc> npm run test:fork
```

The strict runner:

- builds SBF artifacts with `anchor build --no-idl`;
- starts `solana-test-validator` against `MAINNET_RPC_URL`;
- clones external protocol programs and protocol accounts;
- loads registry, dispatcher, and all five local adapters with
  `--bpf-program` using the checked-in program ids;
- injects a deterministic funded USDC token account for the test wallet;
- patches Doves oracle timestamps before validator startup;
- injects documented local fixtures for required accounts that the current
  mainnet RPC reports as absent, currently the derived Maple Whirlpool oracle;
- runs `tests/mainnet-fork/roundtrip.spec.ts` with
  `RUN_MAINNET_FORK_TESTS=1`.

The round-trip spec requires `RUN_MAINNET_FORK_TESTS=1`. Without that explicit
flag, local test runs execute only the preflight layer, keeping strict fork
evidence separate from normal development checks.

## Protocol Notes

Kamino:

- The adapter uses the reserve-liquidity and reserve-collateral instructions
  directly.
- Reserve liquidity, collateral mint, lending market, and market authority are
  cloned from mainnet.

MarginFi:

- The adapter creates a MarginFi PDA account for the position authority.
- Position value is reported in base-mint units, not raw I80F48 share bits.
- Deposit and withdraw paths use the live USDC bank and liquidity vault.

Jupiter:

- The adapter calls `addLiquidity2` and `removeLiquidity2`.
- The fork runner patches Doves oracle timestamps before validator startup so
  price freshness is deterministic.

Maple:

- The adapter uses the live Orca Whirlpool for the Maple Syrup token/USDC route.
- The configured Whirlpool oracle PDA currently returns `AccountNotFound` from
  mainnet RPC, so the runner injects an empty system-owned startup fixture for
  that account before validator startup.

Drift:

- Insurance fund withdrawal is two-phase.
- The fork suite treats the first withdraw as the request phase and checks that
  pending shares remain until the protocol unlock period elapses.
