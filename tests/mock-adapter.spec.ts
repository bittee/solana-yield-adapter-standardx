import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorProvider, BN } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import {
  adapterEntryPda,
  MOCK_ADAPTER_PROGRAM_ID,
  positionAuthorityPda,
  positionPda,
  registryPda,
  USDC_MINT,
} from "../sdk/src/index.js";

const registryAccount = registryPda();
const adapterEntry = adapterEntryPda(MOCK_ADAPTER_PROGRAM_ID);

let provider: AnchorProvider;
let registry: Program;
let dispatcher: Program;
let mockAdapter: Program;
let owner: PublicKey;
let position: PublicKey;
let positionAuthority: PublicKey;

type MockPositionAccount = {
  shares: BN;
  cachedValue: BN;
};

type MockAdapterAccounts = {
  position: {
    fetch(address: PublicKey): Promise<MockPositionAccount>;
  };
};

async function ensureRegistered(): Promise<void> {
  const registryInfo = await provider.connection.getAccountInfo(registryAccount);
  if (!registryInfo) {
    await registry.methods
      .initializeRegistry(owner)
      .accounts({
        registry: registryAccount,
        payer: owner,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  }

  const entryInfo = await provider.connection.getAccountInfo(adapterEntry);
  if (!entryInfo) {
    await registry.methods
      .registerAdapter(MOCK_ADAPTER_PROGRAM_ID, USDC_MINT, { mock: {} }, 0)
      .accounts({
        registry: registryAccount,
        adapterEntry,
        governance: owner,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  }

  await registry.methods
    .enableAdapter()
    .accounts({
      registry: registryAccount,
      adapterEntry,
      governance: owner,
    })
    .rpc();
}

function routeAccounts() {
  return {
    position,
    positionAuthority,
    baseMint: USDC_MINT,
    adapterVault: owner,
    owner,
    ownerTokenAccount: owner,
    registryEntry: adapterEntry,
    tokenProgram: SystemProgram.programId,
    systemProgram: SystemProgram.programId,
    adapterProgram: MOCK_ADAPTER_PROGRAM_ID,
  };
}

describe("dispatcher + registry + mock adapter", () => {
  before(() => {
    provider = AnchorProvider.env();
    anchor.setProvider(provider);

    registry = anchor.workspace.Registry as Program;
    dispatcher = anchor.workspace.Dispatcher as Program;
    mockAdapter = anchor.workspace.MockAdapter as Program;

    owner = provider.wallet.publicKey;
    position = positionPda(MOCK_ADAPTER_PROGRAM_ID, owner, USDC_MINT);
    positionAuthority = positionAuthorityPda(MOCK_ADAPTER_PROGRAM_ID, position);
  });

  it("routes deposit, value, and withdraw", async () => {
    await ensureRegistered();

    await dispatcher.methods
      .routeDeposit(new BN(1_000), new BN(1_000))
      .accounts(routeAccounts())
      .rpc();

    const valueTx = await dispatcher.methods
      .routeCurrentValue()
      .accounts(routeAccounts())
      .transaction();
    valueTx.feePayer = owner;
    valueTx.recentBlockhash = (await provider.connection.getLatestBlockhash()).blockhash;
    const simulation = await provider.connection.simulateTransaction(valueTx);
    if (simulation.value.err) {
      throw new Error(`current_value simulation failed: ${JSON.stringify(simulation.value.err)}`);
    }
    const returnData = simulation.value.returnData;
    if (!returnData) {
      throw new Error("dispatcher did not return current_value data");
    }
    const value = Buffer.from(returnData.data[0], "base64").readBigUInt64LE(0);
    if (value !== 1_000n) {
      throw new Error(`expected value 1000, got ${value.toString()}`);
    }

    await dispatcher.methods
      .routeWithdraw(new BN(400), new BN(400))
      .accounts(routeAccounts())
      .rpc();

    const stored = await (mockAdapter.account as unknown as MockAdapterAccounts).position.fetch(
      position,
    );
    if (!stored.shares.eq(new BN(600))) {
      throw new Error(`expected 600 shares, got ${stored.shares.toString()}`);
    }
  });

  it("rejects routing while adapter is disabled", async () => {
    await ensureRegistered();
    await registry.methods
      .disableAdapter()
      .accounts({
        registry: registryAccount,
        adapterEntry,
        governance: owner,
      })
      .rpc();

    let failed = false;
    try {
      await dispatcher.methods
        .routeDeposit(new BN(1), new BN(1))
        .accounts(routeAccounts())
        .rpc();
    } catch {
      failed = true;
    }

    await registry.methods
      .enableAdapter()
      .accounts({
        registry: registryAccount,
        adapterEntry,
        governance: owner,
      })
      .rpc();

    if (!failed) {
      throw new Error("disabled adapter route unexpectedly succeeded");
    }
  });
});
