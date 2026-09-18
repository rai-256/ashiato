// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useState } from "react";
import { AchievementPanel } from "./AchievementPanel";
import { CoverageGrid } from "./CoverageGrid";
import { type Achievement, type SourceCoverage } from "./coverage";
import { orderCoverageWithArchives } from "./archives";
import { DAY_TZ, MIN_TARGET_PX, SURFACE, TEXT, tone, YEAR_WEEKS } from "./tokens";

/**
 * `Asia/Tokyo` の「今日」（`YYYY-MM-DD`）。**`toISOString()` は UTC の日**なので使わない。
 *
 * `en-CA` は `YYYY-MM-DD` を返すロケール。`timeZone` を渡すのが、環境の設定に依らず
 * 特定のタイムゾーンの日を引く標準の手。
 */
export function todayInTz(now: Date, timeZone: string = DAY_TZ): string {
  return new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(now);
}

/**
 * 1 年ぶん（53 週）を引く。**窓は画面が決める。合否の窓は API が決める**（第 6 回 Q23）。
 *
 * **日は `Asia/Tokyo` で切る**（review/code.md の R10）。UTC で切っていたときは
 * JST の 00:00〜09:00 のあいだ `to` が前日になり、**毎日 9 時間だけ今日が格子から消えた**。
 */
export function yearRange(now: Date): { from: string; to: string } {
  const to = todayInTz(now);
  // 日付の引き算は UTC 正午を起点にする（夏時間もうるう秒も跨がない安全な足場）
  const anchor = new Date(`${to}T12:00:00Z`);
  anchor.setUTCDate(anchor.getUTCDate() - (YEAR_WEEKS * 7 - 1));
  return { from: anchor.toISOString().slice(0, 10), to };
}

/** 読み出しの状態。**「読み込み中」と「データが無い」を分ける**（review/code.md の R19）。 */
type Load<T> = { at: "loading" } | { at: "ok"; value: T } | { at: "failed"; why: string };

/**
 * S-1 稼働状況（ST02）。**ソースごとに格子を分けて縦に積む**（深掘り 第 4 回 Q15）。
 *
 * 開いた直後は各ソースの直近 4〜5 週だけが出る（第 7 回 Q28）。
 */
export function App(): React.ReactElement {
  const [sources, setSources] = useState<Load<SourceCoverage[]>>({ at: "loading" });
  const [achievement, setAchievement] = useState<Load<Achievement>>({ at: "loading" });

  useEffect(() => {
    const { from, to } = yearRange(new Date());
    const get = async (path: string): Promise<unknown> => {
      const res = await fetch(path);
      if (!res.ok) throw new Error(`status_${res.status}`);
      return res.json();
    };
    const why = (e: unknown): string => (e instanceof Error ? e.message : "unknown");
    // **別々に受ける**（review/code.md の R19 / H-2）。`Promise.all` にしていたときは、
    // 達成の取得が落ちると**正常に取れた格子 5 本まで消えた**。
    // この画面の目的は「データが無い」の意味を残すことなので、
    // **取得に失敗したことと、データが無いことを混ぜてはいけない。**
    get(`/api/coverage?from=${from}&to=${to}`)
      .then((v) => setSources({ at: "ok", value: v as SourceCoverage[] }))
      .catch((e: unknown) => setSources({ at: "failed", why: why(e) }));
    get("/api/coverage/achievement")
      .then((v) => setAchievement({ at: "ok", value: v as Achievement }))
      .catch((e: unknown) => setAchievement({ at: "failed", why: why(e) }));
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
      {/* **1 日の一覧への行き先は見出しと同じ行に置く**（ST16 / design D8）。行を増やすと、
          ひとスクロールの勘定（one-scroll.test.tsx）に効く */}
      <header style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", gap: 8 }}>
        <h1 style={{ font: "600 18px/1.3 system-ui, sans-serif", margin: "0 0 16px" }}>
          収集が動いていたか
        </h1>
        <a
          href="#/day/"
          data-testid="to-day"
          style={{
            color: tone(TEXT.normal),
            font: "400 14px/1.6 system-ui, sans-serif",
            minHeight: MIN_TARGET_PX,
            minWidth: MIN_TARGET_PX,
            display: "inline-flex",
            alignItems: "center",
          }}
        >
          1 日の滞在へ
        </a>
        {/* **マスタ管理への入口**（ST19 / design D9。仮 —— 入口の置き場を決め直すのは ST25） */}
        <a
          href="#/master"
          data-testid="to-master"
          style={{
            color: tone(TEXT.normal),
            font: "400 14px/1.6 system-ui, sans-serif",
            minHeight: MIN_TARGET_PX,
            minWidth: MIN_TARGET_PX,
            display: "inline-flex",
            alignItems: "center",
          }}
        >
          マスタ管理へ
        </a>
      </header>
      {achievement.at === "loading" && <p data-testid="achievement-loading">読み込み中…</p>}
      {achievement.at === "failed" && (
        <p role="alert" data-testid="achievement-error">
          達成の読み出しに失敗しました（{achievement.why}）。達成日数が 0 なのではありません。
        </p>
      )}
      {achievement.at === "ok" && <AchievementPanel data={achievement.value} />}

      {sources.at === "loading" && <p data-testid="coverage-loading">読み込み中…</p>}
      {sources.at === "failed" && (
        <p role="alert" data-testid="coverage-error">
          稼働状況の読み出しに失敗しました（{sources.why}）。収集が止まったのではありません。
        </p>
      )}
      {sources.at === "ok" && sources.value.length === 0 && (
        <p data-testid="coverage-empty">ソースが 1 本も返りませんでした（登録簿を確認してください）。</p>
      )}
      {/* **退役したソースは後ろ**（ST03 の R63）—— Must の 5 本を 1 画面から押し出さない */}
      {sources.at === "ok" &&
        orderCoverageWithArchives(sources.value).map((s) => <CoverageGrid key={s.logical_source} source={s} />)}
    </main>
  );
}
