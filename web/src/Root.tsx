// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useState } from "react";
import { App, todayInTz } from "./App";
import { DayView } from "./DayView";
import { MasterView } from "./MasterView";
import { dayFromHash, isRealDate } from "./stays";
import { SCHEMES, tone } from "./tokens";

/**
 * 画面の行き先を決める（ST16 / design D8）。**S-1（稼働状況）はルートのまま** ——
 * 入口を決め直すのは ST25。1 日の一覧は `#/day/YYYY-MM-DD`（日付を省けば今日）、
 * マスタ管理は `#/master`（ST19 / design D9）。
 */
export function Root(): React.ReactElement {
  const [hash, setHash] = useState(() => window.location.hash);
  useEffect(() => {
    const on = (): void => setHash(window.location.hash);
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  // **S-6 マスタ管理**（ST19 / design D9）。`#/day/…` とルート（S-1）は変えない
  if (/^#\/master\/?$/.test(hash)) return <MasterView />;
  const day = dayFromHash(hash);
  if (day === undefined) return <App />;
  // **暦に無い日付は、そう出す**（R47）。そのまま一覧を開くと見出しが「13 月 45 日」になり、前後の日へ移るボタンが黙って効かない
  if (day !== null && !isRealDate(day)) {
    return (
      <main
        data-testid="day-invalid"
        style={{ background: tone(SCHEMES.dark.ground), color: tone(SCHEMES.dark.text), minHeight: "100vh", padding: 12 }}
      >
        <p role="alert">アドレスの日付（{day}）を日付として読めません。</p>
        <a href="#/day/" style={{ color: tone(SCHEMES.dark.text), minHeight: 24, minWidth: 24, display: "inline-flex", alignItems: "center" }}>
          今日の一覧へ
        </a>
      </main>
    );
  }
  return <DayView date={day ?? todayInTz(new Date())} />;
}
