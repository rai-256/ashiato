// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 表面のトークン。**`docs/ui-direction.md` の確定値から引く 1 か所**（design D12）。
 *
 * 確定値: 色相 132° / 彩度 30% / 地の明るさ 12 / 面 3 段。
 * **S-1 のためだけの色を足さない** —— 足すと ST25（S-2 主表現）が別のトークン系を持つ。
 */

/** 色相。`ui-direction.md` の確定値（緑 = 苔の方向）。 */
export const HUE = 132;
/** 彩度。同上。**低く抑える**側で決まっている。 */
export const SAT = 30;

/**
 * 面の 3 段。地の明るさ 12 から、`ui-direction.md` の「面 3 段」で積む。
 * 格子は `surface2` の上に置く —— **最も暗い段のセルがそこで読める**（下の `BAND` を参照）。
 */
export const SURFACE = { ground: 12, surface1: 18, surface2: 24 } as const;

/**
 * 格子のセルが担う **3 段**（design D10 / 深掘り 第 5 回 Q21）。
 *
 * **7 段にしない。** WCAG SC 1.4.11（NFR-23）は隣接する色に 3:1 を求めるが、
 * 7 段を隣接 3:1 で並べると 3^6 = 729:1 が要り、**sRGB の理論最大は 21:1**。
 * 21:1 を 6 等分しても隣接 1.661:1 にしかならず、**計算で不成立が確定している**。
 * 8 状態の区別は**週を選んだときの文字**が担う。
 *
 * この 3 つの明るさは隣接比が 3.89:1 と 3.81:1 で、NFR-23 を満たす
 * （`state-contrast.test.ts` が値から計算して確かめる。ここに書いた数を信じない）。
 */
export const BAND = {
  /** ③④⑤⑥⑦ を畳んだ段。地に近い側 */
  other: 9,
  /** ② 動いていた・記録なし */
  alive_no_record: 40,
  /** ① 記録あり */
  recorded: 92,
} as const;

/** 文字の明るさ。NFR-18（4.5:1）は `text-contrast.test.ts` が確かめる。 */
export const TEXT = { normal: 92, muted: 72 } as const;

/**
 * 明暗の 2 つの面と文字（NFR-17 / ST16 の 1 日の一覧）。**OS の明暗に追従し、取得できないときはダーク。**
 *
 * ダークは S-1 と同じ値。ライトは `docs/ui-direction-playground.html` がライトの面を置く帯
 * （地が明るい側。明度 66 以上）に、同じ 3 段の間隔で置いた。
 * 4.5:1（NFR-18）とフォーカスの輪郭の 3:1（NFR-22 / NFR-23）は `day-view-limits.test.tsx` が値から計算して確かめる。
 */
export const SCHEMES = {
  dark: { ground: SURFACE.ground, surface1: SURFACE.surface1, surface2: SURFACE.surface2, text: TEXT.normal, muted: TEXT.muted },
  light: { ground: 96, surface1: 91, surface2: 86, text: 14, muted: 30 },
} as const;

export type Scheme = keyof typeof SCHEMES;

/** HSL の文字列。**色相と彩度をここでしか触らせない。** */
export function tone(lightness: number): string {
  return `hsl(${HUE} ${SAT}% ${lightness}%)`;
}

/**
 * 操作できるものの最小の大きさ（NFR-19）。
 *
 * **週の帯がこれを満たす。セルは操作対象にしない**（深掘り 第 4 回 Q16）——
 * 1 週 7 日を横に並べると 1 セルが 24 px に満たない幅になりうる。
 * 選ぶ単位を週にすれば、セルの大きさに関わらず NFR-19 を満たせる。
 */
export const MIN_TARGET_PX = 24;

/**
 * 日の区切り。**サーバの `DAY_TZ`（`crates/server/src/coverage.rs`）と同じでなければならない**
 * （深掘り Q2 / design D1）。
 *
 * 画面が UTC で日を切っていたときは、**JST の 00:00〜09:00 のあいだ今日が格子に出ず**、
 * 格子の窓と達成の窓が 1 日ずれた（review/code.md の R10）。
 * 「日を引く場所がアプリと SQL に割れると、片方だけずれても誰も気付かない」——
 * サーバ側の `facts()` が書いているのと同じ割れ方が、画面側に残っていた。
 */
export const DAY_TZ = "Asia/Tokyo";

/**
 * 開いた直後に見せる週数（深掘り 第 7 回 Q28 は「直近 4〜5 週」）。
 *
 * **5 ではなく 4**（第 8 回 Q30）。Q28 の根拠にした 600 px は
 * `5 ソース × 5 行 × 24 px` で**セルだけを積んだ勘定**で、見出し・余白・ボタン・
 * 達成の表を数えていなかった。実際に宣言されている箱を積むと約 1,491 px あり、
 * **ひとスクロール（2 画面 = 1,280 px）に収まらない**。
 * 5 → 4 で 5 ソースぶん 130 px を返す。`one-scroll.test.tsx` が勘定を固定する。
 */
export const INITIAL_WEEKS = 4;

/**
 * 基準の画面の高さ（CSS px）。**NFR-19 が幅に使っている 360 px の端末**に、
 * その端末の高さを合わせたもの（360 × 640）。
 */
export const VIEWPORT_H_PX = 640;

/** ひとスクロール = **2 画面ぶん**（第 8 回 Q30）。開いて 1 画面、1 回スクロールで残り。 */
export const ONE_SCROLL_PX = VIEWPORT_H_PX * 2;

/** ソースの節どうしの間隔。**勘定に効くので `one-scroll.test.tsx` が見ている。** */
export const SECTION_GAP_PX = 12;
/** ソースの節の内側の余白。同上。 */
export const SECTION_PAD_PX = 8;

/** 伸ばしたときの週数（1 年）。 */
export const YEAR_WEEKS = 53;
