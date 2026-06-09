import { PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import {
  DRIFT_IF_ADAPTER_PROGRAM_ID,
  JUPITER_JLP_ADAPTER_PROGRAM_ID,
  KAMINO_USDC_ADAPTER_PROGRAM_ID,
  MAPLE_SYRUP_ADAPTER_PROGRAM_ID,
  MARGINFI_USDC_ADAPTER_PROGRAM_ID,
  USDC_MINT,
} from "../../sdk/src/index.js";

export const TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

export const KAMINO = {
  adapter: KAMINO_USDC_ADAPTER_PROGRAM_ID,
  protocolVariant: 1,
  program: new PublicKey("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD"),
  reserve: new PublicKey("D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59"),
  lendingMarket: new PublicKey("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF"),
  reserveLiquiditySupply: new PublicKey("Bgq7trRgVMeq33yt235zM2onQ4bRDBsY5EWiTetF4qw6"),
  reserveCollateralMint: new PublicKey("B8V6WVjPxW1UGwVDfxH2d2r8SyT4cqn7dQRK6XneVa7D"),
} as const;

export function kaminoMarketAuthority(): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("lma"), KAMINO.lendingMarket.toBuffer()],
    KAMINO.program,
  )[0];
}

export const MARGINFI = {
  adapter: MARGINFI_USDC_ADAPTER_PROGRAM_ID,
  protocolVariant: 2,
  program: new PublicKey("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA"),
  group: new PublicKey("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8"),
  usdcBank: new PublicKey("2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB"),
  liquidityVault: new PublicKey("7jaiZR5Sk8hdYN9MxTpczTcwbWpb5WEoxSANuUwveuat"),
  liquidityVaultAuthority: new PublicKey("3uxNepDbmkDNq6JhRja5Z8QwbTrfmkKP8AKZV5chYDGG"),
  oracle: new PublicKey("Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX"),
} as const;

export function marginfiAccount(authority: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("marginfi_account"),
      MARGINFI.group.toBuffer(),
      authority.toBuffer(),
      Buffer.from([0, 0]),
      Buffer.from([0, 0]),
    ],
    MARGINFI.program,
  )[0];
}

export const JUPITER = {
  adapter: JUPITER_JLP_ADAPTER_PROGRAM_ID,
  protocolVariant: 3,
  program: new PublicKey("PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu"),
  perpetuals: new PublicKey("H4ND9aYttUVLFmNypZqLjZ52FYiGvdEB45GmwNoKEjTj"),
  transferAuthority: new PublicKey("AVzP2GeRmqGphJsMxWoqjpUifPpCret7LqWhD8NWQK49"),
  eventAuthority: new PublicKey("37hJBDnntwqhGbK7L6M1bLyvccj4u55CCUiLPdYkiqBN"),
  pool: new PublicKey("5BUwFW4nRbftYTDMbgxykoFWqWHPzahFSNAaaaJtVKsq"),
  usdcCustody: new PublicKey("G18jKKXQwBbrHeiK3C9MRXhkHsLHf7XgCSisykV46EZa"),
  custodyTokenAccount: new PublicKey("WzWUoCmtVv7eqAbU3BfKPU3fhLP6CXR8NCJH78UK9VS"),
  usdcDovesPrice: new PublicKey("6Jp2xZUTWdDD2ZyUPRzeMdc6AFQ5K3pFgZxk2EijfjnM"),
  usdcPythnetPrice: new PublicKey("Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX"),
  jlpMint: new PublicKey("27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4"),
  custodies: [
    new PublicKey("7xS2gz2bTp3fwCC7knJvUWTEU9Tycczu6VhJYKgi1wdz"),
    new PublicKey("AQCGyheWPLeo6Qp9WpYS9m3Qj479t7R636N9ey1rEjEn"),
    new PublicKey("5Pv3gM9JrFFH883SWAhvJC9RPYmo8UNxuFtv5bMMALkm"),
    new PublicKey("G18jKKXQwBbrHeiK3C9MRXhkHsLHf7XgCSisykV46EZa"),
    new PublicKey("4vkNeXiYEUizLdrpdPS1eC2mccyM4NUPRtERrk6ZETkk"),
  ],
  dovesAgPrices: [
    new PublicKey("FYq2BWQ1V5P1WFBqr3qB2Kb5yHVvSv7upzKodgQE5zXh"),
    new PublicKey("AFZnHPzy4mvVCffrVwhewHbFc93uTHvDSFrVH7GtfXF1"),
    new PublicKey("hUqAT1KQ7eW1i6Csp9CXYtpPfSAvi835V7wKi5fRfmC"),
    new PublicKey("6Jp2xZUTWdDD2ZyUPRzeMdc6AFQ5K3pFgZxk2EijfjnM"),
    new PublicKey("Fgc93D641F8N2d1xLjQ4jmShuD3GE3BsCXA56KBQbF5u"),
  ],
} as const;

export const MAPLE = {
  adapter: MAPLE_SYRUP_ADAPTER_PROGRAM_ID,
  protocolVariant: 4,
  program: new PublicKey("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"),
  syrupMint: new PublicKey("AvZZF1YaZDziPY2RCK4oJrRVrbN3mTD9NL24hPeaZeUj"),
  whirlpool: new PublicKey("6fteKNvMdv7tYmBoJHhj1jx6rHcEwC6RdSEmVpyS613J"),
  tokenVaultA: new PublicKey("FM2RuqFYo9umA1yc5FyQn6pSDZJZ1MXAdaekJZ4dQCvi"),
  tokenVaultB: new PublicKey("Fw6Xr45rBBrXbWJd5ZbSg44kacrKRLef4rHkZ8gWC5Ab"),
  tickArray0: new PublicKey("4yRC9NUHB2dwxfZyrqA8dDqH8GkcUVKU5F7W3ZPnbQtd"),
  buyTickArray1: new PublicKey("AdLyWhs7xrwkBFCYEo3n9BiwgXMZzXMefh8K9wMWoy1j"),
  buyTickArray2: new PublicKey("AofDEAkfQxcyeochNwxyQehYm6SpL3qrtxm7ZEZtPptp"),
  sellTickArray1: new PublicKey("9qUH5rp6Xw7NqghvbR9eQu6xTjEu5QTCHMbjdiiDVd5S"),
  sellTickArray2: new PublicKey("BQ95wDV5A7z4c9cExYMWE2KvcqhbdjoxXcoQ88erFtyH"),
} as const;

export function mapleOracle(): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("oracle"), MAPLE.whirlpool.toBuffer()],
    MAPLE.program,
  )[0];
}

export const DRIFT = {
  adapter: DRIFT_IF_ADAPTER_PROGRAM_ID,
  protocolVariant: 5,
  program: new PublicKey("dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH"),
} as const;

const MARKET_INDEX_0 = Buffer.from([0, 0]);

function driftPda(seeds: Buffer[]): PublicKey {
  return PublicKey.findProgramAddressSync(seeds, DRIFT.program)[0];
}

export function driftState(): PublicKey {
  return driftPda([Buffer.from("drift_state")]);
}

export function driftSigner(): PublicKey {
  return driftPda([Buffer.from("drift_signer")]);
}

export function driftSpotMarket(): PublicKey {
  return driftPda([Buffer.from("spot_market"), MARKET_INDEX_0]);
}

export function driftSpotMarketVault(): PublicKey {
  return driftPda([Buffer.from("spot_market_vault"), MARKET_INDEX_0]);
}

export function driftInsuranceFundVault(): PublicKey {
  return driftPda([Buffer.from("insurance_fund_vault"), MARKET_INDEX_0]);
}

export function driftUserStats(authority: PublicKey): PublicKey {
  return driftPda([Buffer.from("user_stats"), authority.toBuffer()]);
}

export function driftInsuranceFundStake(authority: PublicKey): PublicKey {
  return driftPda([Buffer.from("insurance_fund_stake"), authority.toBuffer(), MARKET_INDEX_0]);
}

export const EXTERNAL_PROGRAMS = [
  KAMINO.program,
  MARGINFI.program,
  JUPITER.program,
  MAPLE.program,
  DRIFT.program,
] as const;

export const MAINNET_CLONES = uniquePubkeys([
  USDC_MINT,
  KAMINO.reserve,
  KAMINO.lendingMarket,
  KAMINO.reserveLiquiditySupply,
  KAMINO.reserveCollateralMint,
  MARGINFI.group,
  MARGINFI.usdcBank,
  MARGINFI.liquidityVault,
  MARGINFI.oracle,
  JUPITER.perpetuals,
  JUPITER.transferAuthority,
  JUPITER.pool,
  JUPITER.usdcCustody,
  JUPITER.custodyTokenAccount,
  JUPITER.usdcDovesPrice,
  JUPITER.usdcPythnetPrice,
  JUPITER.jlpMint,
  ...JUPITER.custodies,
  ...JUPITER.dovesAgPrices,
  MAPLE.syrupMint,
  MAPLE.whirlpool,
  MAPLE.tokenVaultA,
  MAPLE.tokenVaultB,
  MAPLE.tickArray0,
  MAPLE.buyTickArray1,
  MAPLE.buyTickArray2,
  MAPLE.sellTickArray1,
  MAPLE.sellTickArray2,
  mapleOracle(),
  driftState(),
  driftSpotMarket(),
  driftSpotMarketVault(),
  driftInsuranceFundVault(),
]);

export const PATCHED_DOVES_ACCOUNTS = uniquePubkeys([
  JUPITER.usdcDovesPrice,
  ...JUPITER.dovesAgPrices,
]);

export const SYSVAR_ACCOUNTS = {
  instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
  rent: SYSVAR_RENT_PUBKEY,
} as const;

function uniquePubkeys(keys: readonly PublicKey[]): PublicKey[] {
  const seen = new Set<string>();
  const out: PublicKey[] = [];
  for (const key of keys) {
    const id = key.toBase58();
    if (!seen.has(id)) {
      seen.add(id);
      out.push(key);
    }
  }
  return out;
}
