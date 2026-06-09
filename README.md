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

## Status

This repository is currently in documentation/scaffolding stage. The implementation
should be considered complete only when the dispatcher, registry, all five
adapters, fork tests, and docs are present and build cleanly.
