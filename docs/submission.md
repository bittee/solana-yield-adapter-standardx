# Bounty Submission Runbook

Use this runbook for the final bounty submission package. The submission should
include the public GitHub URL, the verified devnet registry address, strict
mainnet-fork evidence, and the full successful output from the final gate.

## Required Environment

- Anchor CLI `0.31.1`
- Solana CLI and `solana-test-validator` `2.2.20`
- Rust with `rustfmt` and `clippy`
- Node.js with dependencies installed by `npm install`
- `MAINNET_RPC_URL` pointing at a mainnet RPC that supports cloning all required
  protocol accounts
- a funded devnet wallet in `ANCHOR_WALLET` or `~/.config/solana/id.json`

## One-Time Devnet Registry Deployment

The bounty requires the registry contract to be deployed to devnet.
Before deploying, ensure the registry program keypair exists locally and its
public key equals the registry id in `programs/registry/src/lib.rs`,
`Anchor.toml`, and `sdk/src/index.ts`.

This workspace builds deployable SBF artifacts with Anchor's no-IDL path:

```bash
anchor build --no-idl
```

If you do not already have the registry program keypair, generate a new local
set and synchronize all checked-in public ids:

```bash
npm run keys:sync
```

If you already have `program-keypairs/registry-keypair.json`, restore it into
Anchor's deploy directory:

```bash
npm run keys:restore:registry
npm run keys:verify:registry
```

Do not commit `program-keypairs/` or `target/deploy/*-keypair.json`.

```bash
npm run deploy:registry:devnet
npm run verify:registry:devnet
```

Record these values in the submission:

- registry program id: `BWTrd2xVhy2T12CLrr9ffy3StQtYdJtRWGejWLVtCd2p`
- registry PDA: `33qVn5v9GNtJJTpLCy2AeFvvbU3TuY1HWZgZv7nmntjW`
- governance address
- deployment and initialization signatures

## Final Gate

Run the full gate from a clean checkout:

```bash
npm install
npm run keys:ids:check
anchor build --no-idl
MAINNET_RPC_URL=<mainnet-rpc> npm run bounty:check
```

This gate is intentionally strict. It verifies the local toolchain, checked-in
program ids, devnet registry state, required documentation, conformance tests,
and the mainnet-fork adapter round trips.

On success, include:

- complete terminal output from `npm run bounty:check`
- `target/syas-mainnet-fork-evidence.json`
- devnet registry verification output

## Evidence Checklist

Submit strict fork output, not account-plan output, as adapter correctness
evidence. Judging correctness is based on all five adapters passing the strict
fork round-trip suite:

- Kamino USDC
- MarginFi USDC
- Jupiter JLP
- Maple Syrup exposure
- Drift Insurance Fund

The final package should include:

- public GitHub repository URL
- registry program id: `BWTrd2xVhy2T12CLrr9ffy3StQtYdJtRWGejWLVtCd2p`
- registry PDA: `33qVn5v9GNtJJTpLCy2AeFvvbU3TuY1HWZgZv7nmntjW`
- governance address: `4vhi8SvYFGnaq2xhr8ocjf4HcEHqsSqM9swT56SNPNoh`
- `npm run verify:registry:devnet` output
- `npm run test:fork` output
- `npm run bounty:check` output
- `target/syas-mainnet-fork-evidence.json`
