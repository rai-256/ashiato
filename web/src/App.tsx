// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useState } from "react";
import { AchievementPanel } from "./AchievementPanel";
import { CoverageGrid } from "./CoverageGrid";
import type { Achievement, SourceCoverage } from "./coverage";
import { SURFACE, TEXT, tone, YEAR_WEEKS } from "./tokens";

/** 1 年ぶん（53 週）を引く。**窓は画面が決める。合否の窓は API が決める**（第 6 回 Q23）。 */
function yearRange(today: Date): { from: string; to: string } {
  const to = new Date(today);
  const from = new Date(today);
  from.setUTCDate(from.getUTCDate() - (YEAR_WEEKS * 7 - 1));
  return { from: from.toISOString().slice(0, 10), to: to.toISOString().slice(0, 10) };
}

/**
 * S-1 稼働状況（ST02）。**ソースごとに格子を分けて縦に積む**（深掘り 第 4 回 Q15）。
 *
 * 開いた直後は各ソースの直近 4〜5 週だけが出る（第 7 回 Q28）——
 * 5 ソース × 5 行 × 24 px ≒ 600 px なので、**直近 1 か月が 1 画面に収まる**。
 */
export function App(): React.ReactElement {
  const [sources, setSources] = useState<SourceCoverage[] | null>(null);
  const [achievement, setAchievement] = useState<Achievement | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const { from, to } = yearRange(new Date());
    const get = async (path: string): Promise<unknown> => {
      const res = await fetch(path);
      if (!res.ok) throw new Error(String(res.status));
      return res.json();
    };
    Promise.all([get(`/api/coverage?from=${from}&to=${to}`), get("/api/coverage/achievement")])
      .then(([cov, ach]) => {
        setSources(cov as SourceCoverage[]);
        setAchievement(ach as Achievement);
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : "unknown"));
  }, []);

  return (
    <main
      style={{
        background: tone(SURFACE.ground),
        color: tone(TEXT.normal),
        font: "400 14px/1.6 system-ui, sans-serif",
        minHeight: "100vh",
        // **横スクロールを出さない**（NFR-19 / 完了の判定）。縦長の格子は幅いっぱいに収まる
        maxWidth: "100%",
        overflowX: "hidden",
        padding: 12,
      }}
    >
      <h1 style={{ font: "600 18px/1.3 system-ui, sans-serif", margin: "0 0 16px" }}>
        収集が動いていたか
      </h1>
      {error !== null && <p role="alert">読み出せませんでした（{error}）</p>}
      {achievement !== null && <AchievementPanel data={achievement} />}
      {sources?.map((s) => (
        <CoverageGrid key={s.logical_source} source={s} />
      ))}
    </main>
  );
}
