// GENERATED FILE -- do not edit by hand.
// Produced by sdk/ts/scripts/codegen.mjs from target/idl/aegis.json.
// Re-run `npm run codegen` (sdk/ts/) after `anchor build` changes the IDL; `npm run codegen:check` detects a stale commit.


import type { FieldSpec } from '../borshValues.js';

export type { BadDebtAbsorbed, Borrowed, CollateralDeposited, CollateralFeesWithdrawn, CollateralWithdrawn, InterestAccrued, Liquidated, MarketCreated, PositionClosed, PositionInitialized, ProtocolInitialized, Repaid, Supplied, Withdrawn } from './types.js';
import { BADDEBTABSORBED_FIELDS, BORROWED_FIELDS, COLLATERALDEPOSITED_FIELDS, COLLATERALFEESWITHDRAWN_FIELDS, COLLATERALWITHDRAWN_FIELDS, INTERESTACCRUED_FIELDS, LIQUIDATED_FIELDS, MARKETCREATED_FIELDS, POSITIONCLOSED_FIELDS, POSITIONINITIALIZED_FIELDS, PROTOCOLINITIALIZED_FIELDS, REPAID_FIELDS, SUPPLIED_FIELDS, WITHDRAWN_FIELDS } from './types.js';

export const EVENT_DISCRIMINATORS: Record<string, Uint8Array> = {
  "BadDebtAbsorbed": Uint8Array.from([79, 7, 46, 220, 149, 148, 250, 170]),
  "Borrowed": Uint8Array.from([225, 182, 241, 78, 34, 145, 253, 230]),
  "CollateralDeposited": Uint8Array.from([244, 62, 77, 11, 135, 112, 61, 96]),
  "CollateralFeesWithdrawn": Uint8Array.from([221, 146, 167, 114, 246, 136, 215, 194]),
  "CollateralWithdrawn": Uint8Array.from([51, 224, 133, 106, 74, 173, 72, 82]),
  "InterestAccrued": Uint8Array.from([79, 218, 196, 73, 32, 148, 138, 71]),
  "Liquidated": Uint8Array.from([231, 57, 55, 75, 0, 170, 246, 68]),
  "MarketCreated": Uint8Array.from([88, 184, 130, 231, 226, 84, 6, 58]),
  "PositionClosed": Uint8Array.from([157, 163, 227, 228, 13, 97, 138, 121]),
  "PositionInitialized": Uint8Array.from([105, 129, 56, 195, 211, 51, 160, 231]),
  "ProtocolInitialized": Uint8Array.from([173, 122, 168, 254, 9, 118, 76, 132]),
  "Repaid": Uint8Array.from([38, 248, 231, 7, 150, 164, 172, 23]),
  "Supplied": Uint8Array.from([137, 114, 239, 72, 162, 75, 133, 39]),
  "Withdrawn": Uint8Array.from([20, 89, 223, 198, 194, 124, 219, 13]),
};

export const EVENT_FIELD_SPECS: Record<string, readonly FieldSpec[]> = {
  "BadDebtAbsorbed": BADDEBTABSORBED_FIELDS,
  "Borrowed": BORROWED_FIELDS,
  "CollateralDeposited": COLLATERALDEPOSITED_FIELDS,
  "CollateralFeesWithdrawn": COLLATERALFEESWITHDRAWN_FIELDS,
  "CollateralWithdrawn": COLLATERALWITHDRAWN_FIELDS,
  "InterestAccrued": INTERESTACCRUED_FIELDS,
  "Liquidated": LIQUIDATED_FIELDS,
  "MarketCreated": MARKETCREATED_FIELDS,
  "PositionClosed": POSITIONCLOSED_FIELDS,
  "PositionInitialized": POSITIONINITIALIZED_FIELDS,
  "ProtocolInitialized": PROTOCOLINITIALIZED_FIELDS,
  "Repaid": REPAID_FIELDS,
  "Supplied": SUPPLIED_FIELDS,
  "Withdrawn": WITHDRAWN_FIELDS,
};
