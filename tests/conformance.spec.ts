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
  });
});
