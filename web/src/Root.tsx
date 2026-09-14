// SPDX-License-Identifier: AGPL-3.0-only
import { useEffect, useState } from "react";
import { App, todayInTz } from "./App";
import { DayView } from "./DayView";
import { dayFromHash } from "./stays";

/**
 * 画面の行き先を決める（ST16 / design D8）。**S-1（稼働状況）はルートのまま** ——
 * 入口を決め直すのは ST25。1 日の一覧は `#/day/YYYY-MM-DD`（日付を省けば今日）。
 */
export function Root(): React.ReactElement {
  const [hash, setHash] = useState(() => window.location.hash);
  useEffect(() => {
    const on = (): void => setHash(window.location.hash);
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  const day = dayFromHash(hash);
  if (day === undefined) return <App />;
  return <DayView date={day ?? todayInTz(new Date())} />;
}
