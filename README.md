# Solana Yield Adapter Standard

Reference implementation for a small, composable yield-adapter interface on
Solana.

The repository contains an Anchor dispatcher, a governance-gated adapter
registry, five protocol adapters, a TypeScript SDK surface, strict mainnet-fork
tests, and the developer docs needed for another team to build a compatible
adapter quickly.

## At A Glance

| Area                         | Status                            |
| ---------------------------- | --------------------------------- |
| Core dispatcher              | `programs/dispatcher`             |
| Adapter registry             | `programs/registry`               |
| Kamino USDC adapter          | `programs/adapters/kamino-usdc`   |
| MarginFi USDC adapter        | `programs/adapters/marginfi-usdc` |
| Jupiter JLP adapter          | `programs/adapters/jupiter-jlp`   |
| Maple Syrup exposure adapter | `programs/adapters/maple-syrup`   |
| Drift Insurance Fund adapter | `programs/adapters/drift-if`      |
| Standard specification       | `docs/standard.md`                |
| Adapter guide                | `docs/build-your-own-adapter.md`  |
| Mainnet-fork harness         | `scripts/mainnet-fork/run.ts`     |
| Submission gate              | `npm run bounty:check`            |

## Bounty Coverage

| Requirement                  | Implementation                                                                         |
| ---------------------------- | -------------------------------------------------------------------------------------- |
| Core dispatcher contract     | Anchor router with `deposit`, `withdraw`, and `current_value` in `programs/dispatcher` |
| Five reference adapters      | Kamino, MarginFi, Jupiter JLP, Maple Syrup exposure, and Drift IF adapter programs     |
| On-chain adapter registry    | Governance-gated registry in `programs/registry`, deployed and initialized on devnet   |
| Mainnet-fork tests           | Strict fork runner that clones mainnet accounts and executes adapter round trips       |
| Developer specification      | `docs/standard.md`                                                                     |
| Build-your-own-adapter guide | `docs/build-your-own-adapter.md`                                                       |
| Submission evidence          | `npm run bounty:check` plus `target/syas-mainnet-fork-evidence.json`                   |

## Design

```text
Application
  -> Dispatcher
    -> Registry
    -> Adapter
      -> Yield Protocol
```

The dispatcher is only a router. It checks the registry entry, validates the
requested base mint, and forwards one of three standard actions:

```text
deposit
withdraw
current_value
```

The registry is the approval layer. It is controlled by governance, stores one
entry per adapter, and never takes custody of user assets.

Each adapter owns its protocol integration: account validation, protocol CPI,
position custody, share accounting, slippage checks, and valuation. The adapter
must also re-check its registry entry, so direct adapter calls cannot bypass
governance.

## Standard Surface

Every application-facing adapter implements:

| Method          | Meaning                                                                 |
| --------------- | ----------------------------------------------------------------------- |
| `deposit`       | Move the user's base asset into the strategy and record position units. |
| `withdraw`      | Exit the strategy or progress an adapter-managed cooldown flow.         |
| `current_value` | Return the current position value in base-mint minor units.             |

The standard keeps the app integration shape fixed while allowing protocol
details to stay inside adapters. Full account semantics and return-data rules
are documented in [docs/standard.md](docs/standard.md).

## Adapter Coverage

| Adapter              | Base / Position            | Integration Path                                         |
| -------------------- | -------------------------- | -------------------------------------------------------- |
| Kamino USDC          | USDC collateral position   | KLend reserve deposit and collateral redemption          |
| MarginFi USDC        | USDC lending position      | MarginFi v2 bank deposit, withdraw, and share valuation  |
| Jupiter JLP          | USDC to JLP                | Jupiter Perps `addLiquidity2` / `removeLiquidity2`       |
| Maple Syrup          | USDC to syrupUSDC exposure | Direct Whirlpool route with independent value accounting |
| Drift Insurance Fund | USDC IF stake              | Drift IF stake lifecycle with cooldown-aware withdraw    |

Protocol account maps and valuation notes are in
[docs/protocol-notes.md](docs/protocol-notes.md).

## Devnet Registry

The registry program is deployed and initialized on devnet.

```text
Registry program id: BWTrd2xVhy2T12CLrr9ffy3StQtYdJtRWGejWLVtCd2p
Registry PDA:        33qVn5v9GNtJJTpLCy2AeFvvbU3TuY1HWZgZv7nmntjW
Governance:          4vhi8SvYFGnaq2xhr8ocjf4HcEHqsSqM9swT56SNPNoh
```

Verify from a checkout:

```bash
npm run verify:registry:devnet
```

## Program IDs

These ids are synchronized across `Anchor.toml`, each program's `declare_id!`,
the SDK, and the fork runner.

| Program               | Program ID                                     |
| --------------------- | ---------------------------------------------- |
| Dispatcher            | `HGj3chDufhrN3LZE31jjK9Kv4ETzmzkxHwxDQKCzUrk`  |
| Registry              | `BWTrd2xVhy2T12CLrr9ffy3StQtYdJtRWGejWLVtCd2p` |
| Mock adapter          | `8p7m5zPd9S52CXnEzNu3JBVqraUSwyktF4JWYaaphmEr` |
| Kamino USDC adapter   | `E1bTG9vyE27xVay1oZrZck6idNJZjgotaeFia1P7q9Vb` |
| MarginFi USDC adapter | `CEy21HbuzU6K9WLueUwXtjeiLicVhyMLPtkMHXEzccXu` |
| Jupiter JLP adapter   | `4A1xkP49MszrDZE3Pzzq6a69tNprr3X799NitbAy7RmN` |
| Maple Syrup adapter   | `7Bw1gXZzHz1RFD1FBkqGbAfVoBTz5CYk73QQkzjw8NWf` |
| Drift IF adapter      | `BemVwXxgBf71TXQQrWJH61SR1oodD7tPzX8LFymeB6tM` |

## Requirements

The project targets the bounty stack:

```text
Anchor 0.31.1
Solana CLI 2.2.20
solana-test-validator 2.2.20
Rust
TypeScript
```

The strict fork tests also require a mainnet RPC URL that can clone the protocol
programs and accounts used by the five adapters.

## Quick Start

```bash
npm install
npm run keys:ids:check
npm run typecheck
npm test
```

Build SBF artifacts:

```bash
npm run build
```

Run strict mainnet-fork adapter round trips:

```bash
export MAINNET_RPC_URL="<mainnet-rpc>"
npm run test:fork
```

The fork runner builds the local Anchor programs, starts
`solana-test-validator` from cloned mainnet state, loads the local dispatcher,
registry, and adapter programs with their checked-in ids, then runs the strict
round-trip suite for all five adapters.

Successful fork runs write:

```text
target/syas-mainnet-fork-evidence.json
```

## Evaluator Path

From a Linux checkout with Anchor, Solana CLI, and a mainnet RPC:

```bash
git clone https://github.com/bittee/solana-yield-adapter-standart.git
cd solana-yield-adapter-standart

npm install
npm run keys:ids:check
npm run verify:registry:devnet

export MAINNET_RPC_URL="<mainnet-rpc>"
npm run test:fork
```

Then run the final gate:

```bash
export ANCHOR_WALLET="$HOME/.config/solana/id.json"
export SOLANA_DEVNET_RPC_URL="<devnet-rpc>"
npm run bounty:check
```

## Submission Gate

For bounty submission, run:

```bash
export ANCHOR_WALLET="$HOME/.config/solana/id.json"
export MAINNET_RPC_URL="<mainnet-rpc>"
export SOLANA_DEVNET_RPC_URL="<devnet-rpc>"

npm run bounty:check
```

The gate checks:

- required docs and adapter conformance;
- synchronized program ids across `Anchor.toml`, Rust, SDK, and deploy keys;
- Rust formatting, TypeScript, and local conformance tests;
- Anchor/Solana toolchain availability;
- devnet registry deployment and initialization;
- strict mainnet-fork round trips for Kamino, MarginFi, Jupiter, Maple, and
  Drift.

Include the successful output of `npm run bounty:check`,
`npm run verify:registry:devnet`, and
`target/syas-mainnet-fork-evidence.json` with the final submission.

## Repository Layout

```text
programs/
  dispatcher/                 standard router
  registry/                   governance-gated adapter registry
  adapters/
    kamino-usdc/
    marginfi-usdc/
    jupiter-jlp/
    maple-syrup/
    drift-if/
crates/
  syas-interface/             shared interface, errors, seeds, return data
  syas-adapter-utils/         adapter-side helpers
sdk/
  src/index.ts                ids, PDA helpers, client helpers
scripts/
  bounty/check.ts             final submission gate
  mainnet-fork/run.ts         strict fork runner
  devnet/                     registry deploy and verify scripts
tests/
  mainnet-fork/               account-plan and round-trip tests
docs/
  standard.md                 adapter standard specification
  build-your-own-adapter.md   implementation guide
  architecture.md             design rationale
  protocol-notes.md           protocol account maps
  submission.md               final runbook
```

## Useful Commands

```bash
npm run keys:ids:check          # verify checked-in program ids are synchronized
npm run keys:restore:registry   # restore registry deploy key from local backup
npm run verify:registry:devnet  # verify deployed devnet registry state
npm run test:fork               # run strict mainnet-fork adapter tests
npm run bounty:check            # final bounty submission gate
```

## Documentation

- [Adapter Standard](docs/standard.md)
- [Build Your Own Adapter](docs/build-your-own-adapter.md)
- [Architecture](docs/architecture.md)
- [Protocol Notes](docs/protocol-notes.md)
- [Mainnet-Fork Tests](docs/mainnet-fork-tests.md)
- [Submission Runbook](docs/submission.md)
