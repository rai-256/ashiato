// SPDX-License-Identifier: AGPL-3.0-only
import { vi } from "vitest";

/**
 * 輪郭の規則は CSSOM の構成可能スタイルシートで当てる（CSP が `<style>` 要素を止めるため。ST28 / D10）。
 * jsdom には `adoptedStyleSheets` が無いので、当てられた規則の文字列を集める。戻り値の `text()` が全部の連結。
 */
export function captureAdoptedRules(): { text: () => string } {
  const texts: string[] = [];
  Object.defineProperty(document, "adoptedStyleSheets", { value: [], writable: true, configurable: true });
  vi.spyOn(CSSStyleSheet.prototype, "replaceSync").mockImplementation((t: string) => {
    texts.push(t);
  });
  return { text: () => texts.join("\n") };
}
