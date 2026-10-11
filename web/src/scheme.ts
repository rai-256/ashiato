// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 明暗とフォーカスの輪郭（NFR-17 / NFR-22）。**全部の画面が同じものを使う。**
 *
 * もとは `DayView.tsx` にあった（ST16）。稼働状況（S-1）にもライトを持たせるため（2026-10-12）、
 * どの画面からも循環なしに引けるようにここへ移した。`DayView.tsx` は同じ名前で再び公開している。
 */
import { useEffect, useState } from "react";
import { FOCUS_RING_PX, SCHEMES, tone, type Scheme } from "./tokens";

/** OS の明暗の設定。**取得できないときはダーク**（NFR-17）。 */
export function useScheme(): Scheme {
  const query = (): Scheme =>
    typeof window.matchMedia === "function" && window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  const [scheme, setScheme] = useState<Scheme>(query);
  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const on = (): void => setScheme(query());
    mq.addEventListener?.("change", on);
    return () => mq.removeEventListener?.("change", on);
  }, []);
  return scheme;
}

/** 操作できるもの（日の移動・日付の指定・画面の行き来）に付ける印。フォーカスの輪郭はこの印に掛ける。 */
export const FOCUS_ATTR = "data-focus-ring";

/** フォーカスの輪郭（NFR-22）。**本文の色・3 px**。`:focus-visible` は style 属性に書けないので、印に掛ける規則を 1 つ置く。 */
export function focusRule(scheme: Scheme): string {
  return `[${FOCUS_ATTR}]:focus-visible { outline: ${FOCUS_RING_PX}px solid ${tone(SCHEMES[scheme].text)}; outline-offset: 2px; }`;
}

/**
 * 輪郭の規則を文書に当てる。**`<style>` 要素は使わない** —— 画面の CSP（`default-src 'self'`。ST28 / D10）が
 * 要素として埋め込んだ style を止める。CSSOM で足した規則（構成可能スタイルシート）は止められない。
 */
export function useFocusRule(scheme: Scheme): void {
  useEffect(() => {
    // 構成可能スタイルシートを持たない環境（jsdom）では当てない。ブラウザは全部持つ（Chromium / Firefox 101+ / Safari 16.4+）
    if (!("adoptedStyleSheets" in document)) return;
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(focusRule(scheme));
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
    return () => {
      document.adoptedStyleSheets = document.adoptedStyleSheets.filter((s) => s !== sheet);
    };
  }, [scheme]);
}
