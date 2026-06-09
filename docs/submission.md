# Bounty Submission Runbook

This repository should be submitted only after the commands below pass in a real
Solana/Anchor environment. The submission must include the public GitHub URL,
the devnet registry address, and the full successful output from the final gate.

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
Before deploying, ensure `target/deploy/registry-keypair.json` exists and its
public key equals the registry id in `programs/registry/src/lib.rs`,
`Anchor.toml`, and `sdk/src/index.ts`. If you generate a new program keypair,
update all three places together before building and submitting.

```bash
npm run deploy:registry:devnet
npm run verify:registry:devnet
```

Record these values in the submission:

- registry program id
- registry PDA
- governance address
- deployment and initialization signatures

## Final Gate

Run the full gate from a clean checkout:

```bash
npm install
MAINNET_RPC_URL=<mainnet-rpc> npm run bounty:check
```

This gate is intentionally strict. It fails if the local machine lacks the
required CLI tools, if deployment keypairs do not match the checked-in program
ids, if the devnet registry is not deployed and initialized, or if the
mainnet-fork round trips do not execute.

On success, include:

- complete terminal output from `npm run bounty:check`
- `target/syas-mainnet-fork-evidence.json`
- devnet registry verification output

## What Not To Claim

Do not submit account-plan or skipped tests as adapter correctness evidence.
Judging correctness is based on all five adapters passing the strict fork
round-trip suite:

- Kamino USDC
- MarginFi USDC
- Jupiter JLP
- Maple Syrup exposure
- Drift Insurance Fund

If Maple or Drift fail because the live deployed protocol does not expose the
expected CPI path, report that exact fork log as an upstream blocker instead of
replacing the route with a mock.
