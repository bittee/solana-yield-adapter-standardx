# Mainnet Fork Tests

The fork suite should show what actually ran against cloned mainnet accounts.
When a protocol path cannot execute, the report should say that directly and
record the reason.

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

If the deployed program does not expose the needed instruction, or the fork
cannot keep required accounts fresh, report that blocker separately. A substitute
program may test local lifecycle logic, but it must not be labeled as live
protocol evidence.

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

If `RUN_MAINNET_FORK_TESTS` is not set, the strict roundtrip suite is skipped.
This prevents local runs from being mislabeled as live protocol evidence.

Current environment note: using `https://api.mainnet-beta.solana.com` from this
machine failed before fork startup because TLS verification saw a certificate
for `sinkhole.cert.gov.ua` instead of `api.mainnet-beta.solana.com`. Strict
roundtrips need a reachable RPC endpoint with a valid certificate from this
environment.

## Known Fork Issues

Kamino:

- Refresh reserve/obligation state before mutation.
- Oracle freshness may require patched local fixture bytes.

MarginFi:

- Withdraw requires health-check remaining accounts.
- Small value differences can happen due to interest accrual.

Jupiter:

- Large account sets may require address lookup tables.
- Short-lived price feeds can expire while the validator starts.

Maple:

- Native Maple Solana mint/redeem needs deployed-program proof before being
  reported as implemented.
- DEX exposure tests should be labeled as DEX exposure.
- The configured Whirlpool oracle PDA currently returns `AccountNotFound` from
  mainnet RPC, so the runner injects an empty system-owned startup fixture for
  that account. Whirlpool treats this as an uninitialized oracle account. This
  only gets the fork validator past cloning; successful Maple round-trip logs
  are still required before claiming adapter correctness.

Drift:

- Insurance fund withdrawal is two-phase.
- Verify deployed entrypoints are CPI-callable before reporting live support.
