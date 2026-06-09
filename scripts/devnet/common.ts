import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { homedir } from "node:os";
import { resolve } from "node:path";
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  SystemProgram,
} from "@solana/web3.js";
import { REGISTRY_PROGRAM_ID, registryPda } from "../../sdk/src/index.js";

export const DEVNET_URL =
  process.env.SOLANA_DEVNET_RPC_URL ?? "https://api.devnet.solana.com";
export const STANDARD_VERSION = 1;

export function devnetConnection(): Connection {
  return new Connection(DEVNET_URL, "confirmed");
}

export function governanceKeypair(): Keypair {
  const walletPath = resolveWalletPath(
    process.env.ANCHOR_WALLET ?? "~/.config/solana/id.json",
  );
  const raw = JSON.parse(readFileSync(walletPath, "utf8")) as number[];
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

export function registryAddress(): PublicKey {
  return registryPda(REGISTRY_PROGRAM_ID);
}

export async function initializeRegistryIfNeeded(
  connection: Connection,
  governance: Keypair,
): Promise<string | undefined> {
  const registry = registryAddress();
  const info = await connection.getAccountInfo(registry);
  if (info) {
    if (!info.owner.equals(REGISTRY_PROGRAM_ID)) {
      throw new Error(
        `registry PDA ${registry.toBase58()} is owned by ${info.owner.toBase58()}, expected ${REGISTRY_PROGRAM_ID.toBase58()}`,
      );
    }
    return undefined;
  }

  const tx = new Transaction().add(
    new TransactionInstruction({
      programId: REGISTRY_PROGRAM_ID,
      keys: [
        { pubkey: registry, isSigner: false, isWritable: true },
        { pubkey: governance.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: concatBytes(
        discriminator("initialize_registry"),
        governance.publicKey.toBytes(),
      ),
    }),
  );

  return sendAndConfirmTransaction(connection, tx, [governance], {
    commitment: "confirmed",
  });
}

export function discriminator(name: string): Buffer {
  return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

export type RegistryState = {
  version: number;
  governance: PublicKey;
  pendingGovernance: PublicKey;
  adapterCount: number;
  bump: number;
};

export function decodeRegistryState(data: Buffer): RegistryState {
  const expected = accountDiscriminator("Registry");
  if (data.length < 79) {
    throw new Error(`registry account is too short: ${data.length} bytes`);
  }
  if (!data.subarray(0, 8).equals(expected)) {
    throw new Error("registry account discriminator does not match Registry");
  }

  const version = data.readUInt16LE(8);
  if (version !== STANDARD_VERSION) {
    throw new Error(
      `registry version is ${version}, expected ${STANDARD_VERSION}`,
    );
  }

  const governance = new PublicKey(data.subarray(10, 42));
  if (governance.equals(PublicKey.default)) {
    throw new Error("registry governance is the default public key");
  }

  return {
    version,
    governance,
    pendingGovernance: new PublicKey(data.subarray(42, 74)),
    adapterCount: data.readUInt32LE(74),
    bump: data.readUInt8(78),
  };
}

function accountDiscriminator(name: string): Buffer {
  return createHash("sha256").update(`account:${name}`).digest().subarray(0, 8);
}

export function concatBytes(...items: Uint8Array[]): Buffer {
  return Buffer.concat(items.map((item) => Buffer.from(item)));
}

function resolveWalletPath(path: string): string {
  if (path === "~") {
    return homedir();
  }
  if (path.startsWith("~/")) {
    return resolve(homedir(), path.slice(2));
  }
  return resolve(path);
}
