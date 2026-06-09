# AGENTS.md

## Mission

Build a reference implementation of a Solana Yield Adapter Standard.
Use clean architecture. Easy to extend and read.
Use repos provided in REPOS.md as references, make sure you don't repeat their mistakes.

Deliverables:

* Dispatcher Program
* Adapter Registry
* 5 Yield Adapters
* Mainnet Fork Tests
* Adapter Specification
* Build Your Own Adapter Guide

Stack:

* Solana 2.2.20
* Anchor 0.31.1
* Rust
* TypeScript

---

## Architecture

Applications interact only with the Dispatcher.

```text
App
  ↓
Dispatcher
  ↓
Registry
  ↓
Adapter
```

Adapters implement protocol-specific logic.

Dispatcher implements protocol-agnostic routing.

Registry stores approved adapters.

---

## Standard Interface

Every adapter must expose:

```rust
deposit(...)
withdraw(...)
current_value(...)
```

Definitions:

* deposit -> allocate assets into strategy
* withdraw -> redeem assets from strategy
* current_value -> return current position value

Keep interface minimal.

Do not add methods unless absolutely required.

---

## Required Adapters

1. Kamino USDC
2. MarginFi USDC
3. Jupiter LP
4. Maple Syrup
5. Drift Insurance Fund

Each adapter must be independently testable.

No adapter may depend on another adapter.

---

## Dispatcher Rules

Dispatcher:

* validates adapter
* routes requests
* standardizes responses

Dispatcher must not:

* contain strategy logic
* contain protocol logic
* perform yield calculations

---

## Registry Rules

Registry:

* registers adapters
* enables adapters
* disables adapters

Registration must be governance-gated.

Registry never holds user funds.

---

## Security Rules

Always validate:

* signer requirements
* account ownership
* PDA derivations
* token mints
* account relationships

Use checked arithmetic.

Avoid panic-based logic.

Prefer explicit errors.

Assume all external accounts are untrusted.

---

## Testing Rules

Every adapter requires:

* deposit test
* withdraw test
* current_value test
* failure-path test

Run tests against mainnet fork whenever possible.

Prefer real protocol state over mocks.

---

## Documentation Rules

Maintain:

```text
docs/
├── standard.md
├── architecture.md
└── build-your-own-adapter.md
```

Primary goal:

A Solana developer should be able to build a new adapter in less than one day.

---

## Code Quality

Prefer:

* small functions
* explicit types
* reusable helpers
* clear naming

Avoid:

* duplicated logic
* giant files
* magic constants
* premature optimization

---

## Repository Structure

```text
programs/
├── dispatcher
├── registry
└── adapters

tests/
docs/
sdk/
scripts/
```

---

## Definition of Done

Complete only when:

* dispatcher works
* registry works
* all 5 adapters work
* all fork tests pass
* documentation is written
* adapter guide is written
* code builds cleanly
* formatting passes
* no unresolved TODOs remain

```
```
