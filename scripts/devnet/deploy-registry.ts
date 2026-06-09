import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { Keypair } from "@solana/web3.js";
import {
  DEVNET_URL,
  devnetConnection,
  governanceKeypair,
  initializeRegistryIfNeeded,
  registryAddress,
} from "./common.js";
import { REGISTRY_PROGRAM_ID } from "../../sdk/src/index.js";

const REGISTRY_KEYPAIR_PATH = "target/deploy/registry-keypair.json";

function run(command: string): void {
  console.log(`\n$ ${command}`);
  const result = spawnSync(command, {
    cwd: process.cwd(),
    shell: true,
    stdio: "inherit",
    env: { ...process.env, ANCHOR_PROVIDER_URL: DEVNET_URL },
  });
  if (result.status !== 0) {
    throw new Error(`${command} exited with ${result.status ?? result.signal}`);
  }
}

async function main(): Promise<void> {
  const governance = governanceKeypair();

  assertProgramKeypair();

  run("anchor build");
  run("anchor deploy --provider.cluster devnet --program-name registry");

  const connection = devnetConnection();
  const programInfo = await connection.getAccountInfo(REGISTRY_PROGRAM_ID);
  if (!programInfo?.executable) {
    throw new Error(
      `registry program ${REGISTRY_PROGRAM_ID.toBase58()} is not executable on devnet`,
    );
  }

  const signature = await initializeRegistryIfNeeded(connection, governance);
  console.log(`devnet RPC: ${DEVNET_URL}`);
  console.log(`registry program: ${REGISTRY_PROGRAM_ID.toBase58()}`);
  console.log(`registry PDA: ${registryAddress().toBase58()}`);
  console.log(`governance: ${governance.publicKey.toBase58()}`);
  if (signature) {
    console.log(`initialize signature: ${signature}`);
  } else {
    console.log("registry PDA already initialized");
  }
}

function assertProgramKeypair(): void {
  if (!existsSync(REGISTRY_KEYPAIR_PATH)) {
    throw new Error(
      `${REGISTRY_KEYPAIR_PATH} is missing. Restore the registry program keypair whose public key is ${REGISTRY_PROGRAM_ID.toBase58()} with npm run keys:restore:registry, or run npm run keys:sync to generate a new local program id set and commit the synchronized public id changes before deploying.`,
    );
  }

  const secret = JSON.parse(readFileSync(REGISTRY_KEYPAIR_PATH, "utf8")) as
    | number[]
    | Uint8Array;
  const actual = Keypair.fromSecretKey(Uint8Array.from(secret)).publicKey;
  if (!actual.equals(REGISTRY_PROGRAM_ID)) {
    throw new Error(
      `${REGISTRY_KEYPAIR_PATH} public key is ${actual.toBase58()}, expected ${REGISTRY_PROGRAM_ID.toBase58()}. Update Anchor.toml/declare_id!/SDK together or use the matching keypair.`,
    );
  }
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
