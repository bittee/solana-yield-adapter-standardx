# Solana Yield Adapter Standard

A Superteam Ukraine bounty implementation of a minimal Solana Yield Adapter
Standard for routing deposits, withdrawals, and position valuation through a
single dispatcher interface.

## About Superteam Ukraine

Superteam Ukraine is focused on onboarding the next generation of developers and
founders in Ukraine into the Solana ecosystem. The community connects talented
builders with opportunities across Solana.

## Mission

Create a reference implementation for a Solana Yield Adapter Standard, including:

- a core dispatcher contract
- five reference adapters
- an on-chain adapter registry
- mainnet-fork tests
- developer documentation and adapter specification

## Tech Stack

- Solana `2.2.20`
- Anchor `0.31.1`
- Rust
- TypeScript

## Architecture

Applications integrate with one dispatcher. The dispatcher validates the adapter
through the registry and routes the request to the selected adapter. Protocol
logic stays inside adapter programs.

```text
Application
  -> Dispatcher
    -> Registry
    -> Adapter
      -> Yield Protocol
```

The dispatcher is protocol-agnostic. The registry is governance-gated and never
holds user funds. Each adapter is independently testable and owns its
protocol-specific account validation, CPI, custody, and valuation logic.

## Standard Interface

Every adapter exposes the same three-method interface:

```rust
deposit(...)
withdraw(...)
current_value(...)
```

- `deposit`: allocate assets into a strategy
- `withdraw`: redeem assets from a strategy
- `current_value`: return current position value in base-mint minor units

## Programs

| Program               | Path                              | Purpose                                                                 |
| --------------------- | --------------------------------- | ----------------------------------------------------------------------- |
| Dispatcher            | `programs/dispatcher`             | Routes `deposit`, `withdraw`, and `current_value` to approved adapters. |
| Registry              | `programs/registry`               | Stores governance-approved adapter entries and status.                  |
| Kamino USDC Adapter   | `programs/adapters/kamino-usdc`   | USDC yield route for Kamino Finance.                                    |
| MarginFi USDC Adapter | `programs/adapters/marginfi-usdc` | USDC lending route for MarginFi.                                        |
| Jupiter JLP Adapter   | `programs/adapters/jupiter-jlp`   | USDC entry into Jupiter LP/JLP exposure.                                |
| Maple Syrup Adapter   | `programs/adapters/maple-syrup`   | USDC entry into Maple Syrup exposure.                                   |
| Drift IF Adapter      | `programs/adapters/drift-if`      | Drift Insurance Fund route.                                             |

## On-Chain Registry

The registry supports:

- governance-controlled initialization
- adapter registration
- adapter enable/disable
- governance transfer

Each adapter entry is stored in its own PDA keyed by adapter program id. The
dispatcher checks the registry entry before routing, and adapters also re-check
their registry entry on direct calls.

## Mainnet-Fork Tests

The strict fork runner is implemented in `scripts/mainnet-fork/run.ts`.

It:

- requires `MAINNET_RPC_URL`
- builds SBF artifacts with `anchor build --no-idl`
- starts `solana-test-validator` from mainnet state
- clones required protocol programs and accounts
- loads local dispatcher, registry, and adapter programs with `--bpf-program`
- runs the adapter round-trip suite in `tests/mainnet-fork/roundtrip.spec.ts`
- writes fork evidence to `target/syas-mainnet-fork-evidence.json`

Run:

```bash
MAINNET_RPC_URL=<mainnet-rpc> npm run test:fork
```

## Developer Documentation

- [Adapter Standard](docs/standard.md)
- [Architecture](docs/architecture.md)
- [Build Your Own Adapter](docs/build-your-own-adapter.md)
- [Protocol Notes](docs/protocol-notes.md)
- [Mainnet Fork Tests](docs/mainnet-fork-tests.md)
- [Bounty Submission Runbook](docs/submission.md)

The adapter guide is written so a Solana team can implement a new adapter in
less than one day by following the standard account prefix, registry checks, PDA
model, and test requirements.

## Quick Start

```bash
npm install
npm run keys:ids:check
npm run typecheck
npm test
npm run build
```

`npm run build` uses `anchor build --no-idl`.

## Devnet Registry Deployment

The bounty requires the registry contract to be deployed to devnet.

```bash
export ANCHOR_WALLET="$HOME/.config/solana/id.json"
export SOLANA_DEVNET_RPC_URL="<devnet-rpc>"

npm run keys:restore:registry
npm run keys:verify:registry
npm run deploy:registry:devnet
npm run verify:registry:devnet
```

If the original registry program keypair is not available, generate and sync a
new local program id set:

```bash
npm run keys:sync
npm run keys:ids:check
```

Commit the synchronized public id changes if `keys:sync` is used. Do not commit
program keypairs.

## Final Bounty Gate

Run this from the Linux environment that has Anchor, Solana CLI, and
`solana-test-validator` installed:

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

The final submission should include:

- public GitHub repository URL
- devnet registry program id and registry PDA
- successful `npm run verify:registry:devnet` output
- successful `npm run test:fork` output
- successful `npm run bounty:check` output
- `target/syas-mainnet-fork-evidence.json`

## Bounty Scope

### Core Dispatcher Contract

Anchor program that routes a standardized interface:

- `deposit`
- `withdraw`
- `current_value`

### Five Reference Adapters

- Kamino USDC
- MarginFi USDC
- Jupiter LP/JLP
- Maple Syrup
- Drift Insurance Fund

### On-Chain Adapter Registry

Governance-gated adapter approval, enable, and disable mechanism.

### Mainnet-Fork Tests

Integration tests for all five adapters against mainnet state.

### Developer Specification

Markdown specification and "Build your own adapter" guide.

## Submission Requirements

- Public GitHub repository containing all source code
- All five adapters passing mainnet-fork tests
- Registry contract deployed to devnet
- Adapter standard specification in markdown format
- "How to build your own adapter" developer guide

## Judging Criteria

| Category                                              | Weight |
| ----------------------------------------------------- | -----: |
| Correctness: adapters function against mainnet fork   |    40% |
| Interface design: clean, minimal, extensible standard |    25% |
| Developer guide quality                               |    20% |
| Code quality and test coverage                        |    15% |

## Reward Structure

- 1st Place: 700 USDC
- 2nd Place: 300 USDC
