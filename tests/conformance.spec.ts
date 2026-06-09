import { readFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const requiredAdapters = [
  "kamino-usdc",
  "marginfi-usdc",
  "jupiter-jlp",
  "maple-syrup",
  "drift-if",
] as const;

function read(path: string): string {
  return readFileSync(path, "utf8");
}

describe("SYAS bounty conformance preflight", () => {
  it("keeps the required documentation set present", () => {
    for (const path of [
      "docs/standard.md",
      "docs/architecture.md",
      "docs/build-your-own-adapter.md",
      "docs/mainnet-fork-tests.md",
      "docs/protocol-notes.md",
      "docs/submission.md",
    ]) {
      if (!existsSync(path)) {
        throw new Error(`missing required documentation file: ${path}`);
      }
    }
  });

  it("keeps every required adapter on the three-method standard interface", () => {
    for (const adapter of requiredAdapters) {
      const source = read(join("programs/adapters", adapter, "src/lib.rs"));
      for (const method of ["deposit", "withdraw", "current_value"]) {
        if (!source.includes(`pub fn ${method}`)) {
          throw new Error(`${adapter} is missing ${method}`);
        }
      }
    }
  });

  it("keeps adapters independent from each other", () => {
    for (const adapter of requiredAdapters) {
      const cargo = read(join("programs/adapters", adapter, "Cargo.toml"));
      for (const other of requiredAdapters) {
        if (other !== adapter && cargo.includes(other)) {
          throw new Error(`${adapter} depends on adapter crate ${other}`);
        }
      }
    }
  });

  it("keeps Anchor devnet program ids available for deployment", () => {
    const anchorToml = read("Anchor.toml");
    if (!anchorToml.includes("[programs.devnet]")) {
      throw new Error("Anchor.toml must define [programs.devnet]");
    }
    for (const program of [
      "dispatcher",
      "registry",
      "kamino_usdc_adapter",
      "marginfi_usdc_adapter",
      "jupiter_jlp_adapter",
      "maple_syrup_adapter",
      "drift_if_adapter",
    ]) {
      if (!anchorToml.includes(`${program} = `)) {
        throw new Error(`Anchor.toml is missing ${program}`);
      }
    }
  });

  it("keeps the dispatcher protocol-agnostic", () => {
    const dispatcher = read("programs/dispatcher/src/lib.rs");
    for (const forbidden of [
      "KLend",
      "marginfi",
      "PERPH",
      "whirlpool",
      "Drift",
      "insurance_fund",
      "ctoken",
    ]) {
      if (dispatcher.toLowerCase().includes(forbidden.toLowerCase())) {
        throw new Error(
          `dispatcher contains protocol-specific term ${forbidden}`,
        );
      }
    }
  });

  it("keeps the registry out of fund custody", () => {
    const registry = read("programs/registry/src/lib.rs");
    for (const forbidden of [
      "anchor_spl",
      "TokenAccount",
      "anchor_spl::token",
      "Transfer",
    ]) {
      if (registry.includes(forbidden)) {
        throw new Error(`registry contains custody/token term ${forbidden}`);
      }
    }
  });

  it("keeps strict fork tests separate from preflight tests", () => {
    const roundtrip = read("tests/mainnet-fork/roundtrip.spec.ts");
    if (!roundtrip.includes("forkTestsEnabled() ? describe : describe.skip")) {
      throw new Error("strict fork tests must remain explicitly gated");
    }
    if (!roundtrip.includes("rejects route while disabled")) {
      throw new Error(
        "strict fork suite must include disabled-adapter failure path",
      );
    }
    if (
      !roundtrip.includes(
        "deposits, reports value, and withdraws through the dispatcher",
      )
    ) {
      throw new Error(
        "strict fork suite must include routed lifecycle coverage",
      );
    }
  });

  it("exposes a single bounty submission gate command", () => {
    const pkg = JSON.parse(read("package.json")) as {
      scripts?: Record<string, string>;
    };
    if (!pkg.scripts?.["bounty:check"]) {
      throw new Error("package.json must define npm run bounty:check");
    }
    for (const script of [
      "keys:ids:check",
      "keys:sync",
      "keys:restore",
      "keys:restore:registry",
      "keys:verify",
      "keys:verify:registry",
      "deploy:registry:devnet",
      "verify:registry:devnet",
    ]) {
      if (!pkg.scripts?.[script]) {
        throw new Error(`package.json must define npm run ${script}`);
      }
    }
  });

  it("keeps the bounty gate strict about devnet and fork evidence", () => {
    const gate = read("scripts/bounty/check.ts");
    for (const required of [
      "verify:registry:devnet",
      "MAINNET_RPC_URL",
      "RUN_MAINNET_FORK_TESTS",
      "keys:ids:check",
      "anchor --version",
      "solana --version",
      "npm run test:fork",
    ]) {
      if (!gate.includes(required)) {
        throw new Error(`bounty gate is missing ${required}`);
      }
    }
  });

  it("does not require deploy keypair files for strict local fork loading", () => {
    const runner = read("scripts/mainnet-fork/run.ts");
    if (!runner.includes("--bpf-program")) {
      throw new Error("strict fork runner must load local programs by id");
    }
    if (runner.includes("program-id")) {
      throw new Error(
        "strict fork runner must not require deploy keypair files",
      );
    }
  });
});
