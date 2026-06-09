import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import {
  EXTERNAL_PROGRAMS,
  LOCAL_ACCOUNT_FIXTURES,
  MAINNET_CLONES,
  PATCHED_DOVES_ACCOUNTS,
  TOKEN_PROGRAM_ID,
} from "./accounts.js";
import {
  DISPATCHER_PROGRAM_ID,
  DRIFT_IF_ADAPTER_PROGRAM_ID,
  JUPITER_JLP_ADAPTER_PROGRAM_ID,
  KAMINO_USDC_ADAPTER_PROGRAM_ID,
  MAPLE_SYRUP_ADAPTER_PROGRAM_ID,
  MARGINFI_USDC_ADAPTER_PROGRAM_ID,
  REGISTRY_PROGRAM_ID,
  USDC_MINT,
} from "../../sdk/src/index.js";

const LOCAL_URL = "http://127.0.0.1:8899";
const WORK_DIR = "/tmp/syas-mainnet-fork";
const LEDGER_DIR = join(WORK_DIR, "ledger");
const PAYER_PATH = join(WORK_DIR, "payer.json");
const EVIDENCE_PATH = resolve("target/syas-mainnet-fork-evidence.json");
const OWNER_USDC_AMOUNT = 1_000_000_000_000n;

type ValidatorExit = { code: number | null; signal: NodeJS.Signals | null };

async function main(): Promise<void> {
  const mainnetRpc = process.env.MAINNET_RPC_URL;
  if (!mainnetRpc) {
    throw new Error(
      "MAINNET_RPC_URL is required for strict mainnet-fork tests",
    );
  }

  await rm(WORK_DIR, { recursive: true, force: true });
  await mkdir(WORK_DIR, { recursive: true });

  const payer = deterministicPayer();
  await writeFile(PAYER_PATH, JSON.stringify(Array.from(payer.secretKey)));
  const ownerTokenAccount = getAssociatedTokenAddressSync(
    USDC_MINT,
    payer.publicKey,
    false,
  );
  const mainnetConnection = new Connection(mainnetRpc, "confirmed");
  const forkSlot = await mainnetConnection.getSlot("confirmed");
  const ownerTokenPath = join(WORK_DIR, "owner-usdc.json");
  await writeFile(
    ownerTokenPath,
    JSON.stringify(tokenAccountFixture(payer.publicKey), null, 2),
  );

  const patchedAccounts = await writePatchedDovesAccounts(mainnetConnection);
  const localAccountFixtures = await writeLocalAccountFixtures();
  await spawnChecked("anchor", ["build", "--no-idl"]);
  assertProgramArtifacts();

  const validatorArgs = [
    "--reset",
    "--quiet",
    "--ledger",
    LEDGER_DIR,
    "--url",
    mainnetRpc,
    ...EXTERNAL_PROGRAMS.flatMap((program) => [
      "--clone-upgradeable-program",
      program.toBase58(),
    ]),
    ...MAINNET_CLONES.filter(
      (key) => !PATCHED_DOVES_ACCOUNTS.some((patched) => patched.equals(key)),
    ).flatMap((account) => ["--clone", account.toBase58()]),
    "--account",
    ownerTokenAccount.toBase58(),
    ownerTokenPath,
    ...patchedAccounts.flatMap(({ pubkey, path }) => [
      "--account",
      pubkey.toBase58(),
      path,
    ]),
    ...localAccountFixtures.flatMap(({ pubkey, path }) => [
      "--account",
      pubkey.toBase58(),
      path,
    ]),
    ...LOCAL_PROGRAMS.flatMap((program) => [
      "--bpf-program",
      EXPECTED_PROGRAM_IDS[program].toBase58(),
      `target/deploy/${program}.so`,
    ]),
  ];

  const validator = spawn("solana-test-validator", validatorArgs, {
    cwd: process.cwd(),
    stdio: "inherit",
  });
  let validatorExit: ValidatorExit | undefined;
  validator.on("exit", (code, signal) => {
    validatorExit = { code, signal };
  });

  const stopValidator = (): void => {
    if (!validator.killed) {
      validator.kill("SIGTERM");
    }
  };
  process.on("exit", stopValidator);
  process.on("SIGINT", () => {
    stopValidator();
    process.exit(130);
  });
  process.on("SIGTERM", () => {
    stopValidator();
    process.exit(143);
  });

  await waitForHealth(() => validatorExit);
  await spawnChecked("solana", [
    "airdrop",
    "1000",
    payer.publicKey.toBase58(),
    "--url",
    LOCAL_URL,
  ]);

  await spawnChecked(
    "node",
    [
      "--import",
      "tsx",
      "./node_modules/mocha/bin/mocha.js",
      "--timeout",
      "1000000",
      "tests/mainnet-fork/roundtrip.spec.ts",
    ],
    {
      ANCHOR_PROVIDER_URL: LOCAL_URL,
      ANCHOR_WALLET: PAYER_PATH,
      RUN_MAINNET_FORK_TESTS: "1",
      MAINNET_RPC_URL: mainnetRpc,
    },
  );

  await writeEvidence(forkSlot, payer.publicKey);
}

const LOCAL_PROGRAMS = [
  "registry",
  "dispatcher",
  "kamino_usdc_adapter",
  "marginfi_usdc_adapter",
  "jupiter_jlp_adapter",
  "maple_syrup_adapter",
  "drift_if_adapter",
] as const;

const EXPECTED_PROGRAM_IDS: Record<(typeof LOCAL_PROGRAMS)[number], PublicKey> =
  {
    registry: REGISTRY_PROGRAM_ID,
    dispatcher: DISPATCHER_PROGRAM_ID,
    kamino_usdc_adapter: KAMINO_USDC_ADAPTER_PROGRAM_ID,
    marginfi_usdc_adapter: MARGINFI_USDC_ADAPTER_PROGRAM_ID,
    jupiter_jlp_adapter: JUPITER_JLP_ADAPTER_PROGRAM_ID,
    maple_syrup_adapter: MAPLE_SYRUP_ADAPTER_PROGRAM_ID,
    drift_if_adapter: DRIFT_IF_ADAPTER_PROGRAM_ID,
  };

function assertProgramArtifacts(): void {
  for (const program of LOCAL_PROGRAMS) {
    const path = `target/deploy/${program}.so`;
    if (!existsSync(path)) {
      throw new Error(
        `${path} is missing. Run anchor build --no-idl before running strict fork tests.`,
      );
    }
  }
}

function deterministicPayer(): Keypair {
  return Keypair.fromSeed(Uint8Array.from(new Array(32).fill(7)));
}

function tokenAccountFixture(owner: PublicKey): unknown {
  const data = Buffer.alloc(165);
  USDC_MINT.toBuffer().copy(data, 0);
  owner.toBuffer().copy(data, 32);
  data.writeBigUInt64LE(OWNER_USDC_AMOUNT, 64);
  data.writeUInt8(1, 108);

  return {
    pubkey: getAssociatedTokenAddressSync(USDC_MINT, owner, false).toBase58(),
    account: {
      lamports: 2_039_280,
      data: [data.toString("base64"), "base64"],
      owner: TOKEN_PROGRAM_ID.toBase58(),
      executable: false,
      rentEpoch: 0,
    },
  };
}

async function writePatchedDovesAccounts(
  connection: Connection,
): Promise<Array<{ pubkey: PublicKey; path: string }>> {
  const now = BigInt(Math.floor(Date.now() / 1000));
  const out: Array<{ pubkey: PublicKey; path: string }> = [];

  for (const pubkey of PATCHED_DOVES_ACCOUNTS) {
    const info = await connection.getAccountInfo(pubkey);
    if (!info) {
      throw new Error(`missing mainnet account ${pubkey.toBase58()}`);
    }
    const data = Buffer.from(info.data);
    if (data.length > 185) {
      data.writeBigInt64LE(now, 177);
    }
    const path = join(WORK_DIR, `${pubkey.toBase58()}.json`);
    await writeFile(
      path,
      JSON.stringify(
        {
          pubkey: pubkey.toBase58(),
          account: {
            lamports: info.lamports,
            data: [data.toString("base64"), "base64"],
            owner: info.owner.toBase58(),
            executable: info.executable,
            rentEpoch: safeRentEpoch(info.rentEpoch),
          },
        },
        null,
        2,
      ),
    );
    out.push({ pubkey, path });
  }

  return out;
}

async function writeLocalAccountFixtures(): Promise<
  Array<{ pubkey: PublicKey; path: string }>
> {
  const out: Array<{ pubkey: PublicKey; path: string }> = [];

  for (const fixture of LOCAL_ACCOUNT_FIXTURES) {
    const path = join(WORK_DIR, `${fixture.pubkey.toBase58()}.json`);
    const data = Buffer.alloc(fixture.dataLength);
    await writeFile(
      path,
      JSON.stringify(
        {
          pubkey: fixture.pubkey.toBase58(),
          account: {
            lamports: fixture.lamports,
            data: [data.toString("base64"), "base64"],
            owner: fixture.owner.toBase58(),
            executable: fixture.executable ?? false,
            rentEpoch: fixture.rentEpoch ?? 0,
          },
        },
        null,
        2,
      ),
    );
    out.push({ pubkey: fixture.pubkey, path });
  }

  return out;
}

function safeRentEpoch(rentEpoch: number | undefined): number {
  return typeof rentEpoch === "number" && Number.isSafeInteger(rentEpoch)
    ? rentEpoch
    : 0;
}

async function writeEvidence(
  forkSlot: number,
  payer: PublicKey,
): Promise<void> {
  const evidence = {
    generatedAt: new Date().toISOString(),
    forkSlot,
    localValidator: LOCAL_URL,
    payer: payer.toBase58(),
    programs: {
      registry: REGISTRY_PROGRAM_ID.toBase58(),
      dispatcher: DISPATCHER_PROGRAM_ID.toBase58(),
      kaminoUsdcAdapter: KAMINO_USDC_ADAPTER_PROGRAM_ID.toBase58(),
      marginfiUsdcAdapter: MARGINFI_USDC_ADAPTER_PROGRAM_ID.toBase58(),
      jupiterJlpAdapter: JUPITER_JLP_ADAPTER_PROGRAM_ID.toBase58(),
      mapleSyrupAdapter: MAPLE_SYRUP_ADAPTER_PROGRAM_ID.toBase58(),
      driftIfAdapter: DRIFT_IF_ADAPTER_PROGRAM_ID.toBase58(),
    },
  };
  await writeFile(EVIDENCE_PATH, JSON.stringify(evidence, null, 2));
  console.log(`mainnet fork evidence: ${EVIDENCE_PATH}`);
}

async function waitForHealth(
  validatorExit: () => ValidatorExit | undefined,
): Promise<void> {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    const exit = validatorExit();
    if (exit) {
      throw new Error(
        `local fork validator exited before becoming healthy: code=${exit.code ?? "<null>"} signal=${exit.signal ?? "<null>"}`,
      );
    }

    try {
      const response = await fetch(LOCAL_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "getHealth" }),
      });
      const body = (await response.json()) as { result?: string };
      if (body.result === "ok") {
        return;
      }
    } catch {
      // validator is still booting
    }
    await delay(1_000);
  }
  throw new Error("local fork validator did not become healthy within 120s");
}

async function spawnChecked(
  command: string,
  args: string[],
  env: Record<string, string> = {},
): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: process.cwd(),
      stdio: "inherit",
      env: { ...process.env, ...env },
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} exited with ${code ?? signal}`));
      }
    });
  });
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
