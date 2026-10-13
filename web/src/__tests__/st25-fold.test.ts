// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 同じアプリの連続の畳み（ST25 / tasks 9.2 / design D8）。畳みの規則は画面の純粋な関数。
 */
import { describe, expect, it } from "vitest";
import { foldRuns, type DayRecord } from "../timeline";

const at = (hm: string): string => `2026-09-07T${hm}:00+09:00`;
let n = 0;
const rec = (source: string, app: string, hm: string, endHm?: string, extra: Partial<DayRecord> = {}): DayRecord => ({
  id: `r${++n}`,
  kind: source === "browser" ? "visit" : "foreground",
  source,
  start: at(hm),
  end: endHm === undefined ? undefined : at(endHm),
  tz_id: "Asia/Tokyo",
  app,
  title: `${app} ${hm}`,
  ...extra,
});

describe("畳み", () => {
  // Scenario: 同じアプリの連続は 1 行に畳まれる
  it("VS Code の 23 件の連続は始まり・終わり・件数を持つ 1 行で、次の行が Chrome", () => {
    const code = Array.from({ length: 23 }, (_, i) => rec("pc", "VS Code", `09:${String(2 + Math.floor(i / 2)).padStart(2, "0")}`, `09:${String(2 + Math.floor((i + 1) / 2)).padStart(2, "0")}`));
    code[22] = { ...code[22], end: at("09:14") };
    const folded = foldRuns([...code, rec("pc", "Chrome", "09:14")]);
    expect(folded).toHaveLength(2);
    const run = folded[0];
    expect(run.at).toBe("run");
    if (run.at !== "run") return;
    expect([run.start, run.end, run.source, run.app, run.count]).toEqual([at("09:02"), at("09:14"), "pc", "VS Code", 23]);
    expect(folded[1].at === "one" && folded[1].record.app).toBe("Chrome");
  });

  // Scenario: 別のアプリが挟まると別の行になる
  it("VS Code・Chrome・VS Code は 3 行（VS Code の行が Chrome を挟んで 2 つ）", () => {
    const folded = foldRuns([
      rec("pc", "VS Code", "09:00"),
      rec("pc", "VS Code", "09:01"),
      rec("pc", "Chrome", "09:02"),
      rec("pc", "Chrome", "09:03"),
      rec("pc", "VS Code", "09:04"),
      rec("pc", "VS Code", "09:05"),
    ]);
    expect(folded.map((f) => (f.at === "run" ? `${f.app}x${f.count}` : "one"))).toEqual(["VS Codex2", "Chromex2", "VS Codex2"]);
  });

  it("同じアプリでも source が違えば畳まない・ブラウザは同じ browser だけ畳む", () => {
    const folded = foldRuns([rec("pc", "Chrome", "09:00"), rec("phone", "Chrome", "09:01"), rec("browser", "Chrome", "09:02"), rec("browser", "Chrome", "09:03"), rec("browser", "Edge", "09:04")]);
    expect(folded.map((f) => f.at)).toEqual(["one", "one", "run", "one"]);
  });

  it("取れていない時間は畳まず、連続を分ける", () => {
    const gap = (hm: string): DayRecord => ({ ...rec("pc", "x", hm), kind: "away", app: undefined });
    const folded = foldRuns([rec("pc", "VS Code", "09:00"), rec("pc", "VS Code", "09:01"), gap("09:02"), gap("09:03"), rec("pc", "VS Code", "09:04"), rec("pc", "VS Code", "09:05")]);
    expect(folded.map((f) => f.at)).toEqual(["run", "one", "one", "run"]);
  });
});
