# Solana Yield Adapter Standard

SYAS is a reference implementation of a small adapter layer for Solana yield
integrations. An application calls one dispatcher interface, and the selected
adapter handles the protocol-specific accounts, CPI, custody, share accounting,
and valuation.

The goal is to make yield integrations look the same at the application layer:

```text
deposit
withdraw
current_value
```

Instead of every wallet, vault, or strategy app integrating Kamino, MarginFi,
Jupiter, Maple, and Drift separately, SYAS defines a common route through a
dispatcher, a governance-gated registry, and independent protocol adapters.

## What Is In This Repo

- Anchor dispatcher program with protocol-agnostic routing.
- Anchor registry program with governance-gated adapter approval.
- Five independent adapter programs:
  - Kamino USDC
  - MarginFi USDC
  - Jupiter JLP
  - Maple Syrup exposure
  - Drift Insurance Fund
- TypeScript SDK helpers for program ids, PDAs, and adapter routes.
- Strict mainnet-fork runner for live protocol-state tests.
- Developer specification and "build your own adapter" guide.
- Submission gate script that runs local checks, devnet registry verification,
  and fork tests.

## Architecture

```text
Application
  -> Dispatcher
    -> Registry
    -> Adapter
      -> Yield Protocol
```

The dispatcher does only routing and admission control. It checks that the
adapter is registered, enabled, and supports the requested base mint. It does not
parse protocol accounts, calculate yield, hold user funds, or branch on protocol
type.

The registry stores one adapter-entry PDA per adapter program id. Governance can
register, enable, disable, and transfer registry ownership. The registry never
holds strategy assets.

Adapters own the protocol-specific logic. Each adapter validates its remaining
accounts, checks token mints and PDA relationships, performs the protocol CPI,
records position units, and reports value in base-mint minor units.

## Standard Interface

Every adapter implements the same application-facing surface:

```rust
deposit(...)
withdraw(...)
current_value(...)
```

`deposit` moves base assets into the adapter strategy and records position units.
`withdraw` redeems position units back to base assets or starts the protocol
cooldown flow. `current_value` reads live protocol state and returns the current
position value.

Detailed semantics are in [docs/standard.md](docs/standard.md).

## Programs

| Program               | Path                              | Role                                                               |
| --------------------- | --------------------------------- | ------------------------------------------------------------------ |
| Dispatcher            | `programs/dispatcher`             | Single route entrypoint for apps.                                  |
| Registry              | `programs/registry`               | Governance-controlled adapter directory.                           |
| Kamino USDC Adapter   | `programs/adapters/kamino-usdc`   | KLend USDC reserve route.                                          |
| MarginFi USDC Adapter | `programs/adapters/marginfi-usdc` | MarginFi USDC bank route.                                          |
| Jupiter JLP Adapter   | `programs/adapters/jupiter-jlp`   | USDC to JLP liquidity route.                                       |
| Maple Syrup Adapter   | `programs/adapters/maple-syrup`   | USDC to syrupUSDC exposure route.                                  |
| Drift IF Adapter      | `programs/adapters/drift-if`      | Drift Insurance Fund staking route with cooldown-aware withdrawal. |

## Repository Layout

```text
programs/
  dispatcher/
  registry/
  adapters/
    kamino-usdc/
    marginfi-usdc/
    jupiter-jlp/
    maple-syrup/
    drift-if/
sdk/
scripts/
  bounty/
  devnet/
  mainnet-fork/
tests/
  mainnet-fork/
docs/
```

## Main Commands

```bash
npm install
npm run keys:ids:check
npm run typecheck
npm test
npm run build
```

`npm run build` runs `anchor build --no-idl`, which is the build path used by the
fork runner and bounty gate.

`npm test` runs fast TypeScript/conformance tests. Strict mainnet-fork adapter
round trips are intentionally separate because they require Solana CLI,
`solana-test-validator`, Anchor, and a mainnet RPC.

## Mainnet-Fork Tests

The strict fork runner lives at [scripts/mainnet-fork/run.ts](scripts/mainnet-fork/run.ts).
It builds local SBF artifacts, starts `solana-test-validator` from mainnet state,
clones required protocol accounts, loads local programs with `--bpf-program`,
and runs [tests/mainnet-fork/roundtrip.spec.ts](tests/mainnet-fork/roundtrip.spec.ts).

```bash
export MAINNET_RPC_URL="<mainnet-rpc>"
npm run test:fork
```

On success, the runner writes:

```text
target/syas-mainnet-fork-evidence.json
```

That file records the fork slot, local validator URL, payer, and program ids used
for the test run.

## Devnet Registry

The registry program is the only program that must be deployed to devnet for the
submission package.

```bash
export ANCHOR_WALLET="$HOME/.config/solana/id.json"
export SOLANA_DEVNET_RPC_URL="<devnet-rpc>"

npm run keys:restore:registry
npm run keys:verify:registry
npm run deploy:registry:devnet
npm run verify:registry:devnet
```

If the matching registry keypair is not available, generate a new local program
id set and commit the synchronized public id changes:

```bash
npm run keys:sync
npm run keys:ids:check
```

Do not commit `program-keypairs/` or `target/deploy/*-keypair.json`.

## Final Verification Gate

`npm run bounty:check` is the full submission gate. It runs the local Rust and
TypeScript checks, verifies required tool versions, checks the devnet registry,
and executes strict mainnet-fork tests.

Run it from a Linux environment with:

- Solana CLI `2.2.20`
- `solana-test-validator` `2.2.20`
- Anchor CLI `0.31.1`
- Rust with `rustfmt` and `clippy`
- Node.js dependencies installed
- funded devnet wallet
- mainnet RPC URL

```bash
export ANCHOR_WALLET="$HOME/.config/solana/id.json"
export MAINNET_RPC_URL="<mainnet-rpc>"
export SOLANA_DEVNET_RPC_URL="<devnet-rpc>"

npm install
npm run keys:ids:check
npm run build
npm run deploy:registry:devnet
npm run verify:registry:devnet
npm run test:fork
npm run bounty:check
```

The useful submission artifacts are:

- public GitHub repository URL
- devnet registry program id
- devnet registry PDA
- `npm run verify:registry:devnet` output
- `npm run test:fork` output
- `npm run bounty:check` output
- `target/syas-mainnet-fork-evidence.json`

## Documentation

- [Adapter Standard](docs/standard.md)
- [Architecture](docs/architecture.md)
- [Build Your Own Adapter](docs/build-your-own-adapter.md)
- [Protocol Notes](docs/protocol-notes.md)
- [Mainnet Fork Tests](docs/mainnet-fork-tests.md)
- [Bounty Submission Runbook](docs/submission.md)

The adapter guide focuses on the path for a new Solana team to implement a new
adapter quickly: use the standard account prefix, validate the registry entry,
derive adapter-owned PDAs, implement the three standard methods, and add
mainnet-fork coverage.
