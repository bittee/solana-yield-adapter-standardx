import { spawnSync } from "node:child_process";

type Check = {
  command: string;
  env?: NodeJS.ProcessEnv;
};

const localChecks: readonly Check[] = [
  { command: "npm run keys:ids:check" },
  { command: "cargo fmt --all -- --check" },
  { command: "cargo check --workspace --all-targets" },
  { command: "cargo test --workspace --all-targets" },
  { command: "cargo clippy --workspace --all-targets -- -D warnings" },
  {
    command:
      'npx prettier --check "{tests,sdk,scripts}/**/*.{ts,js,json}" "*.json" README.md docs/submission.md',
  },
  { command: "npm run typecheck" },
  {
    command: "npm test",
    env: {
      MAINNET_RPC_URL: "",
      RUN_MAINNET_FORK_TESTS: "",
    },
  },
] as const;

const requiredTools = ["anchor", "solana", "solana-test-validator"] as const;

function run(command: string, extraEnv: NodeJS.ProcessEnv = {}): void {
  console.log(`\n$ ${command}`);
  const result = spawnSync(command, {
    cwd: process.cwd(),
    env: { ...process.env, ...extraEnv },
    shell: true,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`${command} exited with ${result.status ?? result.signal}`);
  }
}

function requireTool(tool: string): void {
  const result =
    process.platform === "win32"
      ? spawnSync("where.exe", [tool], {
          stdio: "pipe",
          encoding: "utf8",
        })
      : spawnSync(`command -v ${tool}`, {
          shell: true,
          stdio: "pipe",
          encoding: "utf8",
        });
  if (result.status !== 0) {
    throw new Error(
      `missing required CLI '${tool}'. Install Solana 2.2.20 / Anchor 0.31.1 toolchain before bounty submission.`,
    );
  }
  console.log(`${tool}: ${result.stdout.trim()}`);
}

function requireVersion(command: string, expected: string): void {
  const result = spawnSync(command, {
    cwd: process.cwd(),
    shell: true,
    stdio: "pipe",
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`${command} exited with ${result.status ?? result.signal}`);
  }
  const output = `${result.stdout} ${result.stderr}`.trim();
  if (!output.includes(expected)) {
    throw new Error(
      `${command} must report ${expected}; got '${output || "<empty>"}'`,
    );
  }
  console.log(`${command}: ${output}`);
}

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} is required for bounty submission fork tests`);
  }
  return value;
}

function main(): void {
  for (const check of localChecks) {
    run(check.command, check.env);
  }

  for (const tool of requiredTools) {
    requireTool(tool);
  }
  requireVersion("anchor --version", "0.31.1");
  requireVersion("solana --version", "2.2.20");
  requireVersion("solana-test-validator --version", "2.2.20");

  requireEnv("MAINNET_RPC_URL");

  run("anchor build --no-idl");
  run("npm run verify:registry:devnet");
  run("npm run test:fork", { RUN_MAINNET_FORK_TESTS: "1" });
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
