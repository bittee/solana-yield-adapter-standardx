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
- [Bounty Submission Runbook](docs/submission.md)

The docs synthesize the repositories listed in `REPOS.md` and call out problems
to avoid, especially dispatcher/registry coupling, bloated adapter interfaces,
protocol paths that stop at scaffolding, and fork tests that do not execute live
round trips.

## Bounty Readiness

This repository contains the required dispatcher, registry, five adapter
programs, fork harness, SDK helpers, and developer documentation. Final bounty
submission still depends on two external proofs that cannot be faked locally:

- registry deployed and initialized on devnet
- all five adapters passing strict mainnet-fork round trips against live
  protocol state

Use `docs/submission.md` and `npm run bounty:check` as the final submission
gate. Do not submit skipped tests or account-plan tests as fork-pass evidence.

| Area                         | Status                                                                                   | Evidence                                                                                              | Remaining gap                                                                                                                                              |
| ---------------------------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dispatcher                   | Implemented; forwards standardized return data from adapters for all three routes        | `cargo check --workspace --all-targets` and strict fork gate                                          | Needs successful `npm run bounty:check` output in an environment with Anchor/Solana CLI.                                                                   |
| Registry                     | Implemented with governance-gated registration, enable, disable, and governance transfer | `cargo check --workspace --all-targets`, `npm run verify:registry:devnet`                             | Must be deployed and initialized on devnet before submission.                                                                                              |
| Mock adapter                 | Compiles; local integration test exists                                                  | `tests/mock-adapter.spec.ts`                                                                          | Test requires `ANCHOR_PROVIDER_URL` and `ANCHOR_WALLET`; it is skipped instead of faked when no local validator is configured.                             |
| Kamino USDC adapter          | Compiles                                                                                 | `cargo check --workspace --all-targets`                                                               | Needs strict mainnet-fork deposit/current_value/withdraw and failure-path execution with live Kamino accounts.                                             |
| MarginFi USDC adapter        | Compiles                                                                                 | `cargo check --workspace --all-targets`                                                               | Needs strict mainnet-fork deposit/current_value/withdraw and failure-path execution with live MarginFi accounts.                                           |
| Jupiter JLP adapter          | Compiles                                                                                 | `cargo check --workspace --all-targets`                                                               | Needs strict mainnet-fork deposit/current_value/withdraw and failure-path execution with live Jupiter accounts.                                            |
| Maple Syrup adapter          | Compiles as a syrupUSDC exposure route                                                   | `cargo check --workspace --all-targets`; `docs/protocol-notes.md`                                     | Needs strict mainnet-fork deposit/current_value/withdraw and failure-path execution through the documented syrupUSDC liquidity route.                      |
| Drift Insurance Fund adapter | Compiles                                                                                 | `cargo check --workspace --all-targets`                                                               | Needs strict mainnet-fork deposit/current_value/withdraw/cooldown settlement and failure-path execution with live Drift accounts.                          |
| Mainnet fork tests           | Implemented as strict gated round trips                                                  | `tests/mainnet-fork/roundtrip.spec.ts`; `npm run test:fork` requires `MAINNET_RPC_URL`                | Requires Anchor CLI, Solana validator tooling, deployable SBF artifacts, and a mainnet RPC URL. Local fork loading does not require program keypair files. |
| Documentation                | Present                                                                                  | `docs/standard.md`, `docs/architecture.md`, `docs/build-your-own-adapter.md`, and fork/protocol notes | Should be updated again after real fork execution proves exact account lists and protocol behavior.                                                        |

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
- [ ] Install/use Anchor `0.31.1` and run `anchor build --no-idl`
      successfully.
- [ ] Run `npm run keys:ids:check` before submission. If the registry program id
      must change, run `npm run keys:sync` and commit the synchronized public id
      changes across Anchor/Rust/SDK/docs.
- [ ] Deploy and initialize the registry on devnet with
      `npm run deploy:registry:devnet`, then verify with
      `npm run verify:registry:devnet`.
- [ ] Start a local validator with deployed dispatcher, registry, and mock
      adapter, set `ANCHOR_PROVIDER_URL`/`ANCHOR_WALLET`, and run `npm test` with
      the mock integration tests active.
- [ ] Provide `MAINNET_RPC_URL` and run `npm run test:fork` successfully.
- [ ] Prove deposit, withdraw, current_value, and failure-path tests for each of
      Kamino USDC, MarginFi USDC, Jupiter JLP, Maple Syrup, and Drift Insurance
      Fund against forked mainnet state.
- [ ] For Maple, submit it explicitly as a syrupUSDC exposure adapter and include
      successful fork logs for the documented liquidity route.
- [ ] Re-run `cargo fmt --all -- --check`, `cargo test --workspace --all-targets`,
      `npm run typecheck`, `npm test`, `anchor build --no-idl`, and
      `npm run test:fork` before final submission.

## Useful Commands

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
npm run typecheck
npm test
npm run keys:ids:check
npm run keys:sync
npm run keys:restore:registry
npm run keys:verify:registry
npm run keys:restore
npm run keys:verify
npm run deploy:registry:devnet
npm run verify:registry:devnet
npm run bounty:check
MAINNET_RPC_URL=<mainnet-rpc> npm run test:fork
anchor build --no-idl
```

`npm test` intentionally keeps strict mainnet-fork adapter round trips pending
unless `RUN_MAINNET_FORK_TESTS=1`. The mock adapter integration also requires an
Anchor localnet environment (`ANCHOR_PROVIDER_URL` and `ANCHOR_WALLET`).

`npm run keys:ids:check` verifies that program ids are synchronized across
`Anchor.toml`, `declare_id!`, and `sdk/src/index.ts`. `npm run keys:sync`
generates local program keypairs under ignored `program-keypairs/`, restores them
to `target/deploy/`, and updates the checked-in public ids. This is needed only
when you do not already have the registry program keypair for devnet deployment;
strict fork tests load local `.so` files by public id and do not require program
keypair files.

`npm run bounty:check` is the submission gate. It runs all local Rust and
TypeScript checks, then requires the Solana CLI, `solana-test-validator`, Anchor
CLI, and `MAINNET_RPC_URL` before executing `anchor build --no-idl` and strict fork
roundtrips. A bounty submission should include the full successful output of that
command.
