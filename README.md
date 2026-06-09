# Solana Yield Adapter Standard

A reference implementation of a minimal Solana yield adapter interface:

```text
deposit()
withdraw()
current_value()
```

Applications integrate with the dispatcher once. The dispatcher validates and
routes. Each adapter owns protocol-specific CPI, custody, accounting, and
valuation.

## Target Stack

- Solana `2.2.20`
- Anchor `0.31.1`
- Rust
- TypeScript

## Required Programs

- Dispatcher Program
- Adapter Registry
- Kamino USDC adapter
- MarginFi USDC adapter
- Jupiter LP/JLP adapter
- Maple Syrup adapter
- Drift Insurance Fund adapter

## Documentation

- [Architecture](docs/architecture.md)
- [Standard](docs/standard.md)
- [Build Your Own Adapter](docs/build-your-own-adapter.md)
- [Protocol Notes](docs/protocol-notes.md)
- [Mainnet Fork Tests](docs/mainnet-fork-tests.md)
- [Reference Repo Lessons](docs/reference-repo-lessons.md)

The docs synthesize the repositories listed in `REPOS.md` and call out problems
to avoid, especially dispatcher/registry coupling, bloated adapter interfaces,
protocol paths that stop at scaffolding, and fork tests that do not execute live
round trips.

## Current Bounty Status

This repository is **not yet bounty-ready as a production protocol integration**.
It is a strong architectural scaffold with compiling on-chain programs, honest
mainnet-fork test planning, and protocol-specific adapter code paths, but the
five required adapters still need live fork validation against real mainnet
protocol state before they should be represented as complete.

| Area                         | Status                                                                         | Evidence                                                                                                                               | Remaining gap                                                                                                                                                                                                          |
| ---------------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dispatcher                   | Compiles; forwards standardized return data from adapters for all three routes | `cargo check --workspace --all-targets`                                                                                                | Needs deployed localnet/fork round-trip proof through Anchor once CLI and validator are available.                                                                                                                     |
| Registry                     | Compiles                                                                       | `cargo check --workspace --all-targets`                                                                                                | Governance-gated registration is implemented, but fork/localnet execution must be shown.                                                                                                                               |
| Mock adapter                 | Compiles; local integration test exists                                        | `tests/mock-adapter.spec.ts`                                                                                                           | Test requires `ANCHOR_PROVIDER_URL` and `ANCHOR_WALLET`; it is skipped instead of faked when no local validator is configured.                                                                                         |
| Kamino USDC adapter          | Compiles                                                                       | `cargo check --workspace --all-targets`                                                                                                | Needs strict mainnet-fork deposit/current_value/withdraw and failure-path execution with live Kamino accounts.                                                                                                         |
| MarginFi USDC adapter        | Compiles                                                                       | `cargo check --workspace --all-targets`                                                                                                | Needs strict mainnet-fork deposit/current_value/withdraw and failure-path execution with live MarginFi accounts.                                                                                                       |
| Jupiter JLP adapter          | Compiles                                                                       | `cargo check --workspace --all-targets`                                                                                                | Needs strict mainnet-fork deposit/current_value/withdraw and failure-path execution with live Jupiter accounts.                                                                                                        |
| Maple Syrup adapter          | Compiles                                                                       | `cargo check --workspace --all-targets`                                                                                                | The implementation currently models this path through Orca/Whirlpool-style accounts documented in protocol notes; this must be reconciled with the exact Maple Syrup production program/account model and fork-tested. |
| Drift Insurance Fund adapter | Compiles                                                                       | `cargo check --workspace --all-targets`                                                                                                | Needs strict mainnet-fork deposit/current_value/withdraw/cooldown settlement and failure-path execution with live Drift accounts.                                                                                      |
| Mainnet fork tests           | Planned and gated                                                              | `npm test` shows strict adapter round trips pending unless `RUN_MAINNET_FORK_TESTS=1`; `npm run test:fork` requires `MAINNET_RPC_URL`. | Requires Anchor CLI, Solana validator tooling, deployable BPF artifacts, and a mainnet RPC URL.                                                                                                                        |
| Documentation                | Present                                                                        | `docs/standard.md`, `docs/architecture.md`, `docs/build-your-own-adapter.md`, and fork/protocol notes                                  | Should be updated again after real fork execution proves exact account lists and protocol behavior.                                                                                                                    |

## Submission Checklist

Before submitting this bounty, do **all** of the following and include logs:

- [x] Keep the standard interface limited to `deposit`, `withdraw`, and
      `current_value`.
- [x] Keep applications routed through the dispatcher and registry rather than
      directly coupling to adapters.
- [x] Compile every Rust crate with `cargo check --workspace --all-targets`.
- [x] Run TypeScript type checking with `npm run typecheck`.
- [x] Keep tests explicit about environment requirements instead of silently
      pretending fork coverage ran.
- [ ] Install/use Anchor `0.31.1` and run `anchor build` successfully.
- [ ] Start a local validator with deployed dispatcher, registry, and mock
      adapter, set `ANCHOR_PROVIDER_URL`/`ANCHOR_WALLET`, and run `npm test` with
      the mock integration tests active.
- [ ] Provide `MAINNET_RPC_URL` and run `npm run test:fork` successfully.
- [ ] Prove deposit, withdraw, current_value, and failure-path tests for each of
      Kamino USDC, MarginFi USDC, Jupiter JLP, Maple Syrup, and Drift Insurance
      Fund against forked mainnet state.
- [ ] Resolve the Maple Syrup account/program mapping against the actual Maple
      production integration instead of relying on Whirlpool-style placeholders.
- [ ] Re-run `cargo fmt --all -- --check`, `cargo test --workspace --all-targets`,
      `npm run typecheck`, `npm test`, `anchor build`, and `npm run test:fork`
      before final submission.

## Useful Commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
npm run typecheck
npm test
npm run bounty:check
MAINNET_RPC_URL=<mainnet-rpc> npm run test:fork
anchor build
```

`npm test` intentionally keeps strict mainnet-fork adapter round trips pending
unless `RUN_MAINNET_FORK_TESTS=1`. The mock adapter integration also requires an
Anchor localnet environment (`ANCHOR_PROVIDER_URL` and `ANCHOR_WALLET`).

`npm run bounty:check` is the submission gate. It runs all local Rust and
TypeScript checks, then requires the Solana CLI, `solana-test-validator`, Anchor
CLI, and `MAINNET_RPC_URL` before executing `anchor build` and strict fork
roundtrips. A bounty submission should include the full successful output of that
command.
