import * as anchor from "@coral-xyz/anchor";
import {
  AccountMeta,
  ComputeBudgetProgram,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { createHash } from "node:crypto";
import {
  adapterEntryPda,
  adapterVaultPda,
  DISPATCHER_PROGRAM_ID,
  positionAuthorityPda,
  positionPda,
  receiptVaultPda,
  registryPda,
  REGISTRY_PROGRAM_ID,
  USDC_MINT,
} from "../../sdk/src/index.js";
import {
  DRIFT,
  driftInsuranceFundStake,
  driftInsuranceFundVault,
  driftSigner,
  driftSpotMarket,
  driftSpotMarketVault,
  driftState,
  driftUserStats,
  JUPITER,
  KAMINO,
  kaminoMarketAuthority,
  MAPLE,
  mapleOracle,
  MARGINFI,
  marginfiAccount,
  SYSVAR_ACCOUNTS,
  TOKEN_PROGRAM_ID,
} from "../../scripts/mainnet-fork/accounts.js";

let cachedProvider: anchor.AnchorProvider | undefined;

export function forkProvider(): anchor.AnchorProvider {
  if (!cachedProvider) {
    cachedProvider = anchor.AnchorProvider.env();
    anchor.setProvider(cachedProvider);
  }
  return cachedProvider;
}

const registryAccount = registryPda();

export type ForkAdapterSpec = {
  name: string;
  adapterProgram: PublicKey;
  protocolVariant: number;
  receiptMint?: PublicKey;
  amountIn: bigint;
  tail(ctx: RouteContext, action: RouteAction): AccountMeta[];
  delayedWithdraw?: boolean;
};

type RouteAction = "deposit" | "withdraw" | "current_value";

export type RouteContext = {
  owner: PublicKey;
  ownerTokenAccount: PublicKey;
  position: PublicKey;
  positionAuthority: PublicKey;
  adapterVault: PublicKey;
  receiptVault?: PublicKey;
};

export function forkTestsEnabled(): boolean {
  return process.env.RUN_MAINNET_FORK_TESTS === "1";
}

export function owner(): PublicKey {
  return forkProvider().wallet.publicKey;
}

export function ownerUsdcAccount(): PublicKey {
  return getAssociatedTokenAddressSync(USDC_MINT, owner(), false);
}

export function routeContext(spec: ForkAdapterSpec): RouteContext {
  const position = positionPda(spec.adapterProgram, owner(), USDC_MINT);
  const positionAuthority = positionAuthorityPda(spec.adapterProgram, position);
  return {
    owner: owner(),
    ownerTokenAccount: ownerUsdcAccount(),
    position,
    positionAuthority,
    adapterVault: adapterVaultPda(spec.adapterProgram, position, USDC_MINT),
    receiptVault: spec.receiptMint
      ? receiptVaultPda(spec.adapterProgram, position, spec.receiptMint)
      : undefined,
  };
}

export async function ensureAdapterRegistered(
  spec: ForkAdapterSpec,
): Promise<void> {
  const adapterEntry = adapterEntryPda(spec.adapterProgram);
  const tx = new Transaction();

  const provider = forkProvider();
  if (!(await provider.connection.getAccountInfo(registryAccount))) {
    tx.add(
      new TransactionInstruction({
        programId: REGISTRY_PROGRAM_ID,
        keys: [
          { pubkey: registryAccount, isSigner: false, isWritable: true },
          { pubkey: owner(), isSigner: true, isWritable: true },
          {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ],
        data: concatBytes(
          discriminator("initialize_registry"),
          owner().toBytes(),
        ),
      }),
    );
  }

  if (!(await provider.connection.getAccountInfo(adapterEntry))) {
    tx.add(
      new TransactionInstruction({
        programId: REGISTRY_PROGRAM_ID,
        keys: [
          { pubkey: registryAccount, isSigner: false, isWritable: true },
          { pubkey: adapterEntry, isSigner: false, isWritable: true },
          { pubkey: owner(), isSigner: true, isWritable: true },
          {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ],
        data: concatBytes(
          discriminator("register_adapter"),
          spec.adapterProgram.toBytes(),
          USDC_MINT.toBytes(),
          Uint8Array.from([spec.protocolVariant]),
          Uint8Array.from([0]),
        ),
      }),
    );
  }

  tx.add(enableAdapterInstruction(spec));

  if (tx.instructions.length > 0) {
    await provider.sendAndConfirm(tx, []);
  }
}

export async function disableAdapter(spec: ForkAdapterSpec): Promise<void> {
  const provider = forkProvider();
  await provider.sendAndConfirm(
    new Transaction().add(
      new TransactionInstruction({
        programId: REGISTRY_PROGRAM_ID,
        keys: [
          { pubkey: registryAccount, isSigner: false, isWritable: false },
          {
            pubkey: adapterEntryPda(spec.adapterProgram),
            isSigner: false,
            isWritable: true,
          },
          { pubkey: owner(), isSigner: true, isWritable: false },
        ],
        data: discriminator("disable_adapter"),
      }),
    ),
    [],
  );
}

export async function routeDeposit(spec: ForkAdapterSpec): Promise<void> {
  await sendRoute(spec, "route_deposit", [u64(spec.amountIn), u64(0n)]);
}

export async function routeWithdraw(
  spec: ForkAdapterSpec,
  positionAmount: bigint,
): Promise<void> {
  await sendRoute(spec, "route_withdraw", [u64(positionAmount), u64(0n)]);
}

export async function simulateCurrentValue(
  spec: ForkAdapterSpec,
): Promise<bigint> {
  const provider = forkProvider();
  const ctx = routeContext(spec);
  const tx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    routeInstruction(spec, "route_current_value", [], ctx),
  );
  tx.feePayer = owner();
  tx.recentBlockhash = (
    await provider.connection.getLatestBlockhash()
  ).blockhash;
  const signed = await provider.wallet.signTransaction(tx);
  const simulation = await provider.connection.simulateTransaction(signed);
  if (simulation.value.err) {
    throw new Error(
      `${spec.name} current_value failed: ${JSON.stringify(simulation.value.err)}`,
    );
  }
  const returned = simulation.value.returnData;
  if (!returned) {
    throw new Error(`${spec.name} current_value returned no data`);
  }
  return Buffer.from(returned.data[0], "base64").readBigUInt64LE(0);
}

export async function readPositionShares(
  spec: ForkAdapterSpec,
): Promise<bigint> {
  const account = await forkProvider().connection.getAccountInfo(
    routeContext(spec).position,
  );
  if (!account) {
    return 0n;
  }
  return account.data.readBigUInt64LE(72);
}

export async function tokenBalance(account: PublicKey): Promise<bigint> {
  const response = await forkProvider()
    .connection.getTokenAccountBalance(account)
    .catch(() => null);
  return response ? BigInt(response.value.amount) : 0n;
}

export const ADAPTER_SPECS: ForkAdapterSpec[] = [
  {
    name: "Kamino USDC",
    adapterProgram: KAMINO.adapter,
    protocolVariant: KAMINO.protocolVariant,
    receiptMint: KAMINO.reserveCollateralMint,
    amountIn: 1_000_000n,
    tail: (ctx) => [
      writable(false, KAMINO.reserveCollateralMint),
      writable(true, requiredReceipt(ctx, "Kamino")),
      writable(true, KAMINO.reserve),
      writable(false, KAMINO.lendingMarket),
      writable(false, kaminoMarketAuthority()),
      writable(true, KAMINO.reserveLiquiditySupply),
      writable(false, SYSVAR_ACCOUNTS.instructions),
      writable(false, KAMINO.program),
    ],
  },
  {
    name: "MarginFi USDC",
    adapterProgram: MARGINFI.adapter,
    protocolVariant: MARGINFI.protocolVariant,
    amountIn: 1_000_000n,
    tail: (ctx) => [
      writable(true, marginfiAccount(ctx.positionAuthority)),
      writable(false, MARGINFI.group),
      writable(true, MARGINFI.usdcBank),
      writable(false, MARGINFI.liquidityVaultAuthority),
      writable(true, MARGINFI.liquidityVault),
      writable(false, MARGINFI.oracle),
      writable(false, SYSVAR_ACCOUNTS.instructions),
      writable(false, MARGINFI.program),
    ],
  },
  {
    name: "Jupiter JLP",
    adapterProgram: JUPITER.adapter,
    protocolVariant: JUPITER.protocolVariant,
    receiptMint: JUPITER.jlpMint,
    amountIn: 1_000_000n,
    tail: (ctx) => [
      writable(true, JUPITER.jlpMint),
      writable(true, requiredReceipt(ctx, "Jupiter")),
      writable(false, JUPITER.transferAuthority),
      writable(false, JUPITER.perpetuals),
      writable(true, JUPITER.pool),
      writable(true, JUPITER.usdcCustody),
      writable(false, JUPITER.usdcDovesPrice),
      writable(false, JUPITER.usdcPythnetPrice),
      writable(true, JUPITER.custodyTokenAccount),
      writable(false, JUPITER.eventAuthority),
      writable(false, JUPITER.program),
      ...JUPITER.custodies.map((pubkey) => writable(false, pubkey)),
      ...JUPITER.dovesAgPrices.map((pubkey) => writable(false, pubkey)),
    ],
  },
  {
    name: "Maple Syrup",
    adapterProgram: MAPLE.adapter,
    protocolVariant: MAPLE.protocolVariant,
    receiptMint: MAPLE.syrupMint,
    amountIn: 1_000_000n,
    tail: (ctx, action) => {
      const tickArray1 =
        action === "withdraw" ? MAPLE.sellTickArray1 : MAPLE.buyTickArray1;
      const tickArray2 =
        action === "withdraw" ? MAPLE.sellTickArray2 : MAPLE.buyTickArray2;
      return [
        writable(false, MAPLE.syrupMint),
        writable(true, requiredReceipt(ctx, "Maple")),
        writable(true, MAPLE.whirlpool),
        writable(true, MAPLE.tokenVaultA),
        writable(true, MAPLE.tokenVaultB),
        writable(true, MAPLE.tickArray0),
        writable(true, tickArray1),
        writable(true, tickArray2),
        writable(true, mapleOracle()),
        writable(false, MAPLE.program),
      ];
    },
  },
  {
    name: "Drift Insurance Fund",
    adapterProgram: DRIFT.adapter,
    protocolVariant: DRIFT.protocolVariant,
    amountIn: 1_000_000n,
    delayedWithdraw: true,
    tail: (ctx) => [
      writable(true, driftUserStats(ctx.positionAuthority)),
      writable(true, driftInsuranceFundStake(ctx.positionAuthority)),
      writable(true, driftState()),
      writable(true, driftSpotMarket()),
      writable(true, driftSpotMarketVault()),
      writable(true, driftInsuranceFundVault()),
      writable(false, driftSigner()),
      writable(false, SYSVAR_ACCOUNTS.rent),
      writable(false, DRIFT.program),
    ],
  },
];

function enableAdapterInstruction(
  spec: ForkAdapterSpec,
): TransactionInstruction {
  return new TransactionInstruction({
    programId: REGISTRY_PROGRAM_ID,
    keys: [
      { pubkey: registryAccount, isSigner: false, isWritable: false },
      {
        pubkey: adapterEntryPda(spec.adapterProgram),
        isSigner: false,
        isWritable: true,
      },
      { pubkey: owner(), isSigner: true, isWritable: false },
    ],
    data: discriminator("enable_adapter"),
  });
}

async function sendRoute(
  spec: ForkAdapterSpec,
  name: "route_deposit" | "route_withdraw",
  args: Uint8Array[],
): Promise<void> {
  const provider = forkProvider();
  const tx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    routeInstruction(spec, name, args, routeContext(spec)),
  );
  await provider.sendAndConfirm(tx, []);
}

function routeInstruction(
  spec: ForkAdapterSpec,
  name: "route_deposit" | "route_withdraw" | "route_current_value",
  args: Uint8Array[],
  ctx: RouteContext,
): TransactionInstruction {
  const action = routeAction(name);
  return new TransactionInstruction({
    programId: DISPATCHER_PROGRAM_ID,
    keys: [
      writable(true, ctx.position),
      writable(false, ctx.positionAuthority),
      writable(false, USDC_MINT),
      writable(true, ctx.adapterVault),
      { pubkey: ctx.owner, isSigner: true, isWritable: true },
      writable(true, ctx.ownerTokenAccount),
      writable(false, adapterEntryPda(spec.adapterProgram)),
      writable(false, TOKEN_PROGRAM_ID),
      writable(false, SystemProgram.programId),
      writable(false, spec.adapterProgram),
      ...spec.tail(ctx, action),
    ],
    data: concatBytes(discriminator(name), ...args),
  });
}

function routeAction(
  name: "route_deposit" | "route_withdraw" | "route_current_value",
): RouteAction {
  if (name === "route_deposit") {
    return "deposit";
  }
  if (name === "route_withdraw") {
    return "withdraw";
  }
  return "current_value";
}

function writable(isWritable: boolean, pubkey: PublicKey): AccountMeta {
  return { pubkey, isSigner: false, isWritable };
}

function requiredReceipt(ctx: RouteContext, name: string): PublicKey {
  if (!ctx.receiptVault) {
    throw new Error(`${name} spec is missing a receipt vault`);
  }
  return ctx.receiptVault;
}

function discriminator(name: string): Buffer {
  return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

function u64(value: bigint): Uint8Array {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(value);
  return out;
}

function concatBytes(...items: Uint8Array[]): Buffer {
  return Buffer.concat(items.map((item) => Buffer.from(item)));
}
