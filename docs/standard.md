# Solana Yield Adapter Standard

Version: `SYAS-1`

The standard is the contract between applications, the dispatcher, and every
adapter. It intentionally covers only the three actions an application needs for
a yield position: enter, exit, and value.

## Interface

Every adapter must provide these user-facing methods:

```rust
deposit(...)
withdraw(...)
current_value(...)
```

Definitions:

- `deposit` allocates base assets into the adapter's strategy.
- `withdraw` redeems or begins redeeming position units back into base assets.
- `current_value` reports the current position value in base-asset minor units.

Do not widen the standard to include setup, governance, protocol migration, or
debugging calls. An adapter program can have private operational instructions,
but an application integrating through the dispatcher should only need the three
standard calls.

## Instruction Semantics

### deposit

Inputs:

- `amount: u64`
- `min_position_out: u64`

Required behavior:

- Validate the registry entry and base mint.
- Validate owner signer and token accounts.
- Move `amount` of the base asset into the adapter custody path.
- CPI into the protocol or otherwise acquire the strategy position.
- Record the position units received.
- Require units received to satisfy `min_position_out`.
- Emit a deposit event.

### withdraw

Inputs:

- `position_amount: u64`
- `min_amount_out: u64`

Required behavior:

- Validate the registry entry and base mint.
- Validate the owner and position relationship.
- Redeem `position_amount` from the strategy when possible.
- Transfer redeemed base assets to the owner token account.
- Require assets out to satisfy `min_amount_out` when the withdrawal settles.
- Emit either a settled withdrawal event or a pending withdrawal event.

Some protocols have cooldowns. For those protocols, `withdraw` should be
stateful and idempotent:

- First call requests the protocol withdrawal and records pending state.
- Later call, after the cooldown condition is satisfied, settles the withdrawal.
- Calls before the cooldown is ready return an explicit locked error.

This preserves the three-method integration while still supporting protocols
that require a waiting period.

### current_value

Inputs:

- No amount input.

Required behavior:

- Validate the registry entry and base mint.
- Read live protocol state required to value the caller's position.
- Return the value as `u64` base-asset minor units through Solana return data.
- Emit a value event.

Rules:

- Compute the caller's claim from their recorded units and the venue's live
  conversion state.
- Venue-wide liquidity, TVL, reserve totals, and pool balances can help validate
  inputs, but they are not the position value by themselves.
- Use checked arithmetic and widen intermediate products before division.
- If a required rate, oracle, or market account cannot be trusted at the current
  slot, return an explicit error instead of estimating.
- A stored value is only acceptable when the adapter verifies it is still fresh
  under that venue's rules.

## Registry Status

Recommended adapter status lifecycle:

```text
Registered -> Enabled -> Disabled
```

Optional metadata states such as `Deprecated` may be added, but user flows only
care whether the adapter is enabled.

The dispatcher and adapter both reject disabled adapters.

## Capabilities

SYAS-1 treats all approved adapters as implementations of all three methods.
Capability bits may be stored for off-chain discovery, but they should not split
the application-facing API into incompatible variants.

If capability bits are stored, use:

```text
1 << 0 deposit
1 << 1 withdraw
1 << 2 current_value
```

## Errors

The implementation should define explicit errors for at least:

- `Unauthorized`
- `AdapterNotRegistered`
- `AdapterDisabled`
- `InvalidRegistryEntry`
- `InvalidRemainingAccounts`
- `MintMismatch`
- `InvalidTokenAccount`
- `InvalidPda`
- `SlippageExceeded`
- `WithdrawalPending`
- `WithdrawalLocked`
- `OracleStale`
- `MathOverflow`
- `ProtocolError`

Avoid `unwrap`, unchecked indexing, and panic-based logic in on-chain code.

## Events

Suggested standard events:

```rust
AdapterRouted {
    adapter: Pubkey,
    action: AdapterAction,
}

Deposited {
    owner: Pubkey,
    adapter: Pubkey,
    amount_in: u64,
    position_out: u64,
}

Withdrawn {
    owner: Pubkey,
    adapter: Pubkey,
    position_in: u64,
    amount_out: u64,
}

WithdrawalPending {
    owner: Pubkey,
    adapter: Pubkey,
    position_in: u64,
    unlock_ts: i64,
}

ValueReported {
    owner: Pubkey,
    adapter: Pubkey,
    value: u64,
}
```

Events are observability, not authorization. They should make tests and
integrations easier to inspect, while account constraints and explicit checks
remain the authority.

## Conformance

An adapter conforms when:

- It exposes `deposit`, `withdraw`, and `current_value`.
- It can be called through the dispatcher without dispatcher changes.
- It rejects direct calls when its registry entry is disabled.
- It validates all untrusted accounts.
- It respects slippage limits.
- It reports `current_value` in the base mint's minor units.
- It has deposit, withdraw, current_value, and failure-path tests.

## Standard Account Prefix

Dispatcher routes use the same first nine accounts for every adapter:

```text
0. position
1. position authority PDA
2. base mint
3. adapter base-token vault
4. owner signer
5. owner base-token account
6. registry entry
7. token program
8. system program
```

Protocol-specific accounts start after this prefix. Adapters may add receipt
vaults, protocol markets, or protocol programs, but they must preserve the
meaning of the standard prefix. The base-token vault is always the adapter-owned
vault for the registered base mint, even when the venue receipt token is a
different mint.
