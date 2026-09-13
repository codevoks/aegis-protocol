// I-TX-01 / INV-RES-06 (HARD acceptance test): for every realistic instruction transaction,
// construct the actual transaction/message with real signatures, real account metas, and a real
// blockhash lifetime, serialize with `@solana/kit`'s own transaction codec, and measure the exact
// serialized byte count -- never an approximation based on account count. Every one must fit the
// CLASSIC 1232-byte legacy transaction limit with NO address lookup table, regardless of RV-7's
// SIMD-0296 finding (`docs/ecosystem-research.md` §17).
//
// `liquidate` is measured for both the no-callback path and the callback path with a realistic
// callback account surface (item 19) -- this is the instruction with the largest account count in
// the protocol and the one place a real budget problem would first appear.

import {
  appendTransactionMessageInstructions,
  createTransactionMessage,
  generateKeyPairSigner,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type Address,
  type Instruction,
  type KeyPairSigner,
} from '@solana/kit';
import { blockhash } from '@solana/rpc-types';
import { getTransactionSize, isTransactionWithinSizeLimit } from '@solana/transactions';
import { getSetComputeUnitLimitInstruction } from '@solana-program/compute-budget';
import { TOKEN_PROGRAM_ADDRESS } from '@solana-program/token';
import { describe, expect, it } from 'vitest';
import {
  buildAccrueInterestInstruction,
  buildBorrowInstruction,
  buildClosePositionInstruction,
  buildDepositCollateralInstruction,
  buildInitPositionInstruction,
  buildLiquidateInstruction,
  buildRepayInstruction,
  buildSupplyInstruction,
  buildWithdrawCollateralInstruction,
  buildWithdrawInstruction,
} from '../src/ix.js';
import { PYTH_RECEIVER_PROGRAM_ADDRESS, SYSTEM_PROGRAM_ADDRESS } from '../src/config.js';

const LEGACY_TRANSACTION_SIZE_LIMIT = 1232;
// A placeholder, structurally valid (32-byte) blockhash -- byte size is identical to any real
// blockhash a live cluster would return, which is all this test needs (it never sends the
// transaction).
const FAKE_BLOCKHASH = blockhash('11111111111111111111111111111111');

async function fakeAddress(): Promise<Address> {
  return (await generateKeyPairSigner()).address;
}

async function realisticTransactionBytes(
  feePayer: KeyPairSigner,
  instructions: Instruction[],
): Promise<{ size: number; withinLimit: boolean }> {
  const computeBudgetIx = getSetComputeUnitLimitInstruction({ units: 600_000 });
  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(feePayer, m),
    (m) =>
      setTransactionMessageLifetimeUsingBlockhash(
        { blockhash: FAKE_BLOCKHASH, lastValidBlockHeight: 0n },
        m,
      ),
    (m) => appendTransactionMessageInstructions([computeBudgetIx, ...instructions], m),
  );
  const signed = await signTransactionMessageWithSigners(message);
  return { size: getTransactionSize(signed), withinLimit: isTransactionWithinSizeLimit(signed) };
}

interface Case {
  name: string;
  build: (payer: KeyPairSigner) => Promise<Instruction>;
}

async function commonAccounts() {
  return {
    protocol: await fakeAddress(),
    market: await fakeAddress(),
    position: await fakeAddress(),
    feePosition: await fakeAddress(),
    collateralVault: await fakeAddress(),
    loanVault: await fakeAddress(),
    ownerLoanAta: await fakeAddress(),
    ownerCollateralAta: await fakeAddress(),
    loanMint: await fakeAddress(),
    collateralMint: await fakeAddress(),
    collateralPriceUpdate: await fakeAddress(),
    loanPriceUpdate: await fakeAddress(),
  };
}

const cases: Case[] = [
  {
    name: 'init_position',
    build: async (payer) => {
      const a = await commonAccounts();
      return buildInitPositionInstruction(await fakeAddress(), {
        payer: payer.address,
        market: a.market,
        owner: payer.address,
        position: a.position,
        systemProgram: SYSTEM_PROGRAM_ADDRESS,
      });
    },
  },
  {
    name: 'deposit_collateral',
    build: async (payer) => {
      const a = await commonAccounts();
      return buildDepositCollateralInstruction(
        await fakeAddress(),
        {
          depositor: payer.address,
          market: a.market,
          position: a.position,
          collateralVault: a.collateralVault,
          depositorCollateralAta: a.ownerCollateralAta,
          collateralMint: a.collateralMint,
          collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
        },
        { amount: 1_000_000_000n },
      );
    },
  },
  {
    name: 'withdraw_collateral',
    build: async (payer) => {
      const a = await commonAccounts();
      return buildWithdrawCollateralInstruction(
        await fakeAddress(),
        {
          owner: payer.address,
          protocol: a.protocol,
          market: a.market,
          position: a.position,
          collateralVault: a.collateralVault,
          ownerCollateralAta: a.ownerCollateralAta,
          collateralMint: a.collateralMint,
          collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralPriceUpdate: a.collateralPriceUpdate,
          loanPriceUpdate: a.loanPriceUpdate,
        },
        { amount: 1_000_000_000n },
      );
    },
  },
  {
    name: 'supply',
    build: async (payer) => {
      const a = await commonAccounts();
      return buildSupplyInstruction(
        await fakeAddress(),
        {
          owner: payer.address,
          protocol: a.protocol,
          market: a.market,
          position: a.position,
          feePosition: a.feePosition,
          loanVault: a.loanVault,
          ownerLoanAta: a.ownerLoanAta,
          loanMint: a.loanMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
        },
        { assets: 1_000_000_000n, shares: 0n },
      );
    },
  },
  {
    name: 'withdraw',
    build: async (payer) => {
      const a = await commonAccounts();
      return buildWithdrawInstruction(
        await fakeAddress(),
        {
          owner: payer.address,
          protocol: a.protocol,
          market: a.market,
          position: a.position,
          feePosition: a.feePosition,
          loanVault: a.loanVault,
          ownerLoanAta: a.ownerLoanAta,
          loanMint: a.loanMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
        },
        { assets: 1_000_000_000n, shares: 0n },
      );
    },
  },
  {
    name: 'borrow',
    build: async (payer) => {
      const a = await commonAccounts();
      return buildBorrowInstruction(
        await fakeAddress(),
        {
          owner: payer.address,
          protocol: a.protocol,
          market: a.market,
          position: a.position,
          feePosition: a.feePosition,
          loanVault: a.loanVault,
          ownerLoanAta: a.ownerLoanAta,
          loanMint: a.loanMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralPriceUpdate: a.collateralPriceUpdate,
          loanPriceUpdate: a.loanPriceUpdate,
        },
        { assets: 900_000_000n, shares: 0n },
      );
    },
  },
  {
    name: 'repay',
    build: async (payer) => {
      const a = await commonAccounts();
      return buildRepayInstruction(
        await fakeAddress(),
        {
          payer: payer.address,
          market: a.market,
          position: a.position,
          feePosition: a.feePosition,
          loanVault: a.loanVault,
          payerLoanAta: a.ownerLoanAta,
          loanMint: a.loanMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
        },
        { assets: 900_000_000n, shares: 0n },
      );
    },
  },
  {
    name: 'accrue_interest',
    build: async () => {
      const a = await commonAccounts();
      return buildAccrueInterestInstruction(await fakeAddress(), {
        market: a.market,
        feePosition: a.feePosition,
      });
    },
  },
  {
    name: 'close_position',
    build: async (payer) => {
      const a = await commonAccounts();
      return buildClosePositionInstruction(await fakeAddress(), {
        owner: payer.address,
        market: a.market,
        position: a.position,
      });
    },
  },
  {
    name: 'liquidate (no callback)',
    build: async (payer) => {
      const a = await commonAccounts();
      return buildLiquidateInstruction(
        await fakeAddress(),
        {
          liquidator: payer.address,
          protocol: a.protocol,
          market: a.market,
          position: a.position,
          feePosition: a.feePosition,
          loanVault: a.loanVault,
          collateralVault: a.collateralVault,
          liquidatorLoanAta: a.ownerLoanAta,
          liquidatorCollateralAta: a.ownerCollateralAta,
          loanMint: a.loanMint,
          collateralMint: a.collateralMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralPriceUpdate: a.collateralPriceUpdate,
          loanPriceUpdate: a.loanPriceUpdate,
          // callbackProgram/callbackCollateralAccount omitted -> optional-account sentinel
        },
        { repayAssets: 900_000_000n, seizeCollateral: 0n, callbackData: new Uint8Array() },
      );
    },
  },
  {
    name: 'liquidate (callback, realistic account surface)',
    build: async (payer) => {
      const a = await commonAccounts();
      const callbackProgram = await fakeAddress();
      const callbackCollateralAccount = await fakeAddress();
      // A realistic callback's own remaining_accounts surface (ADR-0013 §1.4): its authority PDA
      // plus a reserve token account, mirroring `labs/example-liquidator`'s real account list
      // (`bots/liquidator/src/demo.ts`'s `remainingAccounts`).
      const remainingAccounts = [
        { address: await fakeAddress(), writable: false }, // callback authority PDA
        { address: await fakeAddress(), writable: true }, // callback loan-asset reserve
      ];
      return buildLiquidateInstruction(
        await fakeAddress(),
        {
          liquidator: payer.address,
          protocol: a.protocol,
          market: a.market,
          position: a.position,
          feePosition: a.feePosition,
          loanVault: a.loanVault,
          collateralVault: a.collateralVault,
          liquidatorLoanAta: a.ownerLoanAta,
          liquidatorCollateralAta: a.ownerCollateralAta,
          loanMint: a.loanMint,
          collateralMint: a.collateralMint,
          loanTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralTokenProgram: TOKEN_PROGRAM_ADDRESS,
          collateralPriceUpdate: a.collateralPriceUpdate,
          loanPriceUpdate: a.loanPriceUpdate,
          callbackProgram,
          callbackCollateralAccount,
        },
        { repayAssets: 900_000_000n, seizeCollateral: 0n, callbackData: new Uint8Array(32) },
        remainingAccounts,
      );
    },
  },
];

describe('I-TX-01 / INV-RES-06 -- every instruction fits 1232 bytes, no ALT', () => {
  const results: { name: string; bytes: number; ok: boolean }[] = [];

  for (const c of cases) {
    it(`${c.name} fits the classic 1232-byte legacy transaction limit`, async () => {
      const payer = await generateKeyPairSigner();
      const ix = await c.build(payer);
      const { size, withinLimit } = await realisticTransactionBytes(payer, [ix]);
      results.push({ name: c.name, bytes: size, ok: size <= LEGACY_TRANSACTION_SIZE_LIMIT });
      expect(withinLimit).toBe(true);
      expect(size).toBeLessThanOrEqual(LEGACY_TRANSACTION_SIZE_LIMIT);
    });
  }

  it('prints the transaction-size table used in the Phase 9 report', () => {
    // Populated by the `it` blocks above (vitest runs a describe block's tests in declaration
    // order); printed here, last, purely for human-readable evidence in CI/test output.
    // eslint-disable-next-line no-console
    console.log('\nInstruction | Serialized bytes | <=1232?');
    console.log('---|---|---');
    for (const r of results) {
      console.log(`${r.name} | ${r.bytes} | ${r.ok ? 'yes' : 'NO'}`);
    }
    expect(results.length).toBeGreaterThan(0);
  });
});
