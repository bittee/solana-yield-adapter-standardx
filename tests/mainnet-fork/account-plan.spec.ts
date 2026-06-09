import { PublicKey } from "@solana/web3.js";
import {
  adapterEntryPda,
  DRIFT_IF_ADAPTER_PROGRAM_ID,
  JUPITER_JLP_ADAPTER_PROGRAM_ID,
  KAMINO_USDC_ADAPTER_PROGRAM_ID,
  MAPLE_SYRUP_ADAPTER_PROGRAM_ID,
  MARGINFI_USDC_ADAPTER_PROGRAM_ID,
  REGISTRY_PROGRAM_ID,
  USDC_MINT,
} from "../../sdk/src/index.js";
import {
  DRIFT,
  driftInsuranceFundVault,
  driftSigner,
  driftSpotMarket,
  driftSpotMarketVault,
  driftState,
  EXTERNAL_PROGRAMS,
  JUPITER,
  KAMINO,
  kaminoMarketAuthority,
  MAINNET_CLONES,
  MAPLE,
  mapleOracle,
  MARGINFI,
  PATCHED_DOVES_ACCOUNTS,
} from "../../scripts/mainnet-fork/accounts.js";

describe("mainnet-fork account plan", () => {
  it("uses the expected adapter program ids", () => {
    assertKey(KAMINO.adapter, KAMINO_USDC_ADAPTER_PROGRAM_ID);
    assertKey(MARGINFI.adapter, MARGINFI_USDC_ADAPTER_PROGRAM_ID);
    assertKey(JUPITER.adapter, JUPITER_JLP_ADAPTER_PROGRAM_ID);
    assertKey(MAPLE.adapter, MAPLE_SYRUP_ADAPTER_PROGRAM_ID);
    assertKey(DRIFT.adapter, DRIFT_IF_ADAPTER_PROGRAM_ID);
  });

  it("derives registry entries from the active registry id", () => {
    assertKey(
      adapterEntryPda(KAMINO.adapter),
      PublicKey.findProgramAddressSync(
        [Buffer.from("adapter_entry"), KAMINO.adapter.toBuffer()],
        REGISTRY_PROGRAM_ID,
      )[0],
    );
    assertKey(
      adapterEntryPda(DRIFT.adapter),
      PublicKey.findProgramAddressSync(
        [Buffer.from("adapter_entry"), DRIFT.adapter.toBuffer()],
        REGISTRY_PROGRAM_ID,
      )[0],
    );
  });

  it("derives protocol PDAs used in adapter tails", () => {
    assertKey(
      kaminoMarketAuthority(),
      new PublicKey("9DrvZvyWh1HuAoZxvYWMvkf2XCzryCpGgHqrMjyDWpmo"),
    );
    assertKey(
      mapleOracle(),
      new PublicKey("H7j5FQpwTUMwxrWeuyrLr5Z9oHsPFiaRqNaERVsuE1c8"),
    );
    assertKey(
      driftState(),
      new PublicKey("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN"),
    );
    assertKey(
      driftSigner(),
      new PublicKey("JCNCMFXo5M5qwUPg2Utu1u6YWp3MbygxqBsBeXXJfrw"),
    );
    assertKey(
      driftSpotMarket(),
      new PublicKey("6gMq3mRCKf8aP3ttTyYhuijVZ2LGi14oDsBbkgubfLB3"),
    );
    assertKey(
      driftSpotMarketVault(),
      new PublicKey("GXWqPpjQpdz7KZw9p7f5PX2eGxHAhvpNXiviFkAB8zXg"),
    );
    assertKey(
      driftInsuranceFundVault(),
      new PublicKey("2CqkQvYxp9Mq4PqLvAQ1eryYxebUh4Liyn5YMDtXsYci"),
    );
  });

  it("contains every required clone once", () => {
    const cloneSet = new Set(MAINNET_CLONES.map((key) => key.toBase58()));
    const required = [
      USDC_MINT,
      KAMINO.reserve,
      KAMINO.lendingMarket,
      KAMINO.reserveLiquiditySupply,
      KAMINO.reserveCollateralMint,
      MARGINFI.group,
      MARGINFI.usdcBank,
      MARGINFI.liquidityVault,
      MARGINFI.oracle,
      JUPITER.pool,
      JUPITER.jlpMint,
      JUPITER.usdcCustody,
      JUPITER.custodyTokenAccount,
      MAPLE.syrupMint,
      MAPLE.whirlpool,
      MAPLE.tokenVaultA,
      MAPLE.tokenVaultB,
      MAPLE.buyTickArray1,
      MAPLE.sellTickArray1,
      driftState(),
      driftSpotMarket(),
      driftSpotMarketVault(),
      driftInsuranceFundVault(),
    ];

    for (const key of required) {
      if (!cloneSet.has(key.toBase58())) {
        throw new Error(`missing clone ${key.toBase58()}`);
      }
    }

    if (cloneSet.size !== MAINNET_CLONES.length) {
      throw new Error("clone list contains duplicate pubkeys");
    }
  });

  it("separates upgradeable programs from cloned data accounts", () => {
    const programSet = new Set(EXTERNAL_PROGRAMS.map((key) => key.toBase58()));
    for (const clone of MAINNET_CLONES) {
      if (programSet.has(clone.toBase58())) {
        throw new Error(
          `program id appears in clone list: ${clone.toBase58()}`,
        );
      }
    }
  });

  it("knows which Doves accounts need timestamp patching", () => {
    const patched = new Set(
      PATCHED_DOVES_ACCOUNTS.map((key) => key.toBase58()),
    );
    for (const key of JUPITER.dovesAgPrices) {
      if (!patched.has(key.toBase58())) {
        throw new Error(`missing Doves patch account ${key.toBase58()}`);
      }
    }
  });
});

function assertKey(actual: PublicKey, expected: PublicKey): void {
  if (!actual.equals(expected)) {
    throw new Error(
      `expected ${expected.toBase58()}, got ${actual.toBase58()}`,
    );
  }
}
