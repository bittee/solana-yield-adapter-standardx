import {
  ADAPTER_SPECS,
  disableAdapter,
  ensureAdapterRegistered,
  forkTestsEnabled,
  ownerUsdcAccount,
  readPositionShares,
  routeDeposit,
  routeWithdraw,
  simulateCurrentValue,
  tokenBalance,
} from "./harness.js";

const describeFork = forkTestsEnabled() ? describe : describe.skip;

describeFork("mainnet-fork adapter roundtrips", () => {
  for (const spec of ADAPTER_SPECS) {
    describe(spec.name, () => {
      it("rejects route while disabled", async () => {
        await ensureAdapterRegistered(spec);
        await disableAdapter(spec);

        let failed = false;
        try {
          await routeDeposit(spec);
        } catch {
          failed = true;
        } finally {
          await ensureAdapterRegistered(spec);
        }

        if (!failed) {
          throw new Error(`${spec.name} accepted a deposit while disabled`);
        }
      });

      it("deposits, reports value, and withdraws through the dispatcher", async () => {
        await ensureAdapterRegistered(spec);
        const ownerTokenAccount = ownerUsdcAccount();
        const balanceBefore = await tokenBalance(ownerTokenAccount);

        await routeDeposit(spec);

        const shares = await readPositionShares(spec);
        if (shares === 0n) {
          throw new Error(`${spec.name} deposit produced zero position shares`);
        }

        const value = await simulateCurrentValue(spec);
        if (value === 0n) {
          throw new Error(
            `${spec.name} current_value returned zero after deposit`,
          );
        }

        await routeWithdraw(spec, shares);

        if (spec.delayedWithdraw) {
          const afterRequestShares = await readPositionShares(spec);
          if (afterRequestShares === 0n) {
            throw new Error(
              `${spec.name} unexpectedly settled during the request phase`,
            );
          }
          return;
        }

        const balanceAfter = await tokenBalance(ownerTokenAccount);
        if (balanceAfter <= balanceBefore - spec.amountIn) {
          throw new Error(
            `${spec.name} withdraw did not return any base asset`,
          );
        }
      });
    });
  }
});

describe("mainnet-fork adapter roundtrips guard", () => {
  it("keeps strict fork execution explicit", () => {
    if (forkTestsEnabled()) {
      return;
    }
    if (process.env.MAINNET_RPC_URL) {
      throw new Error(
        "set RUN_MAINNET_FORK_TESTS=1 to execute strict fork roundtrips",
      );
    }
  });
});
