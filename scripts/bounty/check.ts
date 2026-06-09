import { spawnSync } from "node:child_process";

const localChecks = [
  "cargo fmt --all -- --check",
  "cargo check --workspace --all-targets",
  "cargo test --workspace --all-targets",
  "cargo clippy --workspace --all-targets -- -D warnings",
  'npx prettier --check "{tests,sdk,scripts}/**/*.{ts,js,json}" "*.json" README.md',
  "npm run typecheck",
  "npm test",
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
  const result = spawnSync(`command -v ${tool}`, {
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

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} is required for bounty submission fork tests`);
  }
  return value;
}

function main(): void {
  for (const command of localChecks) {
    run(command);
  }

  for (const tool of requiredTools) {
    requireTool(tool);
  }

  requireEnv("MAINNET_RPC_URL");

  run("anchor build");
  run("npm run test:fork", { RUN_MAINNET_FORK_TESTS: "1" });
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
