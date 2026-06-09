# Solana Yield Adapter Standard

This repo is my implementation of a small standard interface for Solana yield
adapters.

The idea is simple: an app should not need a different integration shape for
every yield venue. It should be able to call the same three actions everywhere:

```text
deposit
withdraw
current_value
```

The dispatcher gives apps one route. The registry decides which adapters are
approved. Each adapter keeps the messy protocol-specific part in one place:
account validation, CPI calls, custody, position units, and valuation.

## What Is Implemented

- Dispatcher program
- Governance-gated adapter registry
- Five adapter programs:
  - Kamino USDC
  - MarginFi USDC
  - Jupiter JLP
  - Maple Syrup exposure
  - Drift Insurance Fund
- TypeScript SDK helpers for ids and PDAs
- Mainnet-fork test harness
- Devnet registry deploy/verify scripts
- Adapter standard docs
- "Build your own adapter" guide
- `npm run bounty:check` submission gate

## How It Fits Together

```text
App
  -> Dispatcher
    -> Registry
    -> Adapter
      -> Protocol
```

The dispatcher is intentionally boring. It checks the registry entry, checks the
base mint, then forwards the call. It does not know how Kamino, MarginFi,
Jupiter, Maple, or Drift work.

The registry is the governance layer. It stores adapter entries and status. It
does not hold tokens.

Adapters do the protocol work. They validate their own remaining accounts,
derive their own custody PDAs, perform CPI into the target protocol, and report
value back in base-mint units.

## Standard Interface

Every adapter exposes the same application-facing methods:

```rust
deposit(...)
withdraw(...)
current_value(...)
```

`deposit` enters the strategy and records position units.

`withdraw` exits the strategy, or starts/continues a cooldown flow for protocols
that cannot settle immediately.

`current_value` reads current protocol state and returns the position value.

Full semantics are in [docs/standard.md](docs/standard.md).

## Programs

| Program               | Path                              |
| --------------------- | --------------------------------- |
| Dispatcher            | `programs/dispatcher`             |
| Registry              | `programs/registry`               |
| Kamino USDC Adapter   | `programs/adapters/kamino-usdc`   |
| MarginFi USDC Adapter | `programs/adapters/marginfi-usdc` |
| Jupiter JLP Adapter   | `programs/adapters/jupiter-jlp`   |
| Maple Syrup Adapter   | `programs/adapters/maple-syrup`   |
| Drift IF Adapter      | `programs/adapters/drift-if`      |

## Repo Layout

```text
programs/        Anchor programs
sdk/             TypeScript helpers
scripts/         key sync, devnet deploy, fork runner, bounty gate
tests/           conformance and mainnet-fork tests
docs/            standard, architecture, adapter guide, runbooks
```

## Local Checks

These are the quick checks I use before touching the full fork flow:

```bash
npm install
npm run keys:ids:check
npm run typecheck
npm test
npm run build
```

`npm run build` uses:

```bash
anchor build --no-idl
```

That is also the build mode used by the fork runner and bounty gate.

## Mainnet-Fork Tests

The fork runner is [scripts/mainnet-fork/run.ts](scripts/mainnet-fork/run.ts).
It builds the local programs, starts `solana-test-validator` against mainnet
state, clones the required protocol accounts, loads local programs with
`--bpf-program`, then runs the strict round-trip suite.

```bash
export MAINNET_RPC_URL="<mainnet-rpc>"
npm run test:fork
```

Successful fork runs write:

```text
target/syas-mainnet-fork-evidence.json
```

That file is meant to be included with the final submission logs.

## Devnet Registry

The registry program needs to be deployed and initialized on devnet.

```bash
export ANCHOR_WALLET="$HOME/.config/solana/id.json"
export SOLANA_DEVNET_RPC_URL="<devnet-rpc>"

npm run keys:restore:registry
npm run keys:verify:registry
npm run deploy:registry:devnet
npm run verify:registry:devnet
```

If the matching registry keypair is not available locally, generate a new synced
program id set:

```bash
npm run keys:sync
npm run keys:ids:check
```

If you do that, commit the public id changes. Do not commit generated keypairs.

## Final Gate

For bounty submission, the important command is:

```bash
npm run bounty:check
```

It is intentionally strict. It checks local Rust/TypeScript quality, required
tool versions, devnet registry state, and the mainnet-fork adapter tests.

Expected Linux environment:

- Solana CLI `2.2.20`
- `solana-test-validator` `2.2.20`
- Anchor CLI `0.31.1`
- Rust with `rustfmt` and `clippy`
- Node.js dependencies installed
- funded devnet wallet
- mainnet RPC URL

Full run:

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

## Submission Evidence

The submission should include:

- this public GitHub repo
- devnet registry program id:
  `BWTrd2xVhy2T12CLrr9ffy3StQtYdJtRWGejWLVtCd2p`
- devnet registry PDA: `33qVn5v9GNtJJTpLCy2AeFvvbU3TuY1HWZgZv7nmntjW`
- `npm run verify:registry:devnet` output
- `npm run test:fork` output
- `npm run bounty:check` output
- `target/syas-mainnet-fork-evidence.json`

## Docs

- [Adapter Standard](docs/standard.md)
- [Architecture](docs/architecture.md)
- [Build Your Own Adapter](docs/build-your-own-adapter.md)
- [Protocol Notes](docs/protocol-notes.md)
- [Mainnet Fork Tests](docs/mainnet-fork-tests.md)
- [Submission Runbook](docs/submission.md)
