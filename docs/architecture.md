# Architecture

This repository defines a reference shape for Solana yield integrations. A client
talks to one public router, while each yield venue is isolated behind its own
adapter program. The adapter is the only layer that understands venue accounts,
venue CPI, share bookkeeping, and valuation.

```text
Application
  -> Dispatcher Program
    -> Adapter Registry
    -> Adapter Program
      -> Yield Protocol
```

## Programs

### Dispatcher

The dispatcher is deliberately narrow. Its code should read like routing and
admission control, not like a protocol integration.

Responsibilities:

- Load the registry entry for the requested adapter.
- Verify the adapter is approved and enabled.
- Verify the requested asset mint matches the registry entry.
- Build the adapter instruction data for `deposit`, `withdraw`, or
  `current_value`.
- Forward the standard account prefix and adapter-specific remaining accounts.
- Re-emit or normalize adapter responses.
- Treat missing or malformed adapter return data as an error.

Non-responsibilities:

- No protocol account parsing.
- No protocol-specific CPI.
- No yield or exchange-rate calculations.
- No custody of user funds.
- No hardcoded knowledge of Kamino, MarginFi, Jupiter, Maple, or Drift.
- No mirrored share accounting that can diverge from adapter-owned state.

The main risk for this architecture is router expansion: once the dispatcher
starts listing adapters, keeping balances, or branching on protocol type, every
adapter correction becomes a router migration. This implementation keeps that
pressure out of the dispatcher.

### Registry

The registry is the governance-controlled adapter directory. It never owns
strategy assets or user token accounts.

Responsibilities:

- Initialize registry governance.
- Register adapter entries through governance.
- Enable and disable adapter entries.
- Store immutable or slowly-changing adapter metadata:
  - adapter program id
  - supported base mint
  - protocol label
  - status
  - risk tier
  - metadata URI/hash when used
- Support two-step governance transfer.

Implementation rule:

- Use one registry PDA for global configuration.
- Use one adapter-entry PDA per adapter program id.
- Do not store the full adapter list as a growable `Vec` inside dispatcher
  state.

The separate-entry model avoids account reallocation pressure, gives each
adapter a canonical address, and lets the dispatcher validate one small account
per route.

### Adapters

Each adapter is an independent program. No adapter may depend on another adapter.

Required adapters:

- Kamino USDC
- MarginFi USDC
- Jupiter LP/JLP
- Maple Syrup
- Drift Insurance Fund

Responsibilities:

- Implement the standard adapter interface:
  - `deposit`
  - `withdraw`
  - `current_value`
- Re-check the registry entry inside every adapter instruction, so bypassing the
  dispatcher does not bypass governance.
- Validate signer requirements, account ownership, PDAs, mints, and protocol
  account relationships.
- Custody position assets under adapter-owned PDAs, not under dispatcher PDAs.
- Perform checked arithmetic.
- Return explicit errors instead of panicking.

Adapters may define setup or admin instructions for their own deployment needs.
Those instructions are not part of the application-facing standard. The standard
surface remains the three user operations unless `docs/standard.md` is revised.

## Shared Interface Crate

The implementation should include a small Rust crate shared by dispatcher,
registry, adapters, and tests. It is not an on-chain program.

Suggested contents:

- Instruction discriminators for `deposit`, `withdraw`, `current_value`.
- Standard account seeds.
- Shared errors.
- Shared events.
- Return-data helpers.
- Adapter status and protocol labels.
- Account layout helpers for common position/ticket state if used.

A single crate prevents duplicated discriminators, inconsistent PDA seeds, and
slightly different account layouts from creeping into each program.

## Account Model

Dispatcher calls start with the common accounts needed for routing, ownership,
and token movement. Everything after that boundary belongs to the adapter and
must be validated by adapter code.

Suggested standard prefix:

```text
0. position account
1. position authority PDA
2. base mint
3. adapter base-token vault
4. owner signer
5. owner base-token account
6. registry entry
7. token program
8. system program
9+. protocol-specific remaining accounts
```

The dispatcher validates only what is protocol-agnostic:

- adapter registry entry
- adapter status
- adapter program id
- base mint
- required signer

The adapter validates the protocol-specific tail:

- account count
- exact program ids
- account owners
- PDA derivations
- token mints
- token account authorities
- market/reserve/bank/pool relationships
- oracle freshness where relevant

## Position Custody

The dispatcher should not be a vault authority. A position belongs to the target
adapter, usually with PDAs such as:

```text
position = ["position", owner, base_mint]
position_authority = ["position_authority", position]
vault = ["vault", position, base_mint]
```

This keeps failures local. A bug in one adapter should not put dispatcher-owned
vaults or another adapter's positions at risk.

## Direct Calls

Adapters must perform their own registry check. A Solana transaction can target
an adapter program without involving the dispatcher.

The pattern should be:

```rust
let entry = registry::load_adapter_entry(registry_entry, &crate::ID)?;
require!(entry.status == AdapterStatus::Enabled, ErrorCode::AdapterDisabled);
require_keys_eq!(entry.base_mint, base_mint.key(), ErrorCode::MintMismatch);
```

## Response Model

The dispatcher should expose consistent outcomes. Recommended response behavior:

- `deposit` emits deposited amount and position units received.
- `withdraw` emits position units redeemed and assets returned.
- `current_value` returns a `u64` value in base-mint minor units through Solana
  return data and emits the same value.

For delayed protocols, `withdraw` may store a pending exit and return no payout
until a later `withdraw` can finish the protocol cooldown. That keeps cooldown
handling inside the adapter without adding a fourth standard method.

## Version Targets

This repository targets:

- Solana `2.2.20`
- Anchor `0.31.1`
- Rust
- TypeScript

All checked-in programs and scripts are pinned to these versions for build and
test consistency.
