import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { Keypair } from "@solana/web3.js";

type ProgramConfig = {
  name: string;
  keypairFile: string;
  libPath: string;
  sdkConst: string;
};

const PROGRAMS: readonly ProgramConfig[] = [
  {
    name: "registry",
    keypairFile: "registry-keypair.json",
    libPath: "programs/registry/src/lib.rs",
    sdkConst: "REGISTRY_PROGRAM_ID",
  },
  {
    name: "dispatcher",
    keypairFile: "dispatcher-keypair.json",
    libPath: "programs/dispatcher/src/lib.rs",
    sdkConst: "DISPATCHER_PROGRAM_ID",
  },
  {
    name: "mock_adapter",
    keypairFile: "mock_adapter-keypair.json",
    libPath: "programs/adapters/mock/src/lib.rs",
    sdkConst: "MOCK_ADAPTER_PROGRAM_ID",
  },
  {
    name: "kamino_usdc_adapter",
    keypairFile: "kamino_usdc_adapter-keypair.json",
    libPath: "programs/adapters/kamino-usdc/src/lib.rs",
    sdkConst: "KAMINO_USDC_ADAPTER_PROGRAM_ID",
  },
  {
    name: "marginfi_usdc_adapter",
    keypairFile: "marginfi_usdc_adapter-keypair.json",
    libPath: "programs/adapters/marginfi-usdc/src/lib.rs",
    sdkConst: "MARGINFI_USDC_ADAPTER_PROGRAM_ID",
  },
  {
    name: "jupiter_jlp_adapter",
    keypairFile: "jupiter_jlp_adapter-keypair.json",
    libPath: "programs/adapters/jupiter-jlp/src/lib.rs",
    sdkConst: "JUPITER_JLP_ADAPTER_PROGRAM_ID",
  },
  {
    name: "maple_syrup_adapter",
    keypairFile: "maple_syrup_adapter-keypair.json",
    libPath: "programs/adapters/maple-syrup/src/lib.rs",
    sdkConst: "MAPLE_SYRUP_ADAPTER_PROGRAM_ID",
  },
  {
    name: "drift_if_adapter",
    keypairFile: "drift_if_adapter-keypair.json",
    libPath: "programs/adapters/drift-if/src/lib.rs",
    sdkConst: "DRIFT_IF_ADAPTER_PROGRAM_ID",
  },
] as const;

const KEYPAIR_DIR = "program-keypairs";
const TARGET_DEPLOY_DIR = "target/deploy";
const PUBLIC_KEY_RE = "[1-9A-HJ-NP-Za-km-z]{32,44}";

function main(): void {
  const command = process.argv[2] ?? "ids:check";
  switch (command) {
    case "ids:check":
      verifyIds();
      break;
    case "sync":
      sync();
      break;
    case "restore":
      restore();
      verifyKeys();
      break;
    case "restore:registry":
      restoreRegistry();
      verifyRegistryKey();
      break;
    case "verify":
      verifyKeys();
      break;
    case "verify:registry":
      verifyRegistryKey();
      break;
    default:
      throw new Error(`unknown program-keys command: ${command}`);
  }
}

function sync(): void {
  mkdirSync(KEYPAIR_DIR, { recursive: true });
  mkdirSync(TARGET_DEPLOY_DIR, { recursive: true });

  const publicKeys = new Map<string, string>();
  for (const program of PROGRAMS) {
    const keypair = readOrCreateKeypair(canonicalPath(program));
    publicKeys.set(program.name, keypair.publicKey.toBase58());
    writeKeypair(targetPath(program), keypair);
  }

  updateAnchorToml(publicKeys);
  updateProgramSources(publicKeys);
  updateSdk(publicKeys);
  verifyIds();
  verifyKeys();
}

function restore(): void {
  mkdirSync(TARGET_DEPLOY_DIR, { recursive: true });
  for (const program of PROGRAMS) {
    if (!existsSync(canonicalPath(program))) {
      throw new Error(
        `${canonicalPath(program)} is missing; run npm run keys:sync before deployment or fork tests.`,
      );
    }
    writeKeypair(targetPath(program), readKeypair(canonicalPath(program)));
  }
}

function restoreRegistry(): void {
  mkdirSync(TARGET_DEPLOY_DIR, { recursive: true });
  const registry = registryProgram();
  if (!existsSync(canonicalPath(registry))) {
    throw new Error(
      `${canonicalPath(registry)} is missing; run npm run keys:sync if you do not have the checked-in registry program keypair.`,
    );
  }
  writeKeypair(targetPath(registry), readKeypair(canonicalPath(registry)));
}

function verifyIds(): void {
  const anchorToml = read("Anchor.toml");
  const sdk = read("sdk/src/index.ts");

  for (const program of PROGRAMS) {
    const expected = expectedProgramId(program, anchorToml);
    assertAnchorClustersMatch(program, anchorToml, expected);

    expectIncludes(
      anchorToml,
      `${program.name} = "${expected}"`,
      `Anchor.toml is not synced for ${program.name}`,
    );
    expectIncludes(
      read(program.libPath),
      `declare_id!("${expected}")`,
      `${program.libPath} is not synced`,
    );
    expectIncludes(
      sdk,
      `export const ${program.sdkConst} = new PublicKey(\n  "${expected}",\n);`,
      `sdk/src/index.ts is not synced for ${program.sdkConst}`,
    );
  }

  console.log("program ids are synced");
}

function verifyKeys(): void {
  const anchorToml = read("Anchor.toml");

  for (const program of PROGRAMS) {
    const expected = expectedProgramId(program, anchorToml);
    const target = requireKeypair(
      targetPath(program),
      "run npm run keys:restore or npm run keys:sync",
    );
    const actualTarget = target.publicKey.toBase58();
    if (actualTarget !== expected) {
      throw new Error(
        `${targetPath(program)} public key is ${actualTarget}, expected ${expected}`,
      );
    }

    if (existsSync(canonicalPath(program))) {
      const canonical = readKeypair(canonicalPath(program));
      const actualCanonical = canonical.publicKey.toBase58();
      if (actualCanonical !== expected) {
        throw new Error(
          `${canonicalPath(program)} public key is ${actualCanonical}, expected ${expected}`,
        );
      }
    }
  }

  console.log("program keypairs are restored and synced");
}

function verifyRegistryKey(): void {
  const registry = registryProgram();
  const expected = expectedProgramId(registry, read("Anchor.toml"));
  const target = requireKeypair(
    targetPath(registry),
    "run npm run keys:restore:registry or npm run keys:sync",
  );
  const actualTarget = target.publicKey.toBase58();
  if (actualTarget !== expected) {
    throw new Error(
      `${targetPath(registry)} public key is ${actualTarget}, expected ${expected}`,
    );
  }

  if (existsSync(canonicalPath(registry))) {
    const canonical = readKeypair(canonicalPath(registry));
    const actualCanonical = canonical.publicKey.toBase58();
    if (actualCanonical !== expected) {
      throw new Error(
        `${canonicalPath(registry)} public key is ${actualCanonical}, expected ${expected}`,
      );
    }
  }

  console.log("registry keypair is restored and synced");
}

function updateAnchorToml(publicKeys: Map<string, string>): void {
  let source = read("Anchor.toml");
  for (const [name, publicKey] of publicKeys) {
    const pattern = new RegExp(
      `^${escapeRegExp(name)}\\s*=\\s*"${PUBLIC_KEY_RE}"`,
      "gm",
    );
    source = replaceRequired(source, pattern, `${name} = "${publicKey}"`, name);
  }
  writeIfChanged("Anchor.toml", source);
}

function updateProgramSources(publicKeys: Map<string, string>): void {
  for (const program of PROGRAMS) {
    const publicKey = publicKeys.get(program.name);
    if (!publicKey) {
      throw new Error(`missing public key for ${program.name}`);
    }
    const source = read(program.libPath);
    const updated = replaceRequired(
      source,
      new RegExp(`declare_id!\\("${PUBLIC_KEY_RE}"\\);`),
      `declare_id!("${publicKey}");`,
      program.libPath,
    );
    writeIfChanged(program.libPath, updated);
  }
}

function updateSdk(publicKeys: Map<string, string>): void {
  let source = read("sdk/src/index.ts");
  for (const program of PROGRAMS) {
    const publicKey = publicKeys.get(program.name);
    if (!publicKey) {
      throw new Error(`missing public key for ${program.name}`);
    }
    const pattern = new RegExp(
      `export const ${escapeRegExp(program.sdkConst)} = new PublicKey\\(\\r?\\n\\s*"${PUBLIC_KEY_RE}",\\r?\\n\\);`,
    );
    source = replaceRequired(
      source,
      pattern,
      `export const ${program.sdkConst} = new PublicKey(\n  "${publicKey}",\n);`,
      program.sdkConst,
    );
  }
  writeIfChanged("sdk/src/index.ts", source);
}

function readOrCreateKeypair(path: string): Keypair {
  if (existsSync(path)) {
    return readKeypair(path);
  }

  const keypair = Keypair.generate();
  writeKeypair(path, keypair);
  console.log(`generated ${path}: ${keypair.publicKey.toBase58()}`);
  return keypair;
}

function requireKeypair(path: string, hint: string): Keypair {
  if (!existsSync(path)) {
    throw new Error(`${path} is missing; ${hint}`);
  }
  return readKeypair(path);
}

function readKeypair(path: string): Keypair {
  const raw = JSON.parse(read(path)) as number[];
  if (!Array.isArray(raw) || raw.length !== 64) {
    throw new Error(`${path} must contain a Solana 64-byte keypair array`);
  }
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

function writeKeypair(path: string, keypair: Keypair): void {
  mkdirSync(dirname(path), { recursive: true });
  writeIfChanged(path, JSON.stringify(Array.from(keypair.secretKey)));
}

function canonicalPath(program: ProgramConfig): string {
  return join(KEYPAIR_DIR, program.keypairFile);
}

function targetPath(program: ProgramConfig): string {
  return join(TARGET_DEPLOY_DIR, program.keypairFile);
}

function registryProgram(): ProgramConfig {
  const registry = PROGRAMS.find((program) => program.name === "registry");
  if (!registry) {
    throw new Error("registry program config is missing");
  }
  return registry;
}

function read(path: string): string {
  return readFileSync(path, "utf8");
}

function writeIfChanged(path: string, content: string): void {
  if (existsSync(path) && read(path) === content) {
    return;
  }
  writeFileSync(path, content);
}

function replaceRequired(
  source: string,
  pattern: RegExp,
  replacement: string,
  label: string,
): string {
  if (!pattern.test(source)) {
    throw new Error(`failed to update ${label}`);
  }
  const updated = source.replace(pattern, replacement);
  return updated;
}

function expectIncludes(source: string, needle: string, message: string): void {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function expectedProgramId(program: ProgramConfig, anchorToml: string): string {
  const match = anchorToml.match(
    new RegExp(
      `^${escapeRegExp(program.name)}\\s*=\\s*"(${PUBLIC_KEY_RE})"`,
      "m",
    ),
  );
  if (!match?.[1]) {
    throw new Error(`Anchor.toml is missing ${program.name}`);
  }
  return match[1];
}

function assertAnchorClustersMatch(
  program: ProgramConfig,
  anchorToml: string,
  expected: string,
): void {
  const matches = Array.from(
    anchorToml.matchAll(
      new RegExp(
        `^${escapeRegExp(program.name)}\\s*=\\s*"(${PUBLIC_KEY_RE})"`,
        "gm",
      ),
    ),
  );
  if (matches.length < 2) {
    throw new Error(
      `Anchor.toml must define ${program.name} for localnet and devnet`,
    );
  }
  for (const match of matches) {
    if (match[1] !== expected) {
      throw new Error(`Anchor.toml has mismatched ids for ${program.name}`);
    }
  }
}

main();
