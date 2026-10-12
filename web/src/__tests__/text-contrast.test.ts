// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 文字の対比（NFR-18。ライト・ダークの両方）。
 *
 * **文字に使うトークンすべて × 文字が乗りうる面すべて**を、描くのと同じ値から計算する。
 * 1 組だけ測って済ませると、別の面に乗った文字が割れていても気付かない（ui-direction の UIR-13 と同じ型）。
 * 2026-10-12 の表面で文字の色は 4 つ（本文・控えめ・アクセント・差し色）、面は 3 段になった。
 *
 * **セルには掛からない** —— 本文でも文字画像でもないため（design D10）。格子は `state-contrast.test.ts`。
 */
import { describe, expect, it } from "vitest";
import { toneContrast } from "../contrast";
import { SCHEMES, type Scheme } from "../tokens";

/** NFR-18 の数値を**リテラルで持つ**（トークンと一緒に動かない。ST02 の review R4）。 */
const NFR18_TEXT = 4.5;

const SCHEME_NAMES = Object.keys(SCHEMES) as Scheme[];
/** 文字に使うトークン。**足したらここに足す**（下の「数え漏れ」が止める） */
const TEXT_TOKENS = ["text", "muted", "accent", "spark"] as const;
/** 文字が乗りうる面（地・行と節・詳細とボタンと入力欄） */
const FACES = ["ground", "surface1", "surface2"] as const;

describe("文字の対比", () => {
  it("文字のトークンすべてが、面の 3 段すべての上で 4.5:1 以上（ライト・ダーク）", () => {
    for (const scheme of SCHEME_NAMES) {
      const c = SCHEMES[scheme];
      for (const face of FACES) {
        for (const kind of TEXT_TOKENS) {
          const r = toneContrast(c[kind], c[face]);
          expect(r, `${scheme} の ${kind} が ${face} の上で ${r.toFixed(3)}:1`).toBeGreaterThanOrEqual(NFR18_TEXT);
        }
      }
    }
  });

  it("主ボタンの塗りの上の文字も 4.5:1 以上（ライト・ダーク）", () => {
    for (const scheme of SCHEME_NAMES) {
      const c = SCHEMES[scheme];
      const r = toneContrast(c.onAccent, c.accentFill);
      expect(r, `${scheme} の主ボタンの文字が ${r.toFixed(3)}:1`).toBeGreaterThanOrEqual(NFR18_TEXT);
    }
  });

  it("数え漏れが無い（トークンを足したら上の一覧にも足す）", () => {
    // 文字にも面にも塗りにも使わないトークン（区切りの線と格子）を除いた全部が、上のどこかで測られている
    const measured = new Set<string>([...TEXT_TOKENS, ...FACES, "onAccent", "accentFill"]);
    const notText = new Set(["line", "cell"]);
    for (const scheme of SCHEME_NAMES) {
      for (const key of Object.keys(SCHEMES[scheme])) {
        if (notText.has(key)) continue;
        expect(measured.has(key), `${scheme}.${key} がどの組でも測られていない`).toBe(true);
      }
    }
  });
});
