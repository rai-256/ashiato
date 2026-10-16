// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useState } from "react";
import { App, todayInTz } from "./App";
import { DayView } from "./DayView";
import { FeelInvalid, FeelView } from "./FeelView";
import { feelDateFromHash } from "./feelings";
import { MasterView } from "./MasterView";
import { dayFromHash, isRealDate } from "./stays";
import { msg } from "./messages";
import { useScheme } from "./scheme";
import { linkStyle, pageStyle } from "./surface";
import { SCHEMES } from "./tokens";

const m = msg("root");

/**
 * 画面の行き先を決める（ST16 / design D8、ST25 / design D12）。**ルートは今日の 1 日の画面。**
 * 稼働状況は `#/coverage`、1 日の画面は `#/day/YYYY-MM-DD`（日付を省けば今日）、マスタ管理は `#/master`
 * （個人属性。ST19 / design D9）と `#/master/places`（場所。ST21 / design D12）。解釈できないアドレスも今日の 1 日の画面。
 */
export function Root(): React.ReactElement {
  const [hash, setHash] = useState(() => window.location.hash);
  const scheme = useScheme();
  const c = SCHEMES[scheme];
  useEffect(() => {
    const on = (): void => setHash(window.location.hash);
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  // **S-6 マスタ管理**（ST19 / design D9、ST21 / design D12）
  if (/^#\/master\/?$/.test(hash)) return <MasterView tab="attributes" />;
  if (/^#\/master\/places\/?$/.test(hash)) return <MasterView tab="places" />;
  // **S-3 主観入力**（ST17 / design D12）。日付を省けば暦の今日、暦に無い日付は書く操作を出さない
  const feel = feelDateFromHash(hash);
  if (feel !== undefined) {
    if ("invalid" in feel) return <FeelInvalid raw={feel.invalid} />;
    return <FeelView key={feel.date ?? "today"} date={feel.date} />;
  }
  // **S-1 稼働状況**（ST25 / design D12）。ルートではなく専用のアドレス
  if (/^#\/coverage\/?$/.test(hash)) return <App />;
  // 空・`#/`・解釈できないアドレスは今日の 1 日の画面
  const day = dayFromHash(hash) ?? null;
  // **暦に無い日付は、そう出す**（R47）。そのまま一覧を開くと見出しが「13 月 45 日」になり、前後の日へ移るボタンが黙って効かない
  if (day !== null && !isRealDate(day)) {
    return (
      <main
        data-testid="day-invalid"
        data-scheme={scheme}
        style={pageStyle(c)}
      >
        <p role="alert">{m.invalidDate(day)}</p>
        <a href="#/day/" style={linkStyle(c)}>
          {m.toToday}
        </a>
      </main>
    );
  }
  return <DayView date={day ?? todayInTz(new Date())} />;
}
