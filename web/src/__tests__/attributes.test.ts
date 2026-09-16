// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 主張の原文の組み立てと、`/ingest` の応答の読み方（ST19 / tasks 4.1 / design D4 / D9）。
 *
 * **乱数の強さと導き方はここでしか見られない**（design D4）—— Rust のテストは
 * 画面の組み立てを呼べないので、「乱数が識別子と一致しない」「同じ内容でも異なる」は
 * この側で固定し、「乱数が解析済みに写らない」「鍵を作り直せない」は Rust の結合テストが持つ。
 */
import { describe, expect, it } from "vitest";
import {
  buildClaim,
  emptyInput,
  isAttributesView,
  newNonce,
  readIngestResponse,
  rejectionMessage,
  UNREACHABLE_MESSAGE,
  validFromLabel,
  validFromOf,
  valueLabel,
  type ClaimInput,
} from "../attributes";

/** design D4 の下限（128 bit を base64url で書いた 22 文字）。**リテラルで持つ**（実装と一緒に動かない）。 */
const NONCE_MIN_CHARS = 22;

const ID = "11111111-1111-4111-8111-111111111111";
const KIND = "22222222-2222-4222-8222-222222222222";

function input(over: Partial<ClaimInput> = {}): ClaimInput {
  return { ...emptyInput(KIND), value: "東京都 目黒区", year: "2019", month: "10", ...over };
}

describe("主張の原文", () => {
  // Scenario: 同じ内容の 2 つの主張は別々の乱数を持つ
  it("識別子と主張した日時と値といつからが同じでも、乱数は毎回違い、識別子とも一致しない", () => {
    const now = new Date("2026-09-15T02:00:00Z");
    const a = JSON.parse(buildClaim(input(), now, ID, newNonce()).raw) as { nonce: string };
    const b = JSON.parse(buildClaim(input(), now, ID, newNonce()).raw) as { nonce: string };

    for (const [who, n] of [["1 つ目", a.nonce], ["2 つ目", b.nonce]] as const) {
      expect(n.length, `${who} の乱数が ${n.length} 文字`).toBeGreaterThanOrEqual(NONCE_MIN_CHARS);
      // **識別子から導くと、消去の後に残る `id` の列から乱数が分かる**（design D4）
      expect(n, `${who} の乱数が識別子と一致している`).not.toBe(ID);
      expect(ID, `${who} の乱数が識別子に含まれている`).not.toContain(n);
      expect(n).toMatch(/^[A-Za-z0-9_-]+$/);
    }
    expect(a.nonce, "同じ内容の 2 つの主張が同じ乱数を持っている").not.toBe(b.nonce);
  });

  it("原文は design D1 の形で、主張した日時は押した時刻", () => {
    const now = new Date("2026-09-15T02:00:00Z");
    const built = buildClaim(input({ note: "転職に合わせて" }), now, ID, newNonce());
    const raw = JSON.parse(built.raw) as Record<string, unknown>;
    expect(raw.claim).toBe(ID);
    expect(raw.kind).toBe(KIND);
    expect(raw.value).toBe("東京都 目黒区");
    expect(raw.valid_from).toEqual({ precision: "month", date: "2019-10" });
    expect(raw.supersedes).toBeNull();
    expect(raw.note).toBe("転職に合わせて");
    // **本人が入力する欄を置かない**（深掘り C4）—— 押した時刻がそのまま出来事の時刻になる
    expect(built.assertedAt).toBe("2026-09-15T02:00:00.000Z");
  });

  it("入力が同じなら同じ原文になり、1 か所でも変われば組み直される", () => {
    const now = new Date("2026-09-15T02:00:00Z");
    const nonce = newNonce();
    expect(buildClaim(input(), now, ID, nonce).raw).toBe(buildClaim(input(), now, ID, nonce).raw);
    expect(buildClaim(input({ value: "大阪府" }), now, ID, nonce).raw).not.toBe(
      buildClaim(input(), now, ID, nonce).raw,
    );
  });

  it("「なし」は空文字ではなく null として送られる", () => {
    const raw = JSON.parse(buildClaim(input({ value: null }), new Date(), ID, newNonce()).raw) as {
      value: unknown;
    };
    expect(raw.value).toBeNull();
  });

  it("空の補足は null（空文字を残すと「補足を書いた」と区別できない）", () => {
    const raw = JSON.parse(buildClaim(input({ note: "   " }), new Date(), ID, newNonce()).raw) as {
      note: unknown;
    };
    expect(raw.note).toBeNull();
  });
});

describe("いつから", () => {
  it("精度ごとに欄を組み、欠けていれば null（サーバが断る）", () => {
    expect(validFromOf(input({ precision: "year" }))).toEqual({ precision: "year", date: "2019" });
    expect(validFromOf(input({ precision: "month" }))).toEqual({ precision: "month", date: "2019-10" });
    expect(validFromOf(input({ precision: "day", day: "5" }))).toEqual({ precision: "day", date: "2019-10-05" });
    expect(validFromOf(input({ precision: "unknown" }))).toEqual({ precision: "unknown", date: null });
    expect(validFromOf(input({ precision: "month", month: "" })).date).toBeNull();
  });

  it("**精度を丸めない**（年だけの主張を「1 月 1 日」と書かない）", () => {
    expect(validFromLabel({ precision: "year", date: "2019" })).toBe("2019 年から");
    expect(validFromLabel({ precision: "month", date: "2019-10" })).toBe("2019 年 10 月から");
    expect(validFromLabel({ precision: "day", date: "2019-10-05" })).toBe("2019 年 10 月 5 日から");
    expect(validFromLabel({ precision: "unknown", date: null })).toBe("いつからかは分からない");
  });

  it("「なし」と「まだ書いていない」を混ぜない", () => {
    expect(valueLabel(null)).toBe("なし");
    expect(valueLabel("東京都")).toBe("東京都");
  });
});

describe("/ingest の応答の読み方", () => {
  const res = (status: number, body: unknown): Response =>
    ({ ok: status < 400, status, json: () => Promise.resolve(body) }) as Response;

  it("**400 でも本文の 1 件ごとの結果を読む**（1 件だけ送って断られると 400 が返る）", async () => {
    const out = await readIngestResponse(res(400, [{ accepted: false, error: "invalid_claim_value" }]));
    expect(out).toEqual({ at: "rejected", kind: "invalid_claim_value" });
  });

  it("200 の受理を読む", async () => {
    expect(await readIngestResponse(res(200, [{ accepted: true, error: null }]))).toEqual({ at: "accepted" });
  });

  it("本文が読めない・5xx・401 は「届かなかった」", async () => {
    expect(await readIngestResponse(res(500, []))).toEqual({ at: "unreachable" });
    expect(await readIngestResponse(res(401, []))).toEqual({ at: "unreachable" });
    expect(await readIngestResponse(res(200, "not an array"))).toEqual({ at: "unreachable" });
    const broken = { ok: true, status: 200, json: () => Promise.reject(new Error("x")) } as unknown as Response;
    expect(await readIngestResponse(broken)).toEqual({ at: "unreachable" });
  });

  it("種別ごとに違う文が出て、届かなかったときの文はそのどれとも違う", () => {
    const kinds = ["invalid_claim_value", "invalid_valid_from", "unknown_attribute_kind", "invalid_supersedes"];
    const messages = kinds.map(rejectionMessage);
    expect(new Set(messages).size, "種別が違うのに同じ文が出ている").toBe(kinds.length);
    for (const m of messages) expect(m).not.toBe(UNREACHABLE_MESSAGE);
    expect(rejectionMessage("id_reused")).toContain("id_reused");
  });
});

describe("応答の形の検査", () => {
  const view = {
    today: "2026-09-15",
    kinds: [
      {
        id: KIND,
        name: "住所",
        current: null,
        upcoming: [],
        claims: [],
        superseded: [],
      },
    ],
  };

  it("形が合えば通る", () => {
    expect(isAttributesView(view)).toBe(true);
  });

  it("**形が違えば失敗として出す**（型の宣言だけで通すと描画で落ちて画面が白くなる）", () => {
    expect(isAttributesView(null)).toBe(false);
    expect(isAttributesView({ today: "2026-09-15" })).toBe(false);
    expect(isAttributesView({ ...view, kinds: [{ id: KIND }] })).toBe(false);
    expect(
      isAttributesView({
        ...view,
        kinds: [{ ...view.kinds[0], claims: [{ id: "x", value: "v", valid_from: { precision: "century", date: null }, asserted_at: "t" }] }],
      }),
      "知らない精度が通っている",
    ).toBe(false);
  });
});
