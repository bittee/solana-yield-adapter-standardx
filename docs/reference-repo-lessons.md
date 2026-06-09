# Reference Repo Lessons

The repos in `REPOS.md` are research inputs, not templates. This file records
the engineering decisions to carry forward and the traps to avoid.

## Keep

- Separate programs for dispatcher, registry, and adapters.
- Per-adapter registry entry PDAs keyed by adapter program id.
- A shared interface crate for discriminators, seeds, errors, return-data helpers,
  and account layout helpers.
- Adapter-owned custody PDAs.
- Registry checks inside adapters as well as inside the dispatcher.
- Protocol-specific remaining account order documented next to the adapter code.
- Value math that starts from the units held for the position.
- One reusable conformance suite that every adapter runs.
- Fork scripts that print clone inputs, slot/evidence details, and which mutation
  paths actually executed.

## Avoid

- Combining registry governance into the dispatcher.
- Storing all adapter records in a dispatcher-owned `Vec`; it creates account
  growth pressure and weaker per-adapter identity.
- Adding many user-facing adapter methods and calling that the standard.
- Putting protocol-specific account parsing in the dispatcher.
- Mirroring adapter share balances in dispatcher state; two ledgers eventually
  disagree.
- Guessing a response when return data is absent or malformed.
- Returning protocol-wide liquidity from `current_value`.
- Counting account-plan checks as protocol execution.
- Claiming Maple or Drift support without verifying the actual deployed CPI path.
- Committing deployment keypairs or environment-specific artifacts.
- Using `unwrap`, panics, saturating math, or silent fallbacks where an explicit
  program error is warranted.
- Copying Anchor `1.0.x` / Solana `3.x` manifests into this repo, which targets
  Anchor `0.31.1` and Solana `2.2.20`.

## Repo Notes

### destisAndromeda/SolanaYieldAdapterStandard

Useful:

- Has a shared interface crate and multiple adapter folders.
- Contains dispatcher routing tests and mainnet-fork wiring attempts.

Watch out:

- Uses `adapter_deposit` style methods rather than the minimal method names
  required here.
- Some adapter paths look more like account wiring than settled protocol flows.

### BoozeLee/yield-adapter-standard

Useful:

- Small enough to read quickly as a minimal example.

Watch out:

- Too small for this mission: it does not establish separate dispatcher,
  registry, and five independent adapters.

### dwapetra/solana-yield-adapter-standard

Useful:

- Useful constants, protocol labels, adapter IDs, and account-plan organization.
- Calls out protocol notes and fork completion criteria.

Watch out:

- Registry is embedded in the dispatcher in the inspected implementation.
- The documented adapter ABI includes extra lifecycle/admin methods. For this
  repo, the standard interface should stay at `deposit`, `withdraw`,
  `current_value`.

### btcthirst/Solana-Yield-Adapter-Standard

Useful:

- Clean separation between registry, dispatcher, and adapter programs.
- Useful troubleshooting notes for fork-specific failures.
- Includes a template adapter.

Watch out:

- Contains generated/deployment keypair files in the reference repo.
- Uses a newer stack than this repo's target.

### eternaki/solana-yield-adapter-standard

Useful:

- Has docs, fork tests, and five adapter program directories.

Watch out:

- The dispatcher also owns registry-style whitelist/pause behavior. This repo
  should keep registry governance separate.

### PavloDereniuk/Solana-Yield-Adapter-Standard

Useful:

- Separate interface, registry, dispatcher, and adapter programs.
- Includes runbooks and fork tests.

Watch out:

- Some protocol routes need stronger live verification before they should be
  called complete.

### prettyboyvic/solana-yield-adapter-standard

Useful:

- Strong account derivation and fixture documentation.
- Helpful separation between fixture checks and mutation tests in several
  places.

Watch out:

- Uses a reference adapter approach rather than five fully independent adapters.
- Several tests prove fixture shape rather than a full deposit/value/withdraw
  path.

### eternally-black/Yieldplex

Useful:

- Most complete architecture reference among the inspected repos: interface
  crate, standalone registry, router, independent adapters, SDK helpers, and a
  reusable conformance suite.
- Good examples of adapter-side registry checks and consistent position layouts.
- Useful caveats for Maple exposure and Drift insurance-fund entrypoints.

Watch out:

- Uses Anchor `1.0.2`, not this repo's target Anchor `0.31.1`.
- Adds `initialize_position` and `settle_withdrawal` as interface-level methods.
  This repo should keep the standard interface minimal and model setup/cooldown
  without expanding the required application-facing surface.
