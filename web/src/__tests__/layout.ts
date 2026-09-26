// SPDX-License-Identifier: AGPL-3.0-only
/**
 * **宣言されている箱**を測る道具（design D27 / tasks 10.3）。
 *
 * jsdom は実寸を測らないので、ここで積むのは `element.style` に**実際に入っている値**
 * であって実寸ではない。定数を読み直して定数と突き合わせるのではない ——
 * `SECTION_PAD_PX` を 40 にすれば、これを使う検査が落ちる。
 *
 * **ひとスクロールの勘定と箱の高さの両方が使う**ので、1 か所に置く。
 */

/**
 * CSS の長さを px で読む。**読めなかったら落とす**（review/code-r2.md の M-2）。
 *
 * 黙って 0 にしていたときは、**勘定が要素を数えられていないだけ**でも
 * 「予算に収まっている」という緑が出た。指定が無い（空文字）ときだけ 0 を返す。
 */
export const px = (v: string): number => {
  if (v === "") return 0;
  const n = Number.parseFloat(v);
  if (!Number.isFinite(n) || !v.trim().endsWith("px")) {
    throw new Error(`px で読めない長さ: ${JSON.stringify(v)}`);
  }
  return n;
};

/**
 * 枠線の太さ。**`border: none` は `borderTopWidth` に `"medium"` を返す**
 * （CSS の初期値のキーワード。長さではない）。太さのキーワードは
 * `thin` / `medium` / `thick` = 1 / 3 / 5 px にあたるが、**線種が無ければ 0**。
 *
 * 前は `px()` が読めない値を黙って 0 にしていたので、この区別ごと消えていた
 * （review/code-r2.md の M-2）。
 */
export function borderWidth(el: HTMLElement, side: "Top" | "Bottom"): number {
  const style = el.style.getPropertyValue(`border-${side.toLowerCase()}-style`) || el.style.borderStyle;
  if (style === "none" || style === "hidden") return 0;
  const w = side === "Top" ? el.style.borderTopWidth : el.style.borderBottomWidth;
  const keyword: Record<string, number> = { thin: 1, medium: 3, thick: 5 };
  if (w in keyword) return style === "" ? 0 : keyword[w];
  return px(w);
}

/**
 * その要素に効いている行の高さ。`font` の短縮記法（`600 15px/1.3 ...`）から引き、
 * 宣言が無ければ先祖をたどる（CSS の継承と同じ向き）。
 */
export function lineHeight(el: HTMLElement): number {
  for (let e: HTMLElement | null = el; e !== null; e = e.parentElement) {
    const m = /(\d+(?:\.\d+)?)px\s*\/\s*(\d+(?:\.\d+)?)/.exec(e.style.font);
    if (m !== null) return Number(m[1]) * Number(m[2]);
  }
  // **黙って 0 にしない**（同 M-2）。先祖まで `font` が無い要素は勘定に穴を開ける
  throw new Error(`行の高さが引けない: <${el.tagName.toLowerCase()}>`);
}

/** 直接の子（要素ではないもの）に文字があるか。あれば少なくとも 1 行ぶんの高さを取る。 */
export function hasOwnText(el: HTMLElement): boolean {
  return [...el.childNodes].some(
    (n) => n.nodeType === Node.TEXT_NODE && (n.textContent ?? "").trim() !== "",
  );
}

/**
 * **宣言されている箱の高さ**を積む。
 *
 * 横に並ぶもの（`display: flex` で縦並びでないもの。週の帯の中のセル）は
 * 足さずに**いちばん高いもの**を取る。それ以外は上から下へ積む。
 */
export function declaredHeight(el: HTMLElement): number {
  const s = el.style;
  const chrome =
    px(s.paddingTop) +
    px(s.paddingBottom) +
    borderWidth(el, "Top") +
    borderWidth(el, "Bottom") +
    px(s.marginTop) +
    px(s.marginBottom);
  const kids = [...el.children].filter((c): c is HTMLElement => c instanceof HTMLElement);
  // **横に並ぶもの**: `display: flex` で縦並びでないもの（週の帯の中のセル）と、
  // 表の行（`<tr>` の中のセルは横に並ぶ。足すと 1 行が 4 行ぶんの高さになる）
  const row = (s.display === "flex" && s.flexDirection !== "column") || el.tagName === "TR";
  const stacked =
    kids.length === 0
      ? 0
      : row
        ? Math.max(...kids.map(declaredHeight))
        : kids.reduce((a, k) => a + declaredHeight(k), 0);
  const text = kids.length === 0 || hasOwnText(el) ? lineHeight(el) : 0;
  return chrome + Math.max(px(s.minHeight), stacked, text);
}

/** `root` の内容の上端から数えた、`target` の下端までの距離。 */
export function bottomWithin(root: HTMLElement, target: HTMLElement): number {
  let y = 0;
  for (const child of [...root.children]) {
    if (!(child instanceof HTMLElement)) continue;
    if (child === target) return y + declaredHeight(child);
    if (child.contains(target)) {
      // その子の枠（上の余白・枠線・内側の余白）を足してから中へ降りる
      return (
        y +
        px(child.style.marginTop) +
        borderWidth(child, "Top") +
        px(child.style.paddingTop) +
        bottomWithin(child, target)
      );
    }
    y += declaredHeight(child);
  }
  throw new Error(
    `その要素が <${root.tagName.toLowerCase()}> の下に見つからない: ` +
      `<${target.tagName.toLowerCase()} data-source="${target.getAttribute("data-source") ?? ""}">`,
  );
}

/** 画面の上端から、その要素の下端までの高さ。 */
export function bottomOf(main: HTMLElement, target: HTMLElement): number {
  return px(main.style.paddingTop) + bottomWithin(main, target);
}
