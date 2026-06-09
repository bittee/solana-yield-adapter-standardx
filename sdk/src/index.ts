import { PublicKey } from "@solana/web3.js";

export const REGISTRY_PROGRAM_ID = new PublicKey(
  "5VkZ8ibMyrwzJWsybHqXPQ29z7e6eRdaYph4iCPdCQsb",
);
export const DISPATCHER_PROGRAM_ID = new PublicKey(
  "HGj3chDufhrN3LZE31jjK9Kv4ETzmzkxHwxDQKCzUrk",
);
export const MOCK_ADAPTER_PROGRAM_ID = new PublicKey(
  "8p7m5zPd9S52CXnEzNu3JBVqraUSwyktF4JWYaaphmEr",
);
export const KAMINO_USDC_ADAPTER_PROGRAM_ID = new PublicKey(
  "E1bTG9vyE27xVay1oZrZck6idNJZjgotaeFia1P7q9Vb",
);
export const MARGINFI_USDC_ADAPTER_PROGRAM_ID = new PublicKey(
  "CEy21HbuzU6K9WLueUwXtjeiLicVhyMLPtkMHXEzccXu",
);
export const JUPITER_JLP_ADAPTER_PROGRAM_ID = new PublicKey(
  "4A1xkP49MszrDZE3Pzzq6a69tNprr3X799NitbAy7RmN",
);
export const MAPLE_SYRUP_ADAPTER_PROGRAM_ID = new PublicKey(
  "7Bw1gXZzHz1RFD1FBkqGbAfVoBTz5CYk73QQkzjw8NWf",
);
export const DRIFT_IF_ADAPTER_PROGRAM_ID = new PublicKey(
  "BemVwXxgBf71TXQQrWJH61SR1oodD7tPzX8LFymeB6tM",
);

export const USDC_MINT = new PublicKey(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
);

export function registryPda(programId = REGISTRY_PROGRAM_ID): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("registry")],
    programId,
  )[0];
}

export function adapterEntryPda(
  adapterProgram: PublicKey,
  registryProgram = REGISTRY_PROGRAM_ID,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("adapter_entry"), adapterProgram.toBuffer()],
    registryProgram,
  )[0];
}

export function positionPda(
  adapterProgram: PublicKey,
  owner: PublicKey,
  baseMint: PublicKey,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position"), owner.toBuffer(), baseMint.toBuffer()],
    adapterProgram,
  )[0];
}

export function positionAuthorityPda(
  adapterProgram: PublicKey,
  position: PublicKey,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position_authority"), position.toBuffer()],
    adapterProgram,
  )[0];
}

export function adapterVaultPda(
  adapterProgram: PublicKey,
  position: PublicKey,
  baseMint: PublicKey,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), position.toBuffer(), baseMint.toBuffer()],
    adapterProgram,
  )[0];
}

export function receiptVaultPda(
  adapterProgram: PublicKey,
  position: PublicKey,
  receiptMint: PublicKey,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("receipt_vault"), position.toBuffer(), receiptMint.toBuffer()],
    adapterProgram,
  )[0];
}

export function decodeReturnedU64(data: Buffer): bigint {
  if (data.length < 8) {
    throw new Error("return data is shorter than u64");
  }
  return data.readBigUInt64LE(0);
}
