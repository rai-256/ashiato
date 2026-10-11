// SPDX-License-Identifier: AGPL-3.0-only
/**
 * **本人の決定（2026-10-12 表面の作り直し）を値で固定する**（docs/testing.md §3）。
 *
 * 正典は `docs/ui-direction.md` の「★ 2026-10-12 表面を作り直した」。そこに書かれた数と、
 * prompt 出力が「どの実装でも同じ値になるよう」に添えた導出済みの色を**リテラルで**持ち、
 * `tokens.ts` の値と突き合わせる。トークンを書き換えると、ここが落ちる。
 * 下限（4.5:1 / 3:1）は `text-contrast.test.ts` と `state-contrast.test.ts` が持つ。
 */
import { describe, expect, it } from "vitest";
import {
  ACCENT_HUE,
  ACCENT_SAT,
  DENSITY,
  FOCUS_RING_PX,
  RADIUS,
  SCHEMES,
  SPACE,
  SPARK_HUE,
  SURFACE_HUE,
  SURFACE_SAT,
  TYPE,
} from "../tokens";

describe("表面の決定（2026-10-12）", () => {
  it("色は 3 本: 地と面と文字 140°・18%、アクセント 307°・42%、差し色 150°", () => {
    expect([SURFACE_HUE, SURFACE_SAT]).toEqual([140, 18]);
    expect([ACCENT_HUE, ACCENT_SAT]).toEqual([307, 42]);
    expect(SPARK_HUE).toBe(150);
    for (const c of Object.values(SCHEMES)) {
      // 地・面・文字は同じ色相と彩度（文字の色みも同じ彩度）
      for (const k of ["ground", "surface1", "surface2", "text", "muted"] as const) {
        expect(c[k].slice(0, 2), k).toEqual([140, 18]);
      }
      expect(c.accent.slice(0, 2)).toEqual([307, 42]);
      expect(c.accentFill.slice(0, 2)).toEqual([307, 42]);
      expect(c.spark.slice(0, 2)).toEqual([150, 42]);
    }
  });

  it("prompt 出力が添えた主な色と同じ値（ライト）", () => {
    const c = SCHEMES.light;
    expect(c.ground).toEqual([140, 18, 99]); // 地はほぼ真っ白
    expect(c.surface1).toEqual([140, 18, 95]); // 行の面（面どうしの差は 4 ずつ）
    expect(c.surface2[2]).toBe(91);
    expect(c.text).toEqual([140, 18, 14]);
    expect(c.accent).toEqual([307, 42, 36]);
    expect(c.accentFill).toEqual([307, 42, 38]);
  });

  it("prompt 出力が添えた主な色と同じ値（ダーク）", () => {
    const c = SCHEMES.dark;
    expect(c.ground).toEqual([140, 18, 13]); // わずかに浮かせる
    expect(c.surface1).toEqual([140, 18, 19]);
    expect(c.text).toEqual([140, 18, 92]);
    expect(c.accent).toEqual([307, 42, 74.5]);
  });

  it("角は節 12 px・行 7 px", () => {
    expect(RADIUS.section).toBe(12);
    expect(RADIUS.row).toBe(7);
  });

  it("文字は本文 15.5 px・行間 1.70・比 1.18", () => {
    expect(TYPE.body).toBe(15.5);
    expect(TYPE.lineHeight).toBe(1.7);
    expect(TYPE.ratio).toBe(1.18);
    expect(TYPE.h2).toBeCloseTo(15.5 * 1.18, 1);
    expect(TYPE.h1).toBeCloseTo(15.5 * 1.18 * 1.18, 1);
  });

  it("密度 66。横に並ぶ操作の間は 8 px を下回らない", () => {
    expect(DENSITY).toBe(66);
    expect(SPACE.targetGap).toBeGreaterThanOrEqual(8);
  });

  it("フォーカスの輪は 3 px のまま（色は本文の色。`focusRule` が使う）", () => {
    expect(FOCUS_RING_PX).toBe(3);
  });
});
