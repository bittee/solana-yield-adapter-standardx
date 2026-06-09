import {
  DEVNET_URL,
  decodeRegistryState,
  devnetConnection,
  registryAddress,
} from "./common.js";
import { REGISTRY_PROGRAM_ID } from "../../sdk/src/index.js";

async function main(): Promise<void> {
  const connection = devnetConnection();

  const programInfo = await connection.getAccountInfo(REGISTRY_PROGRAM_ID);
  if (!programInfo) {
    throw new Error(
      `registry program ${REGISTRY_PROGRAM_ID.toBase58()} is missing on devnet`,
    );
  }
  if (!programInfo.executable) {
    throw new Error(
      `registry program ${REGISTRY_PROGRAM_ID.toBase58()} is not executable on devnet`,
    );
  }

  const registry = registryAddress();
  const registryInfo = await connection.getAccountInfo(registry);
  if (!registryInfo) {
    throw new Error(
      `registry PDA ${registry.toBase58()} is missing on devnet; run npm run deploy:registry:devnet`,
    );
  }
  if (!registryInfo.owner.equals(REGISTRY_PROGRAM_ID)) {
    throw new Error(
      `registry PDA ${registry.toBase58()} is owned by ${registryInfo.owner.toBase58()}, expected ${REGISTRY_PROGRAM_ID.toBase58()}`,
    );
  }
  const registryState = decodeRegistryState(registryInfo.data);

  console.log(`devnet RPC: ${DEVNET_URL}`);
  console.log(`registry program executable: ${REGISTRY_PROGRAM_ID.toBase58()}`);
  console.log(`registry PDA initialized: ${registry.toBase58()}`);
  console.log(`registry version: ${registryState.version}`);
  console.log(`registry governance: ${registryState.governance.toBase58()}`);
  console.log(`registry adapter count: ${registryState.adapterCount}`);
  console.log(`registry bump: ${registryState.bump}`);
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
