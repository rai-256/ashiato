// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 毎日の気分の原文の組み立て・数え方・前後・暦（ST17 / tasks 7.1 / design D4 / D12 / D14）。
 *
 * **乱数の強さと導き方はここでしか見られない**（design D4）—— Rust の試験は画面の組み立てを呼べない。
 * 定数は実装から import せず**リテラルで持つ**（実装と一緒に動かない。`docs/testing.md` §3）。
 */
import { describe, expect, it } from "vitest";
import {
  buildFeeling,
  CALENDAR_DAYS,
  calendarDays,
  countNoteChars,
  hasControlChar,
  dayMark,
  NOTE_MAX_CHARS,
  phaseOf,
  type FeelingTarget,
} from "../feelings";

const DAY: FeelingTarget = { kind: "day", date: "2026-10-07" };
const NOW = new Date("2026-10-07T12:30:05.123Z");
const SCALES = [{ id: "valence", version: 1, value: 1 }];

describe("定数", () => {
  it("本人の決定と仮決めの値", () => {
    expect(NOTE_MAX_CHARS).toBe(2000);
    expect(CALENDAR_DAYS).toBe(30);
  });
});

describe("原文の組み立て", () => {
  // Scenario: 同じ内容の 2 つの主観は別々の乱数を持つ
  it("識別子と記入の日時と紐づけ先と尺度が同じでも、乱数は毎回違い、128 bit で、識別子とも一致しない", () => {
    const id = "11111111-1111-4111-8111-111111111111";
    const a = buildFeeling(DAY, SCALES, null, NOW, id);
    const b = buildFeeling(DAY, SCALES, null, NOW, id);
    const na = (JSON.parse(a.raw) as { nonce: string }).nonce;
    const nb = (JSON.parse(b.raw) as { nonce: string }).nonce;
    for (const n of [na, nb]) {
      // 128 bit = 16 バイト = base64url で 22 文字ちょうど
      expect(n).toMatch(/^[A-Za-z0-9_-]{22}$/);
      expect(n).not.toBe(id);
      expect(id).not.toContain(n);
    }
    expect(na).not.toBe(nb);
    expect(a.id).toBe(id);
  });

  it("原文は design D1 の形で、識別子は feeling と一致し、ひとこと無しは null", () => {
    const built = buildFeeling(DAY, SCALES, "   ", NOW);
    const raw = JSON.parse(built.raw) as Record<string, unknown>;
    expect(raw.feeling).toBe(built.id);
    expect(raw.target).toEqual({ kind: "day", date: "2026-10-07" });
    expect(raw.scales).toEqual(SCALES);
    expect(raw.note).toBeNull();
    expect(raw.written_at).toMatch(/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}[+-]\d\d:\d\d$/);
    expect(typeof raw.written_tz).toBe("string");
    expect(new Date(raw.written_at as string).getTime()).toBe(NOW.getTime());
  });

  it("出来事の時刻は対象の時刻（日は JST の 0 時、滞在は始まり）", () => {
    expect(buildFeeling(DAY, SCALES, "ひとこと", NOW).eventTime).toBe("2026-10-07T00:00:00+09:00");
    const stay: FeelingTarget = {
      kind: "stay",
      stay: "22222222-2222-4222-8222-222222222222",
      start: "2026-10-07T01:00:00Z",
      end: "2026-10-07T03:00:00Z",
    };
    const b = buildFeeling(stay, SCALES, "ひとこと", NOW);
    expect(b.eventTime).toBe("2026-10-07T01:00:00Z");
    expect((JSON.parse(b.raw) as { note: string }).note).toBe("ひとこと");
  });
});

describe("ひとことの数え方", () => {
  it("絵文字 2,000 個は 2,000、2,001 個は上限超え", () => {
    expect(countNoteChars("😀".repeat(2000))).toBe(2000);
    expect(countNoteChars("😀".repeat(2001))).toBeGreaterThan(NOTE_MAX_CHARS);
  });
  it("NFD の「が」（2 コードポイント）は NFC にして 1", () => {
    expect(countNoteChars("\u304B\u3099")).toBe(1);
  });  it("サーバと同じく、改行とタブを除く制御文字（C0・DEL・C1）だけを制御文字と読む", () => {
    for (const ch of ["\u0000", "\u0007", "\r", "\u007F", "\u0085", "\u009F"]) expect(hasControlChar(`a${ch}b`), JSON.stringify(ch)).toBe(true);
    for (const ch of ["\n", "\t", " ", "\u3000", "\u200B", "😀"]) expect(hasControlChar(`a${ch}b`), JSON.stringify(ch)).toBe(false);
  });
});

describe("前/後", () => {
  it("対象の終わりより前に書けば前、以後なら後", () => {
    expect(phaseOf(DAY, new Date("2026-10-07T14:59:59Z"))).toBe("before");
    expect(phaseOf(DAY, new Date("2026-10-07T15:00:00Z"))).toBe("after");
    const stay: FeelingTarget = { kind: "stay", stay: "s", start: "2026-10-07T01:00:00Z", end: "2026-10-07T03:00:00Z" };
    expect(phaseOf(stay, new Date("2026-10-07T02:00:00Z"))).toBe("before");
    expect(phaseOf(stay, new Date("2026-10-07T03:00:00Z"))).toBe("after");
  });
});

describe("暦と日の状態", () => {
  it("対象の日で終わる 30 日が古い順に並ぶ", () => {
    const days = calendarDays("2026-10-03");
    expect(days).toHaveLength(30);
    expect(days[0]).toBe("2026-09-04");
    expect(days[29]).toBe("2026-10-03");
  });
  it("日の状態の表示の文字", () => {
    expect(dayMark("recorded", 2)).toBe("+2");
    expect(dayMark("recorded", -1)).toBe("-1");
    expect(dayMark("recorded", 0)).toBe("0");
    expect(dayMark("recorded", null)).toBe("滞");
    expect(dayMark("missing", null)).toBe("未");
    expect(dayMark("before_start", null)).toBe("");
    expect(dayMark("future", null)).toBe("");
  });
});
