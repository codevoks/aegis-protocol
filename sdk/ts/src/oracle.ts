// Oracle client abstraction (`docs/phases/phase-09-sdk-ui.md` item 14, mirroring
// `docs/oracle-design.md` §1's on-chain `PriceSource` trait on the client side).
//
// Hermes/network access is injectable and optional: `HermesOracleClient` is the network-capable
// implementer; `FixtureOracleClient` is a deterministic, fully offline implementer requiring no
// internet access at all. Phase 9 acceptance (the local app/demo) uses ONLY the fixture client --
// no Hermes endpoint is required, and none is hardcoded anywhere in this file. No API key is
// accepted or needed by either client (`docs/oracle-design.md` §8: reading a Pyth price update is
// an account read on-chain, never a CPI; fetching one off-chain to *post* is a separate, optional
// concern this SDK does not require).

export interface PriceQuote {
  /** 32-byte Pyth feed id. */
  feedId: Uint8Array;
  /** Raw integer mantissa, as Pyth publishes it (scale by `10^expo` to get a human price). */
  price: bigint;
  /** Raw confidence interval, same scale as `price`. */
  conf: bigint;
  expo: number;
  /** Unix seconds. */
  publishTime: bigint;
}

export interface OracleClient {
  getLatestPrice(feedId: Uint8Array): Promise<PriceQuote>;
}

function feedIdHex(feedId: Uint8Array): string {
  return Buffer.from(feedId).toString('hex');
}

/**
 * Network-capable Hermes client. The endpoint is always caller-supplied -- never hardcoded here,
 * and this SDK does not default to any specific Hermes host (the legacy keyless
 * `hermes.pyth.network` endpoint requires an API key as of `docs/ecosystem-research.md` §4; this
 * client accepts whichever endpoint and headers the caller has configured, and requires none by
 * default). Not exercised by any required Phase 9 test or demo path.
 */
export class HermesOracleClient implements OracleClient {
  constructor(
    private readonly endpoint: string,
    private readonly headers: Record<string, string> = {},
  ) {}

  async getLatestPrice(feedId: Uint8Array): Promise<PriceQuote> {
    const id = feedIdHex(feedId);
    const url = `${this.endpoint.replace(/\/$/, '')}/v2/updates/price/latest?ids[]=${id}`;
    const resp = await fetch(url, { headers: this.headers });
    if (!resp.ok) {
      throw new Error(`Hermes request failed: ${resp.status} ${resp.statusText}`);
    }
    const body = (await resp.json()) as {
      parsed?: { id: string; price: { price: string; conf: string; expo: number; publish_time: number } }[];
    };
    const parsed = body.parsed?.find((p) => p.id === id);
    if (!parsed) throw new Error(`Hermes response did not include feed ${id}`);
    return {
      feedId,
      price: BigInt(parsed.price.price),
      conf: BigInt(parsed.price.conf),
      expo: parsed.price.expo,
      publishTime: BigInt(parsed.price.publish_time),
    };
  }
}

/**
 * Deterministic, fully offline implementer. This is what the local app/demo uses exclusively --
 * no internet access, no API key, no external process. Prices are set explicitly by the caller
 * (e.g. from `docs/phases/phase-09-sdk-ui.md`'s scripted price-drop demo) and returned verbatim.
 */
export class FixtureOracleClient implements OracleClient {
  private readonly prices = new Map<string, PriceQuote>();

  setPrice(quote: PriceQuote): void {
    this.prices.set(feedIdHex(quote.feedId), quote);
  }

  async getLatestPrice(feedId: Uint8Array): Promise<PriceQuote> {
    const quote = this.prices.get(feedIdHex(feedId));
    if (!quote) throw new Error(`FixtureOracleClient has no price set for feed ${feedIdHex(feedId)}`);
    return quote;
  }
}
